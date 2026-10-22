use crate::{db::mappers::canonicalize_tags, repository::SqliteStatisticRepository};
use chrono::{NaiveDate, SecondsFormat, Utc};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer, Nullable, Text},
};
use lifestats_core::{
    ebook::{
        credit_days, merge_day, BookKey, BookTitle, EbookBundleDefinitions, EbookDayReport,
        EbookError, EbookReadingStore, EbookWriteOutcome, ProducerId, SpanSet, StoredDay,
    },
    shared::{
        ids::StatisticId, ports::error::RepositoryError, projection::replace_source_projection,
    },
    statistic::any_statistic::AnyStatistic,
};
use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroU64,
};

#[derive(QueryableByName)]
struct Identity {
    #[diesel(sql_type=Text)]
    server_id: String,
}
#[derive(QueryableByName)]
struct Binding {
    #[diesel(sql_type=Nullable<Integer>)]
    statistic_id: Option<i32>,
}
#[derive(QueryableByName)]
struct Candidate {
    #[diesel(sql_type=Integer)]
    id: i32,
    #[diesel(sql_type=Text)]
    statistic_type: String,
}
#[derive(QueryableByName)]
struct BookRow {
    #[diesel(sql_type=BigInt)]
    length: i64,
    #[diesel(sql_type=Nullable<Integer>)]
    progress_item_id: Option<i32>,
    #[diesel(sql_type=Text)]
    legacy_spans: String,
}
#[derive(QueryableByName)]
struct Item {
    #[diesel(sql_type=Integer)]
    id: i32,
    #[diesel(sql_type=Text)]
    name: String,
    #[diesel(sql_type=Text)]
    tags: String,
}
#[derive(QueryableByName)]
struct DayRow {
    #[diesel(sql_type=Text)]
    day: String,
    #[diesel(sql_type=Text)]
    spans: String,
    #[diesel(sql_type=BigInt)]
    last_position: i64,
    #[diesel(sql_type=Text)]
    observed_at: String,
}
impl DayRow {
    fn stored(&self) -> Result<StoredDay, EbookError> {
        Ok(StoredDay {
            spans: decode_spans(&self.spans)?,
            last_position: u64::try_from(self.last_position).map_err(storage)?,
            observed_at: self.observed_at.parse().map_err(storage)?,
        })
    }
}
#[derive(QueryableByName)]
struct Credit {
    #[diesel(sql_type=Text)]
    day: String,
    #[diesel(sql_type=BigInt)]
    characters: i64,
}
#[derive(QueryableByName)]
struct Id {
    #[diesel(sql_type=Integer)]
    id: i32,
}

fn storage(error: impl Into<anyhow::Error>) -> EbookError {
    EbookError::Storage(RepositoryError::OperationFailed(error.into()))
}
fn transaction_error(error: anyhow::Error) -> EbookError {
    match error.downcast::<EbookError>() {
        Ok(error) => error,
        Err(error) => storage(error),
    }
}
fn decode_spans(raw: &str) -> Result<SpanSet, EbookError> {
    SpanSet::from_stored(serde_json::from_str(raw).map_err(storage)?)
        .map_err(|e| storage(anyhow::anyhow!(e)))
}
fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn read_book(conn: &mut SqliteConnection, book: &BookKey) -> Result<Option<BookRow>, EbookError> {
    diesel::sql_query(
        "SELECT length,progress_item_id,legacy_spans FROM ebook_books WHERE book_key=?",
    )
    .bind::<Text, _>(book.as_str())
    .get_result(conn)
    .optional()
    .map_err(storage)
}

fn resolve_statistic(
    conn: &mut SqliteConnection,
    definitions: EbookBundleDefinitions,
) -> Result<(StatisticId, bool), EbookError> {
    let binding = diesel::sql_query(
        "SELECT statistic_id FROM ebook_metric_bindings WHERE metric='characters_read'",
    )
    .get_result::<Binding>(conn)
    .optional()
    .map_err(storage)?;
    let candidates=diesel::sql_query("SELECT id,statistic_type FROM statistics WHERE json_extract(tags,'$.source')='ebook' AND json_extract(tags,'$.metric')='characters_read'")
        .load::<Candidate>(conn).map_err(storage)?;
    if candidates.len() > 1
        || candidates
            .iter()
            .any(|c| c.statistic_type != "IntegerAmount")
    {
        return Err(EbookError::BundleConflict);
    }
    if let Some(id) = binding.as_ref().and_then(|b| b.statistic_id) {
        if candidates.first().map(|c| c.id) != Some(id) {
            return Err(EbookError::BundleConflict);
        }
        return Ok((StatisticId(id as u32), false));
    }
    let id = match candidates.first() {
        Some(c) => StatisticId(c.id as u32),
        None => SqliteStatisticRepository::insert_statistic_anyhow(
            conn,
            &AnyStatistic::IntegerAmount(definitions.characters_read),
        )
        .map_err(storage)?
        .get_metadata()
        .id(),
    };
    diesel::sql_query("INSERT INTO ebook_metric_bindings(metric,statistic_id) VALUES('characters_read',?) ON CONFLICT(metric) DO UPDATE SET statistic_id=excluded.statistic_id WHERE statistic_id IS NOT excluded.statistic_id")
        .bind::<Integer,_>(id.0 as i32).execute(conn).map_err(storage)?;
    Ok((id, binding.is_some()))
}

fn unbound_legacy_items(conn: &mut SqliteConnection) -> Result<Vec<Item>, EbookError> {
    diesel::sql_query("SELECT id,name,tags FROM progress_items WHERE json_extract(tags,'$.source')='ebook' AND json_extract(tags,'$.progress_type')='book' AND json_extract(tags,'$.reader_data_id') IS NOT NULL AND NOT EXISTS (SELECT 1 FROM ebook_books b WHERE b.progress_item_id=progress_items.id)")
        .load(conn).map_err(storage)
}
fn item_title(item: &Item) -> Result<BookTitle, EbookError> {
    let tags: HashMap<String, String> = serde_json::from_str(&item.tags).map_err(storage)?;
    BookTitle::parse(
        tags.get("book_title")
            .map_or(item.name.as_str(), String::as_str),
    )
}

fn create_item(
    conn: &mut SqliteConnection,
    book: &BookKey,
    title: &BookTitle,
    length: u64,
) -> Result<i32, EbookError> {
    let tags = HashMap::from([
        ("source".into(), "ebook".into()),
        ("progress_type".into(), "book".into()),
        ("book_key".into(), book.as_str().into()),
    ]);
    let now = timestamp();
    diesel::sql_query("INSERT INTO progress_items(name,category,unit_kind,target_value,tags,canonical_tags,created_at,updated_at) VALUES(?,'読書','Characters',?,?,?,?,?)")
        .bind::<Text,_>(title.as_str()).bind::<BigInt,_>(length as i64)
        .bind::<Text,_>(serde_json::to_string(&tags).map_err(storage)?).bind::<Nullable<Text>,_>(canonicalize_tags(&tags))
        .bind::<Text,_>(&now).bind::<Text,_>(&now).execute(conn).map_err(storage)?;
    diesel::sql_query("SELECT last_insert_rowid() AS id")
        .get_result::<Id>(conn)
        .map(|r| r.id)
        .map_err(storage)
}
fn ensure_book(
    conn: &mut SqliteConnection,
    book: &BookKey,
    title: &BookTitle,
    length: NonZeroU64,
) -> Result<(i32, SpanSet, bool), EbookError> {
    for legacy in unbound_legacy_items(conn)? {
        if item_title(&legacy)? == *title {
            return Err(EbookError::LegacyAdoptionPending);
        }
    }
    let old = read_book(conn, book)?;
    if old
        .as_ref()
        .is_some_and(|b| b.length as u64 != length.get())
    {
        return Err(EbookError::BookLengthMismatch);
    }
    if let Some(item) = old.as_ref().and_then(|b| b.progress_item_id) {
        return Ok((item, decode_spans(&old.unwrap().legacy_spans)?, false));
    }
    let item = create_item(conn, book, title, length.get())?;
    let restored = old.is_some();
    let legacy = old
        .as_ref()
        .map(|r| decode_spans(&r.legacy_spans))
        .transpose()?
        .unwrap_or_default();
    diesel::sql_query("INSERT INTO ebook_books(book_key,length,progress_item_id,legacy_spans,created_at) VALUES(?,?,?,'[]',?) ON CONFLICT(book_key) DO UPDATE SET progress_item_id=excluded.progress_item_id")
        .bind::<Text,_>(book.as_str()).bind::<BigInt,_>(length.get() as i64).bind::<Integer,_>(item).bind::<Text,_>(timestamp()).execute(conn).map_err(storage)?;
    Ok((item, legacy, restored))
}

fn reproject_book(
    conn: &mut SqliteConnection,
    id: StatisticId,
    book: &BookKey,
    legacy: &SpanSet,
    rebuilding: bool,
) -> Result<bool, EbookError> {
    let rows=diesel::sql_query("SELECT day,spans,last_position,observed_at FROM ebook_day_spans WHERE book_key=? ORDER BY day")
        .bind::<Text,_>(book.as_str()).load::<DayRow>(conn).map_err(storage)?;
    let rows: Vec<_> = rows
        .iter()
        .map(|r| {
            Ok((
                r.day.parse::<NaiveDate>().map_err(storage)?,
                decode_spans(&r.spans)?,
            ))
        })
        .collect::<Result<_, EbookError>>()?;
    let credits = credit_days(legacy, rows.iter().map(|(d, s)| (*d, s)));
    let old = diesel::sql_query("SELECT day,characters FROM ebook_day_credits WHERE book_key=?")
        .bind::<Text, _>(book.as_str())
        .load::<Credit>(conn)
        .map_err(storage)?;
    let old: BTreeMap<NaiveDate, i64> = old
        .into_iter()
        .map(|r| Ok((r.day.parse().map_err(storage)?, r.characters)))
        .collect::<Result<_, EbookError>>()?;
    let mut changed = false;
    for (day, new) in credits {
        let new = new as i64;
        if !rebuilding && old.get(&day).copied().unwrap_or(0) != new {
            let current = SqliteStatisticRepository::read_integer(conn, id, day)
                .map_err(storage)?
                .unwrap_or(0);
            let next =
                replace_source_projection(current, old.get(&day).copied().unwrap_or(0), new)?;
            SqliteStatisticRepository::write_integer(conn, id, day, next).map_err(storage)?;
            changed = true;
        }
        if old.get(&day) != Some(&new) {
            diesel::sql_query("INSERT INTO ebook_day_credits(book_key,day,characters) VALUES(?,?,?) ON CONFLICT(book_key,day) DO UPDATE SET characters=excluded.characters")
                .bind::<Text,_>(book.as_str()).bind::<Text,_>(day.to_string()).bind::<BigInt,_>(new).execute(conn).map_err(storage)?;
        }
    }
    Ok(changed)
}
fn rebuild_statistic(conn: &mut SqliteConnection, id: StatisticId) -> Result<bool, EbookError> {
    let rows = diesel::sql_query("SELECT day,characters FROM ebook_day_credits")
        .load::<Credit>(conn)
        .map_err(storage)?;
    let mut totals: BTreeMap<NaiveDate, i128> = BTreeMap::new();
    for row in rows {
        *totals.entry(row.day.parse().map_err(storage)?).or_default() += i128::from(row.characters);
    }
    let mut changed = false;
    for (day, total) in totals {
        let current = SqliteStatisticRepository::read_integer(conn, id, day).map_err(storage)?;
        let next = i64::try_from(i128::from(current.unwrap_or(0)) + total)
            .map_err(|_| EbookError::ProjectionOverflow)?;
        if current != Some(next) {
            SqliteStatisticRepository::write_integer(conn, id, day, next).map_err(storage)?;
            changed = true;
        }
    }
    Ok(changed)
}
fn checkpoint(
    conn: &mut SqliteConnection,
    item: i32,
    producer: ProducerId,
    day: NaiveDate,
    stored: &StoredDay,
) -> Result<(), EbookError> {
    let observed = stored
        .observed_at
        .to_rfc3339_opts(SecondsFormat::Nanos, true);
    diesel::sql_query("INSERT INTO progress_checkpoints(progress_item_id,recorded_at,effective_date,position_value,source,source_event_id,created_at) VALUES(?,?,?,?,'ebook',?,?) ON CONFLICT(progress_item_id,source,source_event_id) WHERE source IS NOT NULL AND source_event_id IS NOT NULL DO UPDATE SET position_value=excluded.position_value,recorded_at=excluded.recorded_at WHERE excluded.recorded_at>progress_checkpoints.recorded_at")
        .bind::<Integer,_>(item).bind::<Text,_>(observed).bind::<Text,_>(day.to_string()).bind::<BigInt,_>(stored.last_position as i64)
        .bind::<Text,_>(format!("{producer}:{day}")).bind::<Text,_>(timestamp()).execute(conn).map_err(storage)?;
    Ok(())
}

impl EbookReadingStore for SqliteStatisticRepository {
    fn server_id(&self) -> Result<String, EbookError> {
        let mut conn = self.connection()?;
        conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            if let Some(row) =
                diesel::sql_query("SELECT server_id FROM ebook_server_identity WHERE singleton=1")
                    .get_result::<Identity>(conn)
                    .optional()?
            {
                return Ok(row.server_id);
            }
            let server_id = uuid::Uuid::new_v4().to_string();
            diesel::sql_query("INSERT INTO ebook_server_identity(singleton,server_id) VALUES(1,?)")
                .bind::<Text, _>(&server_id)
                .execute(conn)?;
            Ok(server_id)
        })
        .map_err(transaction_error)
    }
    fn record_day(
        &self,
        producer: ProducerId,
        book: BookKey,
        day: NaiveDate,
        report: EbookDayReport,
        definitions: EbookBundleDefinitions,
    ) -> Result<EbookWriteOutcome, EbookError> {
        let mut conn = self.connection()?;
        conn.immediate_transaction::<_,anyhow::Error,_>(|conn| {
            let (id,rebuilding)=resolve_statistic(conn,definitions)?;
            let (item,legacy,restored)=ensure_book(conn,&book,&report.title,report.length)?;
            let old=diesel::sql_query("SELECT day,spans,last_position,observed_at FROM ebook_day_spans WHERE producer_id=? AND book_key=? AND day=?")
                .bind::<Text,_>(producer.to_string()).bind::<Text,_>(book.as_str()).bind::<Text,_>(day.to_string()).get_result::<DayRow>(conn).optional().map_err(storage)?;
            let old=old.as_ref().map(DayRow::stored).transpose()?;
            let merged=merge_day(old.as_ref(),&report);
            let mut changed=false;
            if let Some(stored)=merged.as_ref() {
                diesel::sql_query("INSERT INTO ebook_day_spans(producer_id,book_key,day,spans,last_position,observed_at) VALUES(?,?,?,?,?,?) ON CONFLICT(producer_id,book_key,day) DO UPDATE SET spans=excluded.spans,last_position=excluded.last_position,observed_at=excluded.observed_at")
                    .bind::<Text,_>(producer.to_string()).bind::<Text,_>(book.as_str()).bind::<Text,_>(day.to_string())
                    .bind::<Text,_>(serde_json::to_string(stored.spans.pairs()).map_err(storage)?)
                    .bind::<BigInt,_>(stored.last_position as i64).bind::<Text,_>(stored.observed_at.to_rfc3339_opts(SecondsFormat::Nanos,true)).execute(conn).map_err(storage)?;
                changed=reproject_book(conn,id,&book,&legacy,rebuilding)?;
                checkpoint(conn,item,producer,day,stored)?;
            } else if restored {
                if let Some(old)=old.as_ref() { checkpoint(conn,item,producer,day,old)?; }
            }
            if rebuilding { changed|=rebuild_statistic(conn,id)?; }
            Ok(EbookWriteOutcome { changed_statistics: if changed {vec![id]} else {vec![]} })
        }).map_err(transaction_error)
    }
    fn bind_legacy_book(
        &self,
        reader_data_id: u32,
        book: BookKey,
        title: BookTitle,
        length: NonZeroU64,
        definitions: EbookBundleDefinitions,
    ) -> Result<EbookWriteOutcome, EbookError> {
        let _ = (reader_data_id, book, title, length, definitions);
        todo!("legacy adoption unit")
    }
}
