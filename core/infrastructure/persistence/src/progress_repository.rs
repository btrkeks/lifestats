use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use diesel::connection::Connection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_types::{BigInt, Text};
use diesel::SqliteConnection;
use lifestats_core::progress::ports::progress_repository::{
    ProgressCheckpointRange, ProgressRepository, SaveCheckpointResult,
};
use lifestats_core::progress::{
    NewProgressCheckpoint, NewProgressItem, ProgressCheckpoint, ProgressItem, ProgressMetadata,
    ProgressTarget, ProgressUnit,
};
use lifestats_core::shared::ids::{ProgressCheckpointId, ProgressItemId};
use lifestats_core::shared::ports::error::RepositoryError;

use crate::db::mappers::canonicalize_tags;
use crate::db::models::{
    NewProgressCheckpointRow, NewProgressItemRow, ProgressCheckpointRow, ProgressItemRow,
};
use crate::db::schema::{progress_checkpoints, progress_items};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

fn diesel_to_repository_error(err: diesel::result::Error) -> RepositoryError {
    match err {
        diesel::result::Error::NotFound => RepositoryError::NotFound,
        _ => RepositoryError::OperationFailed(err.into()),
    }
}

pub struct SqliteProgressRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteProgressRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, RepositoryError> {
        self.pool
            .get()
            .context("failed to get SQLite connection from pool")
            .map_err(RepositoryError::OperationFailed)
    }

    fn load_item(conn: &mut SqliteConnection, item_id: ProgressItemId) -> Result<ProgressItem> {
        let row = progress_items::table
            .find(item_id.0 as i32)
            .first::<ProgressItemRow>(conn)?;
        row_to_item(row)
    }
}

impl ProgressRepository for SqliteProgressRepository {
    fn create_item(&self, item: NewProgressItem) -> Result<ProgressItem, RepositoryError> {
        let mut conn = self.connection()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let row = new_item_to_row(item)?;
            diesel::insert_into(progress_items::table)
                .values(&row)
                .execute(conn)?;

            let new_id: i64 =
                diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?;
            Self::load_item(conn, ProgressItemId(new_id as u32))
        })
        .map_err(RepositoryError::OperationFailed)
    }

    fn update_item(&self, item: ProgressItem) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        let (unit_kind, unit_label) = item.target().unit().clone().into_storage_parts();
        let tags_json = serde_json::to_string(item.metadata().tags())
            .context("failed to serialize progress item tags")
            .map_err(RepositoryError::OperationFailed)?;
        let canonical_tags = canonicalize_tags(item.metadata().tags());

        diesel::update(progress_items::table.find(item.id().0 as i32))
            .set((
                progress_items::name.eq(item.metadata().name()),
                progress_items::category.eq(item.metadata().category()),
                progress_items::unit_kind.eq(unit_kind),
                progress_items::unit_label.eq(unit_label),
                progress_items::target_value.eq(item.target().value()),
                progress_items::tags.eq(tags_json),
                progress_items::canonical_tags.eq(canonical_tags),
                progress_items::created_at.eq(format_datetime(item.created_at())),
                progress_items::updated_at.eq(format_datetime(item.updated_at())),
            ))
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn find_item_by_id(
        &self,
        item_id: ProgressItemId,
    ) -> Result<Option<ProgressItem>, RepositoryError> {
        let mut conn = self.connection()?;
        let row = progress_items::table
            .find(item_id.0 as i32)
            .first::<ProgressItemRow>(&mut conn)
            .optional()
            .map_err(diesel_to_repository_error)?;

        row.map(row_to_item)
            .transpose()
            .map_err(RepositoryError::OperationFailed)
    }

    fn find_all_items(&self) -> Result<Vec<ProgressItem>, RepositoryError> {
        let mut conn = self.connection()?;
        let rows = progress_items::table
            .order(progress_items::id.asc())
            .load::<ProgressItemRow>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        rows.into_iter()
            .map(row_to_item)
            .collect::<Result<Vec<_>>>()
            .map_err(RepositoryError::OperationFailed)
    }

    fn find_items_by_tags(
        &self,
        tags: &HashMap<String, String>,
    ) -> Result<Vec<ProgressItem>, RepositoryError> {
        let mut conn = self.connection()?;
        let filter_json = serde_json::to_string(tags)
            .context("failed to serialize tag filter")
            .map_err(RepositoryError::OperationFailed)?;

        let query = diesel::sql_query(
            "SELECT id, name, category, unit_kind, unit_label, target_value, tags, canonical_tags, created_at, updated_at \
             FROM progress_items \
             WHERE NOT EXISTS ( \
                 SELECT 1 \
                 FROM json_each(?) AS filter \
                 WHERE json_extract(progress_items.tags, '$.' || filter.key) IS NULL \
                   OR json_extract(progress_items.tags, '$.' || filter.key) <> filter.value \
             ) \
             ORDER BY id ASC",
        )
        .bind::<Text, _>(filter_json);

        let rows: Vec<ProgressItemRow> =
            query.load(&mut conn).map_err(diesel_to_repository_error)?;
        rows.into_iter()
            .map(row_to_item)
            .collect::<Result<Vec<_>>>()
            .map_err(RepositoryError::OperationFailed)
    }

    fn save_checkpoint(
        &self,
        checkpoint: NewProgressCheckpoint,
    ) -> Result<SaveCheckpointResult, RepositoryError> {
        let mut conn = self.connection()?;
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let row = checkpoint_to_row(checkpoint)?;
            let has_idempotency_key = row.source.is_some() && row.source_event_id.is_some();
            let source = row.source.clone();
            let source_event_id = row.source_event_id.clone();
            let item_id = row.progress_item_id;

            let inserted = diesel::insert_or_ignore_into(progress_checkpoints::table)
                .values(&row)
                .execute(conn)?;

            if inserted > 0 {
                let new_id: i64 =
                    diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?;
                let checkpoint = load_checkpoint_by_id(conn, ProgressCheckpointId(new_id as u32))?;
                return Ok(SaveCheckpointResult::Inserted(checkpoint));
            }

            if has_idempotency_key {
                let existing = progress_checkpoints::table
                    .filter(progress_checkpoints::progress_item_id.eq(item_id))
                    .filter(progress_checkpoints::source.eq(source))
                    .filter(progress_checkpoints::source_event_id.eq(source_event_id))
                    .first::<ProgressCheckpointRow>(conn)?;
                return Ok(SaveCheckpointResult::Duplicate(row_to_checkpoint(
                    existing,
                )?));
            }

            anyhow::bail!("manual progress checkpoint insert was ignored unexpectedly")
        })
        .map_err(RepositoryError::OperationFailed)
    }

    fn find_latest_checkpoint(
        &self,
        item_id: ProgressItemId,
    ) -> Result<Option<ProgressCheckpoint>, RepositoryError> {
        let mut conn = self.connection()?;
        let row = progress_checkpoints::table
            .filter(progress_checkpoints::progress_item_id.eq(item_id.0 as i32))
            .order((
                progress_checkpoints::recorded_at.desc(),
                progress_checkpoints::id.desc(),
            ))
            .first::<ProgressCheckpointRow>(&mut conn)
            .optional()
            .map_err(diesel_to_repository_error)?;

        row.map(row_to_checkpoint)
            .transpose()
            .map_err(RepositoryError::OperationFailed)
    }

    fn find_checkpoints(
        &self,
        item_id: ProgressItemId,
        range: ProgressCheckpointRange,
    ) -> Result<Vec<ProgressCheckpoint>, RepositoryError> {
        let mut conn = self.connection()?;
        let mut query = progress_checkpoints::table
            .filter(progress_checkpoints::progress_item_id.eq(item_id.0 as i32))
            .into_boxed();

        if let Some(start) = range.recorded_at_start {
            query = query.filter(progress_checkpoints::recorded_at.ge(format_datetime(start)));
        }
        if let Some(end) = range.recorded_at_end {
            query = query.filter(progress_checkpoints::recorded_at.le(format_datetime(end)));
        }

        let rows = query
            .order((
                progress_checkpoints::recorded_at.asc(),
                progress_checkpoints::id.asc(),
            ))
            .load::<ProgressCheckpointRow>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        rows.into_iter()
            .map(row_to_checkpoint)
            .collect::<Result<Vec<_>>>()
            .map_err(RepositoryError::OperationFailed)
    }
}

fn new_item_to_row(item: NewProgressItem) -> Result<NewProgressItemRow> {
    let (unit_kind, unit_label) = item.target().unit().clone().into_storage_parts();
    let tags = item.metadata().tags().clone();
    Ok(NewProgressItemRow {
        name: item.metadata().name().to_string(),
        category: item.metadata().category().to_string(),
        unit_kind,
        unit_label,
        target_value: item.target().value(),
        tags: serde_json::to_string(&tags).context("failed to serialize progress item tags")?,
        canonical_tags: canonicalize_tags(&tags),
        created_at: format_datetime(item.created_at()),
        updated_at: format_datetime(item.created_at()),
    })
}

fn checkpoint_to_row(checkpoint: NewProgressCheckpoint) -> Result<NewProgressCheckpointRow> {
    let checkpoint = checkpoint.into_checkpoint();
    Ok(NewProgressCheckpointRow {
        progress_item_id: checkpoint.item_id().0 as i32,
        recorded_at: format_datetime(checkpoint.recorded_at()),
        effective_date: checkpoint.effective_date().to_string(),
        position_value: checkpoint.position(),
        source: checkpoint.source().map(str::to_string),
        source_event_id: checkpoint.source_event_id().map(str::to_string),
        note: checkpoint.note().map(str::to_string),
        created_at: format_datetime(checkpoint.created_at()),
    })
}

fn row_to_item(row: ProgressItemRow) -> Result<ProgressItem> {
    let tags: HashMap<String, String> =
        serde_json::from_str(&row.tags).context("failed to parse progress item tags")?;
    let metadata = ProgressMetadata::new(row.name, row.category, tags)?;
    let unit = ProgressUnit::from_storage_parts(&row.unit_kind, row.unit_label)?;
    let target = ProgressTarget::new(row.target_value, unit)?;
    ProgressItem::new(
        ProgressItemId(row.id as u32),
        metadata,
        target,
        parse_datetime(&row.created_at)?,
        parse_datetime(&row.updated_at)?,
    )
    .map_err(Into::into)
}

fn row_to_checkpoint(row: ProgressCheckpointRow) -> Result<ProgressCheckpoint> {
    let checkpoint = ProgressCheckpoint::new(
        ProgressItemId(row.progress_item_id as u32),
        parse_datetime(&row.recorded_at)?,
        NaiveDate::parse_from_str(&row.effective_date, "%Y-%m-%d").with_context(|| {
            format!(
                "failed to parse progress checkpoint effective date {}",
                row.effective_date
            )
        })?,
        row.position_value,
        row.source,
        row.source_event_id,
        row.note,
        parse_datetime(&row.created_at)?,
    )?;
    Ok(checkpoint.with_id(ProgressCheckpointId(row.id as u32)))
}

fn load_checkpoint_by_id(
    conn: &mut SqliteConnection,
    id: ProgressCheckpointId,
) -> Result<ProgressCheckpoint> {
    let row = progress_checkpoints::table
        .find(id.0 as i32)
        .first::<ProgressCheckpointRow>(conn)?;
    row_to_checkpoint(row)
}

fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("failed to parse UTC timestamp {value}"))?
        .with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::TimeZone;
    use diesel_migrations::MigrationHarness;

    fn setup_repository() -> SqliteProgressRepository {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("failed to create test pool");

        {
            let mut conn = pool.get().expect("failed to get conn for migrations");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");
        }
        SqliteProgressRepository::new(Arc::new(pool))
    }

    fn create_item(repo: &SqliteProgressRepository) -> ProgressItem {
        let mut tags = HashMap::new();
        tags.insert("media_type".to_string(), "book".to_string());
        let item = NewProgressItem::new(
            ProgressMetadata::new("Book".to_string(), "Reading".to_string(), tags).unwrap(),
            ProgressTarget::new(100, ProgressUnit::Pages).unwrap(),
            Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap(),
        );
        repo.create_item(item).unwrap()
    }

    #[test]
    fn create_and_fetch_progress_item() {
        let repo = setup_repository();
        let item = create_item(&repo);

        let fetched = repo.find_item_by_id(item.id()).unwrap().unwrap();
        assert_eq!(fetched.metadata().name(), "Book");
        assert_eq!(fetched.target().value(), 100);
        assert_eq!(fetched.metadata().tags().get("media_type").unwrap(), "book");
    }

    #[test]
    fn checkpoint_idempotency_returns_duplicate() {
        let repo = setup_repository();
        let item = create_item(&repo);
        let recorded_at = Utc.with_ymd_and_hms(2026, 5, 2, 20, 0, 0).unwrap();
        let command = || {
            NewProgressCheckpoint::new(
                item.id(),
                recorded_at,
                NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
                42,
                Some("ebook".to_string()),
                Some("evt-1".to_string()),
                None,
                recorded_at,
            )
            .unwrap()
        };

        assert!(matches!(
            repo.save_checkpoint(command()).unwrap(),
            SaveCheckpointResult::Inserted(_)
        ));
        assert!(matches!(
            repo.save_checkpoint(command()).unwrap(),
            SaveCheckpointResult::Duplicate(_)
        ));

        let checkpoints = repo
            .find_checkpoints(item.id(), ProgressCheckpointRange::default())
            .unwrap();
        assert_eq!(checkpoints.len(), 1);
    }

    #[test]
    fn find_items_by_tags_matches_subset() {
        let repo = setup_repository();
        let item = create_item(&repo);
        let mut tags = HashMap::new();
        tags.insert("media_type".to_string(), "book".to_string());

        let results = repo.find_items_by_tags(&tags).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), item.id());
    }
}
