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

#[derive(QueryableByName)]
struct LegacyBinding {
    #[diesel(sql_type=Text)]
    book_key: String,
}
#[derive(QueryableByName)]
struct Furthest {
    #[diesel(sql_type=BigInt)]
    position: i64,
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
/// The progress item a key reports into.
struct BookItem {
    id: i32,
    /// Characters credited before this protocol, through the legacy events route.
    legacy: SpanSet,
    /// The key's item had been deleted and was created again, so it has no checkpoint yet.
    recreated: bool,
}

fn ensure_book(
    conn: &mut SqliteConnection,
    book: &BookKey,
    title: &BookTitle,
    length: NonZeroU64,
) -> Result<BookItem, EbookError> {
    let old = read_book(conn, book)?;
    if old
        .as_ref()
        .is_some_and(|b| b.length as u64 != length.get())
    {
        return Err(EbookError::BookLengthMismatch);
    }
    if let Some(row) = old.as_ref() {
        if let Some(id) = row.progress_item_id {
            let legacy = decode_spans(&row.legacy_spans)?;
            return Ok(BookItem {
                id,
                legacy,
                recreated: false,
            });
        }
    }
    // Only an unbound key can be a legacy book's text; a bound key already has its item. A
    // legacy title this API cannot parse can never match, so it must not fail the request.
    for legacy in unbound_legacy_items(conn)? {
        if item_title(&legacy).is_ok_and(|legacy| legacy == *title) {
            return Err(EbookError::LegacyAdoptionPending);
        }
    }
    let id = create_item(conn, book, title, length.get())?;
    let legacy = old
        .as_ref()
        .map(|r| decode_spans(&r.legacy_spans))
        .transpose()?
        .unwrap_or_default();
    diesel::sql_query("INSERT INTO ebook_books(book_key,length,progress_item_id,legacy_spans,created_at) VALUES(?,?,?,'[]',?) ON CONFLICT(book_key) DO UPDATE SET progress_item_id=excluded.progress_item_id")
        .bind::<Text,_>(book.as_str()).bind::<BigInt,_>(length.get() as i64).bind::<Integer,_>(id).bind::<Text,_>(timestamp()).execute(conn).map_err(storage)?;
    Ok(BookItem {
        id,
        legacy,
        recreated: old.is_some(),
    })
}

/// The key a reader data id was already claimed with.
fn existing_claim(
    conn: &mut SqliteConnection,
    reader_data_id: u32,
) -> Result<Option<String>, EbookError> {
    diesel::sql_query("SELECT book_key FROM ebook_legacy_bindings WHERE reader_data_id=?")
        .bind::<BigInt, _>(i64::from(reader_data_id))
        .get_result::<LegacyBinding>(conn)
        .optional()
        .map(|claim| claim.map(|claim| claim.book_key))
        .map_err(storage)
}

fn key_claimed(conn: &mut SqliteConnection, book: &BookKey) -> Result<bool, EbookError> {
    diesel::sql_query("SELECT book_key FROM ebook_legacy_bindings WHERE book_key=?")
        .bind::<Text, _>(book.as_str())
        .get_result::<LegacyBinding>(conn)
        .optional()
        .map(|claim| claim.is_some())
        .map_err(storage)
}

/// A repeated claim succeeds only with the same key and, once it has an item, the same title.
fn confirm_claim(
    conn: &mut SqliteConnection,
    claimed: &str,
    book: &BookKey,
    existing: Option<&BookRow>,
    title: &BookTitle,
) -> Result<(), EbookError> {
    if claimed != book.as_str() {
        return Err(EbookError::LegacyBindingConflict);
    }
    if let Some(id) = existing.and_then(|row| row.progress_item_id) {
        let item = diesel::sql_query("SELECT id,name,tags FROM progress_items WHERE id=?")
            .bind::<Integer, _>(id)
            .get_result::<Item>(conn)
            .map_err(storage)?;
        if item_title(&item)? != *title {
            return Err(EbookError::LegacyBindingConflict);
        }
    }
    Ok(())
}

/// The progress item the legacy events route created for a reader data id.
fn legacy_item(
    conn: &mut SqliteConnection,
    reader_data_id: u32,
) -> Result<Option<Item>, EbookError> {
    diesel::sql_query("SELECT id,name,tags FROM progress_items WHERE json_extract(tags,'$.source')='ebook' AND json_extract(tags,'$.progress_type')='book' AND CAST(json_extract(tags,'$.reader_data_id') AS TEXT)=? ORDER BY id LIMIT 1")
        .bind::<Text, _>(reader_data_id.to_string())
        .get_result::<Item>(conn)
        .optional()
        .map_err(storage)
}

fn furthest_checkpoint(conn: &mut SqliteConnection, item: i32) -> Result<i64, EbookError> {
    diesel::sql_query("SELECT COALESCE(MAX(position_value),0) AS position FROM progress_checkpoints WHERE progress_item_id=?")
        .bind::<Integer, _>(item)
        .get_result::<Furthest>(conn)
        .map(|row| row.position)
        .map_err(storage)
}

/// Tags the legacy item with the key and returns its prefix [0, furthest checkpoint).
fn adopt_legacy_item(
    conn: &mut SqliteConnection,
    legacy: &Item,
    book: &BookKey,
    title: &BookTitle,
    length: NonZeroU64,
    existing: Option<&BookRow>,
) -> Result<SpanSet, EbookError> {
    if item_title(legacy)? != *title
        || existing
            .and_then(|row| row.progress_item_id)
            .is_some_and(|id| id != legacy.id)
    {
        return Err(EbookError::LegacyBindingConflict);
    }
    let other_book = diesel::sql_query("SELECT id FROM progress_items WHERE id=? AND EXISTS(SELECT 1 FROM ebook_books WHERE progress_item_id=progress_items.id AND book_key<>?)")
        .bind::<Integer, _>(legacy.id)
        .bind::<Text, _>(book.as_str())
        .get_result::<Id>(conn)
        .optional()
        .map_err(storage)?;
    if other_book.is_some() {
        return Err(EbookError::LegacyBindingConflict);
    }
    let furthest = furthest_checkpoint(conn, legacy.id)?;
    if furthest < 0 || furthest as u64 > length.get() {
        return Err(EbookError::LegacyBindingConflict);
    }
    let prefix = SpanSet::from_stored(vec![(0, furthest as u64)])?;
    let mut tags: HashMap<String, String> = serde_json::from_str(&legacy.tags).map_err(storage)?;
    tags.insert("book_key".into(), book.as_str().into());
    diesel::sql_query("UPDATE progress_items SET tags=?,canonical_tags=?,updated_at=? WHERE id=?")
        .bind::<Text, _>(serde_json::to_string(&tags).map_err(storage)?)
        .bind::<Nullable<Text>, _>(canonicalize_tags(&tags))
        .bind::<Text, _>(timestamp())
        .bind::<Integer, _>(legacy.id)
        .execute(conn)
        .map_err(storage)?;
    Ok(prefix)
}

/// Points the key at `item` with `prefix` as its legacy spans, and records the claim.
fn bind(
    conn: &mut SqliteConnection,
    reader_data_id: u32,
    book: &BookKey,
    length: NonZeroU64,
    item: i32,
    prefix: &SpanSet,
) -> Result<(), EbookError> {
    diesel::sql_query("INSERT INTO ebook_books(book_key,length,progress_item_id,legacy_spans,created_at) VALUES(?,?,?,?,?) ON CONFLICT(book_key) DO UPDATE SET progress_item_id=excluded.progress_item_id,legacy_spans=excluded.legacy_spans")
        .bind::<Text, _>(book.as_str())
        .bind::<BigInt, _>(length.get() as i64)
        .bind::<Integer, _>(item)
        .bind::<Text, _>(serde_json::to_string(prefix.pairs()).map_err(storage)?)
        .bind::<Text, _>(timestamp())
        .execute(conn)
        .map_err(storage)?;
    diesel::sql_query("INSERT INTO ebook_legacy_bindings(reader_data_id,book_key) VALUES(?,?)")
        .bind::<BigInt, _>(i64::from(reader_data_id))
        .bind::<Text, _>(book.as_str())
        .execute(conn)
        .map_err(storage)?;
    Ok(())
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
            let item=ensure_book(conn,&book,&report.title,report.length)?;
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
                changed=reproject_book(conn,id,&book,&item.legacy,rebuilding)?;
                checkpoint(conn,item.id,producer,day,stored)?;
            } else if item.recreated {
                if let Some(old)=old.as_ref() { checkpoint(conn,item.id,producer,day,old)?; }
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
        let mut conn = self.connection()?;
        conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            let existing = read_book(conn, &book)?;
            if existing
                .as_ref()
                .is_some_and(|row| row.length as u64 != length.get())
            {
                return Err(EbookError::BookLengthMismatch.into());
            }
            if let Some(claimed) = existing_claim(conn, reader_data_id)? {
                confirm_claim(conn, &claimed, &book, existing.as_ref(), &title)?;
                return Ok(EbookWriteOutcome {
                    changed_statistics: vec![],
                });
            }
            if key_claimed(conn, &book)? {
                return Err(EbookError::LegacyBindingConflict.into());
            }
            let legacy = legacy_item(conn, reader_data_id)?;
            let (id, rebuilding) = resolve_statistic(conn, definitions)?;
            let (item, prefix) = match legacy {
                Some(legacy) => {
                    let prefix =
                        adopt_legacy_item(conn, &legacy, &book, &title, length, existing.as_ref())?;
                    (legacy.id, prefix)
                }
                None => {
                    let item = ensure_book(conn, &book, &title, length)?;
                    (item.id, item.legacy)
                }
            };
            let prefix = existing
                .as_ref()
                .map(|row| decode_spans(&row.legacy_spans))
                .transpose()?
                .unwrap_or_default()
                .union(&prefix);
            bind(conn, reader_data_id, &book, length, item, &prefix)?;
            let mut changed = reproject_book(conn, id, &book, &prefix, rebuilding)?;
            if rebuilding {
                changed |= rebuild_statistic(conn, id)?;
            }
            Ok(EbookWriteOutcome {
                changed_statistics: if changed { vec![id] } else { vec![] },
            })
        })
        .map_err(transaction_error)
    }
}
