//! NEW FILE. SQLite adapter for `OpenNutriTrackerDayStore` (pattern: beurer_day_store.rs).
use chrono::NaiveDate;
use diesel::SqliteConnection;
use lifestats_core::{opennutritracker::*, shared::ids::StatisticId,
    shared::timed_amount_list::TimedAmountList};
use crate::repository::SqliteStatisticRepository;

/// Same BEGIN IMMEDIATE connection as all other mutations in this operation.
fn resolve_metric(conn: &mut SqliteConnection, metric: OpenNutriTrackerMetric, today: NaiveDate)
    -> Result<StatisticId, OpenNutriTrackerDayError> {
    // TODO: json_extract source=SOURCE AND metric=metric.tag() (the same key
    // new_statistic writes). Name/version are not identity. >1 candidates / wrong
    // type / explicit unit != mg => BundleConflict. Reuse a unique correct candidate
    // preserving metadata and goals; else insert_statistic_anyhow(conn,
    // AnyStatistic::TimedAmounts(metric.new_statistic(today))).
    todo!()
}
fn replace_list(conn: &mut SqliteConnection, id: StatisticId, day: NaiveDate, incoming: &TimedAmountList)
    -> Result<bool, OpenNutriTrackerDayError> {
    // TODO: read this row on conn. Deserialize existing JSON as validated list;
    // corruption => Storage error, never treated as absent (a "clear" must not
    // silently succeed over a corrupt row). Typed canonical equality: equal => false;
    // absent or changed => UPSERT canonical JSON, including [].
    // Store observations only; no persisted total.
    todo!()
}
impl OpenNutriTrackerDayStore for SqliteStatisticRepository {
    fn replace_day(&self, day: NaiveDate, today: NaiveDate, values: OpenNutriTrackerDay)
        -> Result<OpenNutriTrackerDayReplacement, OpenNutriTrackerDayError> {
        // TODO: one connection, immediate_transaction:
        //   for metric in OpenNutriTrackerMetric::all() {
        //       let id = resolve_metric(conn, metric, today)?;
        //       if replace_list(conn, id, day, values.list(metric))? { changed.push(id) }
        //   }
        // Commit, THEN return outcome. Any error rolls back provisioning and all
        // writes. Map inner typed errors through anyhow downcast (Beurer convention).
        // Do not call generic repository methods (they acquire another connection).
        todo!()
    }
}
