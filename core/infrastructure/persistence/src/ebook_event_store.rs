use std::sync::Arc;

use anyhow::{anyhow, Context};
use chrono::Utc;
use diesel::connection::Connection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_application::use_cases::integration::{EbookEventStore, EbookSyncEvent};

use crate::db::models::{EbookSyncEventRow, LoggingDataRaw};
use crate::db::schema::{ebook_sync_events, logging_data};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct SqliteEbookEventStore {
    pool: Arc<SqlitePool>,
}

impl SqliteEbookEventStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, ApplicationError> {
        self.pool
            .get()
            .map_err(|err| ApplicationError::Infrastructure(err.into()))
    }
}

impl EbookEventStore for SqliteEbookEventStore {
    fn record_event(&self, event: &EbookSyncEvent) -> ApplicationResult<bool> {
        let mut conn = self.connection()?;

        conn.transaction::<_, anyhow::Error, _>(|conn| {
            let event_row = EbookSyncEventRow {
                event_id: event.event_id.clone(),
                source_name: event.source_name.clone(),
                metric: event.metric.clone(),
                statistic_id: event.statistic_id.0 as i32,
                date: event.date.to_string(),
                delta: event.delta,
                book_id: event.book_id.clone(),
                book_title: event.book_title.clone(),
                confirmed_position_after: event.confirmed_position_after,
                created_at: Utc::now().to_rfc3339(),
            };

            let inserted = diesel::insert_into(ebook_sync_events::table)
                .values(&event_row)
                .on_conflict(ebook_sync_events::event_id)
                .do_nothing()
                .execute(conn)
                .map_err(|err| anyhow!(err))?;

            if inserted == 0 {
                return Ok(false);
            }

            let existing = logging_data::table
                .filter(logging_data::statistic_id.eq(event.statistic_id.0 as i32))
                .filter(logging_data::date.eq(event.date.to_string()))
                .first::<LoggingDataRaw>(conn)
                .optional()
                .map_err(|err| anyhow!(err))?;

            let current_value = existing
                .map(|row| {
                    serde_json::from_str::<i64>(&row.value).with_context(|| {
                        format!(
                            "failed to parse existing ebook sync value for statistic {} on {}",
                            event.statistic_id.0, event.date
                        )
                    })
                })
                .transpose()
                .map_err(|err| anyhow!(err))?
                .unwrap_or_default();

            let new_value = current_value
                .checked_add(event.delta)
                .ok_or_else(|| anyhow!("ebook sync delta overflow"))?;

            let record = LoggingDataRaw {
                statistic_id: event.statistic_id.0 as i32,
                date: event.date.to_string(),
                value: serde_json::to_string(&new_value)?,
            };

            diesel::insert_into(logging_data::table)
                .values(&record)
                .on_conflict((logging_data::statistic_id, logging_data::date))
                .do_update()
                .set(&record)
                .execute(conn)
                .map_err(|err| anyhow!(err))?;

            Ok(true)
        })
        .map_err(ApplicationError::Infrastructure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use diesel_migrations::MigrationHarness;
    use lifestats_core::shared::ids::StatisticId;

    fn setup_store() -> SqliteEbookEventStore {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("failed to build test pool");

        {
            let mut conn = pool.get().expect("failed to get connection");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");

            diesel::sql_query(
                "INSERT INTO statistics (id, name, category, statistic_type, default_goal_valid_since, tags, canonical_tags)
                 VALUES (1, '[Ebook] Characters Read', '読書', 'IntegerAmount', '1970-01-01', '{}', NULL)",
            )
            .execute(&mut conn)
            .expect("failed to seed statistic");
        }

        SqliteEbookEventStore::new(Arc::new(pool))
    }

    fn sample_event(event_id: &str, delta: i64) -> EbookSyncEvent {
        EbookSyncEvent {
            event_id: event_id.to_string(),
            source_name: "ebook".to_string(),
            metric: "characters_read".to_string(),
            statistic_id: StatisticId(1),
            date: chrono::NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
            delta,
            book_id: Some("42".to_string()),
            book_title: "Test Book".to_string(),
            confirmed_position_after: 1234,
        }
    }

    #[test]
    fn records_new_event_once() {
        let store = setup_store();

        let accepted = store
            .record_event(&sample_event("evt-1", 25))
            .expect("record event");
        assert!(accepted);

        let duplicate = store
            .record_event(&sample_event("evt-1", 25))
            .expect("replay event");
        assert!(!duplicate);

        let mut conn = store.connection().expect("connection");
        let logged = logging_data::table
            .first::<LoggingDataRaw>(&mut conn)
            .expect("logged value");

        assert_eq!(logged.statistic_id, 1);
        assert_eq!(logged.date, "2026-04-05");
        assert_eq!(logged.value, "25");
    }

    #[test]
    fn accumulates_multiple_unique_events() {
        let store = setup_store();

        store
            .record_event(&sample_event("evt-1", 25))
            .expect("record first");
        store
            .record_event(&sample_event("evt-2", 10))
            .expect("record second");

        let mut conn = store.connection().expect("connection");
        let logged = logging_data::table
            .first::<LoggingDataRaw>(&mut conn)
            .expect("logged value");

        assert_eq!(logged.value, "35");
    }
}
