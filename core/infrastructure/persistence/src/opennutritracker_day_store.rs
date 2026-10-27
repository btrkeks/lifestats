use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable, Text};
use lifestats_core::opennutritracker::{
    OpenNutriTrackerDay, OpenNutriTrackerDayError, OpenNutriTrackerDayReplacement,
    OpenNutriTrackerDayStore, OpenNutriTrackerMetric, SOURCE, UNIT,
};
use lifestats_core::shared::timed_amount_list::TimedAmountList;
use lifestats_core::shared::{ids::StatisticId, ports::error::RepositoryError};
use lifestats_core::statistic::any_statistic::AnyStatistic;

use crate::db::models::LoggingDataRaw;
use crate::db::schema::logging_data;
use crate::repository::SqliteStatisticRepository;

#[derive(QueryableByName)]
struct Candidate {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    statistic_type: String,
    #[diesel(sql_type = Nullable<Text>)]
    unit: Option<String>,
}

fn storage(error: impl Into<anyhow::Error>) -> OpenNutriTrackerDayError {
    OpenNutriTrackerDayError::Storage(RepositoryError::OperationFailed(error.into()))
}

/// Finds the metric's statistic by the same tags `new_statistic` writes, or creates it.
fn resolve_metric(
    conn: &mut SqliteConnection,
    metric: OpenNutriTrackerMetric,
    today: NaiveDate,
) -> Result<StatisticId, OpenNutriTrackerDayError> {
    let candidates = diesel::sql_query(
        "SELECT id, statistic_type, json_extract(tags,'$.unit') AS unit FROM statistics \
         WHERE json_extract(tags,'$.source')=? AND json_extract(tags,'$.metric')=?",
    )
    .bind::<Text, _>(SOURCE)
    .bind::<Text, _>(metric.tag())
    .load::<Candidate>(conn)
    .map_err(storage)?;
    if candidates.len() > 1
        || candidates.iter().any(|candidate| {
            candidate.statistic_type != "TimedAmounts"
                || candidate.unit.as_deref().is_some_and(|unit| unit != UNIT)
        })
    {
        return Err(OpenNutriTrackerDayError::BundleConflict);
    }
    if let Some(candidate) = candidates.first() {
        return Ok(StatisticId(candidate.id as u32));
    }
    let statistic = AnyStatistic::TimedAmounts(metric.new_statistic(today));
    Ok(
        SqliteStatisticRepository::insert_statistic_anyhow(conn, &statistic)
            .map_err(storage)?
            .get_metadata()
            .id(),
    )
}

/// Stores `incoming` unless the day already holds an equal list. A stored value that
/// is not a valid list is an error, never treated as absent.
fn replace_list(
    conn: &mut SqliteConnection,
    id: StatisticId,
    day: NaiveDate,
    incoming: &TimedAmountList,
) -> Result<bool, OpenNutriTrackerDayError> {
    let current = logging_data::table
        .find((id.0 as i32, day.to_string()))
        .first::<LoggingDataRaw>(conn)
        .optional()
        .map_err(storage)?
        .map(|row| row.value.parse::<TimedAmountList>())
        .transpose()
        .map_err(storage)?;
    if current.as_ref() == Some(incoming) {
        return Ok(false);
    }
    let row = LoggingDataRaw {
        statistic_id: id.0 as i32,
        date: day.to_string(),
        value: serde_json::to_string(incoming).map_err(storage)?,
    };
    diesel::insert_into(logging_data::table)
        .values(&row)
        .on_conflict((logging_data::statistic_id, logging_data::date))
        .do_update()
        .set(&row)
        .execute(conn)
        .map_err(storage)?;
    Ok(true)
}

impl OpenNutriTrackerDayStore for SqliteStatisticRepository {
    fn replace_day(
        &self,
        day: NaiveDate,
        today: NaiveDate,
        values: OpenNutriTrackerDay,
    ) -> Result<OpenNutriTrackerDayReplacement, OpenNutriTrackerDayError> {
        let mut conn = self.connection()?;
        // Generic repository methods take their own connection; everything here
        // must use `conn` so provisioning and both writes commit or roll back together.
        conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            let mut changed_statistics = Vec::new();
            for metric in OpenNutriTrackerMetric::all() {
                let id = resolve_metric(conn, metric, today)?;
                if replace_list(conn, id, day, values.list(metric))? {
                    changed_statistics.push(id);
                }
            }
            Ok(OpenNutriTrackerDayReplacement { changed_statistics })
        })
        .map_err(|error| match error.downcast::<OpenNutriTrackerDayError>() {
            Ok(error) => error,
            Err(error) => storage(error),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel_migrations::MigrationHarness;
    use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
    use lifestats_core::statistic::traits::base_statistic::Statistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::timed_amounts::TimedAmountsStatistic;
    use lifestats_core::statistic::value_objects::goals::timed_amounts_goal::TimedAmountsGoal;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;
    use std::sync::Arc;

    fn repository() -> SqliteStatisticRepository {
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(":memory:"))
            .unwrap();
        pool.get()
            .unwrap()
            .run_pending_migrations(MIGRATIONS)
            .unwrap();
        SqliteStatisticRepository::new(Arc::new(pool))
    }

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn list(value: serde_json::Value) -> TimedAmountList {
        serde_json::from_value(value).unwrap()
    }

    fn day(caffeine: serde_json::Value, nicotine: serde_json::Value) -> OpenNutriTrackerDay {
        OpenNutriTrackerDay::new(list(caffeine), list(nicotine))
    }

    fn statistic_for(
        repo: &SqliteStatisticRepository,
        metric: OpenNutriTrackerMetric,
    ) -> Option<AnyStatistic> {
        repo.find_all_metadata()
            .unwrap()
            .into_iter()
            .find(|statistic| {
                statistic
                    .get_metadata()
                    .get_tag("metric")
                    .map(String::as_str)
                    == Some(metric.tag())
            })
    }

    #[test]
    fn first_sync_provisions_both_and_replays_are_unchanged() {
        let repo = repository();
        let caffeine =
            json!([{"time": "13:40:00", "amount": 60}, {"time": "08:12:00", "amount": 120}]);
        let first = repo
            .replace_day(date(27), date(27), day(caffeine.clone(), json!([])))
            .unwrap();
        assert_eq!(first.changed_statistics.len(), 2);

        let caffeine_stat = statistic_for(&repo, OpenNutriTrackerMetric::Caffeine).unwrap();
        assert!(matches!(caffeine_stat, AnyStatistic::TimedAmounts(_)));
        assert_eq!(
            caffeine_stat.get_metadata().name(),
            "[OpenNutriTracker] Caffeine (mg)"
        );
        assert_eq!(
            repo.find_log_value(caffeine_stat.id(), date(27)).unwrap(),
            Some(json!([{"time": "08:12:00", "amount": 120}, {"time": "13:40:00", "amount": 60}]))
        );
        let nicotine_stat = statistic_for(&repo, OpenNutriTrackerMetric::Nicotine).unwrap();
        assert_eq!(
            repo.find_log_value(nicotine_stat.id(), date(27)).unwrap(),
            Some(json!([]))
        );

        let reordered =
            json!([{"time": "08:12:00", "amount": 120}, {"time": "13:40:00", "amount": 60}]);
        let replay = repo
            .replace_day(date(27), date(28), day(reordered, json!([])))
            .unwrap();
        assert!(replay.changed_statistics.is_empty());
        assert_eq!(repo.find_all_metadata().unwrap().len(), 2);

        let edited = repo
            .replace_day(
                date(27),
                date(28),
                day(json!([]), json!([{"time": null, "amount": 2}])),
            )
            .unwrap();
        assert_eq!(edited.changed_statistics.len(), 2);
        assert_eq!(
            repo.find_log_value(caffeine_stat.id(), date(27)).unwrap(),
            Some(json!([]))
        );
        assert_eq!(
            repo.find_log_value(caffeine_stat.id(), date(26)).unwrap(),
            None
        );
    }

    #[test]
    fn metadata_and_goal_round_trip_and_survive_syncs() {
        let repo = repository();
        repo.replace_day(date(27), date(27), day(json!([]), json!([])))
            .unwrap();
        let caffeine = statistic_for(&repo, OpenNutriTrackerMetric::Caffeine).unwrap();
        let goal = TimedAmountsGoal::new(400, false).unwrap();
        repo.save_default_goal(caffeine.id(), serde_json::to_value(&goal).unwrap())
            .unwrap();
        repo.replace_day(
            date(28),
            date(28),
            day(json!([{"time": "09:00:00", "amount": 95}]), json!([])),
        )
        .unwrap();

        let reloaded = repo.find_metadata_by_id(caffeine.id()).unwrap();
        let AnyStatistic::TimedAmounts(reloaded) = reloaded else {
            panic!("expected TimedAmounts");
        };
        let goal = reloaded.get_default_goal().unwrap();
        assert_eq!((goal.target_amount(), goal.is_at_least()), (400, false));
        assert_eq!(
            reloaded.get_metadata().get_tag("unit").map(String::as_str),
            Some("mg")
        );
    }

    #[test]
    fn conflicting_bindings_write_nothing() {
        let repo = repository();
        let impostor = StatisticMetadata::new(StatisticId(0), "x".into(), "Nicotine".into())
            .with_tag("source".into(), SOURCE.into())
            .with_tag("metric".into(), "nicotine".into());
        repo.create_metadata(AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            impostor,
            None,
            date(1),
        )))
        .unwrap();

        let result = repo.replace_day(
            date(27),
            date(27),
            day(json!([{"time": null, "amount": 50}]), json!([])),
        );
        assert!(matches!(
            result,
            Err(OpenNutriTrackerDayError::BundleConflict)
        ));
        // Caffeine was resolved first; its creation rolled back with the transaction.
        assert!(statistic_for(&repo, OpenNutriTrackerMetric::Caffeine).is_none());
    }

    #[test]
    fn wrong_unit_and_duplicates_conflict() {
        let repo = repository();
        let grams = StatisticMetadata::new(StatisticId(0), "x".into(), "Caffeine (g)".into())
            .with_tag("source".into(), SOURCE.into())
            .with_tag("metric".into(), "caffeine".into())
            .with_tag("unit".into(), "g".into());
        repo.create_metadata(AnyStatistic::TimedAmounts(TimedAmountsStatistic::new(
            grams,
            None,
            date(1),
        )))
        .unwrap();
        assert!(matches!(
            repo.replace_day(date(27), date(27), day(json!([]), json!([]))),
            Err(OpenNutriTrackerDayError::BundleConflict)
        ));

        let repo = repository();
        for _ in 0..2 {
            repo.create_metadata(AnyStatistic::TimedAmounts(
                OpenNutriTrackerMetric::Caffeine.new_statistic(date(1)),
            ))
            .unwrap();
        }
        assert!(matches!(
            repo.replace_day(date(27), date(27), day(json!([]), json!([]))),
            Err(OpenNutriTrackerDayError::BundleConflict)
        ));
    }

    #[test]
    fn adopts_a_correct_statistic_without_unit_tag() {
        let repo = repository();
        let metadata = StatisticMetadata::new(StatisticId(0), "Mine".into(), "Coffee".into())
            .with_tag("source".into(), SOURCE.into())
            .with_tag("metric".into(), "caffeine".into());
        let existing = repo
            .create_metadata(AnyStatistic::TimedAmounts(TimedAmountsStatistic::new(
                metadata,
                None,
                date(1),
            )))
            .unwrap();
        repo.replace_day(
            date(27),
            date(27),
            day(json!([{"time": null, "amount": 5}]), json!([])),
        )
        .unwrap();
        assert_eq!(
            statistic_for(&repo, OpenNutriTrackerMetric::Caffeine)
                .unwrap()
                .id(),
            existing.id()
        );
        assert_eq!(
            repo.find_metadata_by_id(existing.id())
                .unwrap()
                .get_metadata()
                .name(),
            "Coffee"
        );
    }

    #[test]
    fn corrupt_stored_day_is_an_error_not_absent() {
        let repo = repository();
        repo.replace_day(date(27), date(27), day(json!([]), json!([])))
            .unwrap();
        let caffeine = statistic_for(&repo, OpenNutriTrackerMetric::Caffeine).unwrap();
        repo.save_log_value(caffeine.id(), date(27), json!(42))
            .unwrap();

        let result = repo.replace_day(date(27), date(27), day(json!([]), json!([])));
        assert!(matches!(result, Err(OpenNutriTrackerDayError::Storage(_))));
        assert_eq!(
            repo.find_log_value(caffeine.id(), date(27)).unwrap(),
            Some(json!(42))
        );
    }
}
