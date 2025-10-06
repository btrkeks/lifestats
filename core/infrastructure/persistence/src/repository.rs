use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::db::mappers::StatisticDto;
use crate::db::models::{DefaultGoalRaw, GoalDataRaw, LoggingDataRaw};
use crate::db::queries;
use crate::db::schema::{default_goals, goals, logging_data, statistics};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct SqliteStatisticRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteStatisticRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.pool
            .get()
            .context("failed to get SQLite connection from pool")
    }
}

impl StatisticRepository for SqliteStatisticRepository {
    fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic> {
        let mut conn = self.connection()?;

        let dto = statistics::table
            .find(id.0 as i32)
            .first::<StatisticDto>(&mut conn)
            .with_context(|| format!("statistic {} not found", id.0))?;

        let default_goal = queries::fetch_default_goal(&mut conn, id)?;
        dto.into_any_statistic(default_goal)
    }

    fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>> {
        let mut conn = self.connection()?;

        let dtos = statistics::table
            .load::<StatisticDto>(&mut conn)
            .context("failed to load statistics")?;

        let ids: Vec<i32> = dtos.iter().map(|dto| dto.id).collect();
        let default_goal_map = queries::fetch_default_goals(&mut conn, &ids)?;

        let mut result = Vec::with_capacity(dtos.len());
        for dto in dtos {
            let default_goal = default_goal_map.get(&dto.id).cloned();
            result.push(dto.into_any_statistic(default_goal)?);
        }

        Ok(result)
    }

    fn save_metadata(&self, statistic: &AnyStatistic) -> Result<()> {
        let mut conn = self.connection()?;
        let dto = StatisticDto::from_any_statistic(statistic)?;

        diesel::insert_into(statistics::table)
            .values(&dto)
            .on_conflict(statistics::id)
            .do_update()
            .set(&dto)
            .execute(&mut conn)
            .context("failed to upsert statistic metadata")?;

        Ok(())
    }

    fn delete_default_goal(&self, id: StatisticId) -> Result<()> {
        let mut conn = self.connection()?;
        diesel::delete(default_goals::table.find(id.0 as i32))
            .execute(&mut conn)
            .context("failed to delete default goal")?;
        Ok(())
    }

    fn find_log_value(
        &self,
        id: StatisticId,
        date: NaiveDate,
    ) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection()?;
        let row = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.eq(date.to_string()))
            .first::<LoggingDataRaw>(&mut conn)
            .optional()
            .context("failed to fetch log value")?;

        row.map(|raw| serde_json::from_str(&raw.value).context("failed to parse log value JSON"))
            .transpose()
    }

    fn save_log_value(
        &self,
        id: StatisticId,
        date: NaiveDate,
        value: serde_json::Value,
    ) -> Result<()> {
        let mut conn = self.connection()?;
        let serialized = serde_json::to_string(&value).with_context(|| {
            format!(
                "failed to serialize log value for statistic {} on {}",
                id.0, date
            )
        })?;

        let record = LoggingDataRaw {
            statistic_id: id.0 as i32,
            date: date.to_string(),
            value: serialized,
        };

        diesel::insert_into(logging_data::table)
            .values(&record)
            .on_conflict((logging_data::statistic_id, logging_data::date))
            .do_update()
            .set(&record)
            .execute(&mut conn)
            .context("failed to upsert log value")?;

        Ok(())
    }

    fn find_log_values_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>> {
        let mut conn = self.connection()?;
        let rows = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.ge(range.start().to_string()))
            .filter(logging_data::date.le(range.end().to_string()))
            .order(logging_data::date.asc())
            .load::<LoggingDataRaw>(&mut conn)
            .context("failed to fetch logging data range")?;

        rows.into_iter()
            .map(|raw| {
                let date = raw
                    .date
                    .parse::<NaiveDate>()
                    .with_context(|| format!("failed to parse date: {}", raw.date))?;
                let value =
                    serde_json::from_str(&raw.value).context("failed to parse log value JSON")?;
                Ok(DatedValue::new(date, value))
            })
            .collect()
    }

    fn find_goal(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection()?;
        let row = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.eq(date.to_string()))
            .first::<GoalDataRaw>(&mut conn)
            .optional()
            .context("failed to fetch goal")?;

        match row {
            Some(raw) => match raw.goal {
                Some(goal_str) => {
                    let goal = serde_json::from_str(&goal_str)
                        .with_context(|| format!("failed to parse goal JSON: {}", goal_str))?;
                    Ok(Some(goal))
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn save_goal(
        &self,
        id: StatisticId,
        date: NaiveDate,
        goal: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut conn = self.connection()?;

        if let Some(goal_value) = goal {
            let goal_str =
                serde_json::to_string(&goal_value).context("failed to serialize goal to JSON")?;

            let record = GoalDataRaw {
                statistic_id: id.0 as i32,
                date: date.to_string(),
                goal: Some(goal_str),
            };

            diesel::insert_into(goals::table)
                .values(&record)
                .on_conflict((goals::statistic_id, goals::date))
                .do_update()
                .set(&record)
                .execute(&mut conn)
                .context("failed to upsert goal")?;
        } else {
            diesel::delete(
                goals::table
                    .filter(goals::statistic_id.eq(id.0 as i32))
                    .filter(goals::date.eq(date.to_string())),
            )
            .execute(&mut conn)
            .context("failed to delete goal")?;
        }

        Ok(())
    }

    fn find_goals_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>> {
        let mut conn = self.connection()?;
        let rows = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.ge(range.start().to_string()))
            .filter(goals::date.le(range.end().to_string()))
            .order(goals::date.asc())
            .load::<GoalDataRaw>(&mut conn)
            .context("failed to fetch goals range")?;

        let mut result = Vec::new();
        for row in rows {
            if let Some(goal_str) = row.goal {
                let date = row
                    .date
                    .parse::<NaiveDate>()
                    .with_context(|| format!("failed to parse date: {}", row.date))?;
                let goal = serde_json::from_str(&goal_str)
                    .with_context(|| format!("failed to parse goal JSON: {}", goal_str))?;
                result.push(DatedValue::new(date, goal));
            }
        }
        Ok(result)
    }

    fn find_default_goal(&self, id: StatisticId) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection()?;
        let row = queries::fetch_default_goal(&mut conn, id)?;

        match row {
            Some(raw) => {
                let goal = serde_json::from_str(&raw.goal).with_context(|| {
                    format!(
                        "Failed to parse default goal '{}' for statistic_id {}",
                        raw.goal, raw.statistic_id
                    )
                })?;
                Ok(Some(goal))
            }
            None => Ok(None),
        }
    }

    fn save_default_goal(&self, id: StatisticId, goal: serde_json::Value) -> Result<()> {
        let mut conn = self.connection()?;
        let serialized_goal = serde_json::to_string(&goal)?;

        let record = DefaultGoalRaw {
            statistic_id: id.0 as i32,
            goal: serialized_goal,
        };

        diesel::insert_into(default_goals::table)
            .values(&record)
            .on_conflict(default_goals::statistic_id)
            .do_update()
            .set(default_goals::goal.eq(&record.goal))
            .execute(&mut conn)
            .context("failed to upsert default goal")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::NaiveDate;
    use diesel_migrations::MigrationHarness;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    // ==================== Test Helper ====================

    fn setup_repository() -> SqliteStatisticRepository {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1) // Tests are single-threaded, 1 is sufficient
            .build(manager)
            .expect("failed to create test pool");

        {
            let mut conn = pool.get().expect("failed to get conn for migrations");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");
        }
        SqliteStatisticRepository::new(Arc::new(pool))
    }

    fn integer_stat(id: u32) -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(id),
            "Fitness".to_string().into_boxed_str(),
            "Push Ups".to_string().into_boxed_str(),
        );
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date"),
        ))
    }

    fn habit_stat(id: u32) -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(id),
            "Health".to_string().into_boxed_str(),
            "Brushed Teeth".to_string().into_boxed_str(),
        );
        AnyStatistic::Habit(HabitStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date"),
        ))
    }

    fn time_spans_stat(id: u32) -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(id),
            "Productivity".to_string().into_boxed_str(),
            "Study Time".to_string().into_boxed_str(),
        );
        AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date"),
        ))
    }

    // ==================== Metadata Tests ====================

    mod metadata_tests {
        use super::*;

        #[test]
        fn save_and_fetch_metadata() -> Result<()> {
            let repo = setup_repository();
            let stat = integer_stat(1);

            repo.save_metadata(&stat)?;

            let fetched = repo.find_metadata_by_id(StatisticId(1))?;
            assert!(matches!(fetched, AnyStatistic::IntegerAmount(_)));
            assert_eq!(fetched.get_metadata().name(), "Push Ups");
            Ok(())
        }

        #[test]
        fn fetch_all_metadata() -> Result<()> {
            let repo = setup_repository();

            repo.save_metadata(&integer_stat(1))?;
            repo.save_metadata(&habit_stat(2))?;
            repo.save_metadata(&time_spans_stat(3))?;

            let all = repo.find_all_metadata()?;
            assert_eq!(all.len(), 3);
            Ok(())
        }
    }

    // ==================== JSON Interface Tests ====================

    mod json_interface_tests {
        use super::*;

        #[test]
        fn save_and_find_log_value() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
            let value = serde_json::json!(123);

            repo.save_log_value(StatisticId(1), date, value.clone())?;

            let fetched = repo.find_log_value(StatisticId(1), date)?;
            assert_eq!(fetched, Some(value));
            Ok(())
        }

        #[test]
        fn save_and_find_goal() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 3, 2).unwrap();
            let goal = serde_json::json!({
                "target": 100,
                "extra_data1": "some_data"
            });

            repo.save_goal(StatisticId(1), date, Some(goal.clone()))?;

            let fetched = repo.find_goal(StatisticId(1), date)?;
            assert!(fetched.is_some());
            let fetched_value = fetched.unwrap();
            assert_eq!(fetched_value["target"], 100);
            assert_eq!(fetched_value["extra_data1"], "some_data");
            Ok(())
        }

        #[test]
        fn save_and_find_default_goal() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let goal = serde_json::json!({
                "target": 50,
                "extra_data1": "default_data"
            });

            repo.save_default_goal(StatisticId(1), goal.clone())?;

            let fetched = repo.find_default_goal(StatisticId(1))?;
            assert!(fetched.is_some());
            let fetched_value = fetched.unwrap();
            assert_eq!(fetched_value["target"], 50);
            assert_eq!(fetched_value["extra_data1"], "default_data");
            Ok(())
        }
    }

    // ==================== Error Handling Tests ====================

    mod error_handling_tests {
        use super::*;

        #[test]
        fn find_metadata_by_id_not_found() {
            let repo = setup_repository();

            let result = repo.find_metadata_by_id(StatisticId(999));
            assert!(result.is_err());
        }

        #[test]
        fn find_log_value_not_found_returns_none() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
            let value = repo.find_log_value(StatisticId(1), date)?;

            assert!(value.is_none());
            Ok(())
        }

        #[test]
        fn find_goal_not_found_returns_none() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 2).unwrap();
            let goal = repo.find_goal(StatisticId(1), date)?;

            assert!(goal.is_none());
            Ok(())
        }

        #[test]
        fn find_default_goal_not_found_returns_none() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let goal = repo.find_default_goal(StatisticId(1))?;

            assert!(goal.is_none());
            Ok(())
        }

        #[test]
        fn delete_default_goal_that_exists() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let goal = serde_json::json!({"target": 5});
            repo.save_default_goal(StatisticId(1), goal)?;

            repo.delete_default_goal(StatisticId(1))?;

            let fetched = repo.find_default_goal(StatisticId(1))?;
            assert!(fetched.is_none());
            Ok(())
        }

        #[test]
        fn upsert_log_value_updates_existing() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 3).unwrap();

            // Save initial value
            repo.save_log_value(StatisticId(1), date, serde_json::json!(10))?;

            // Save different value for the same date
            repo.save_log_value(StatisticId(1), date, serde_json::json!(20))?;

            // Fetch and verify it was updated
            let value = repo.find_log_value(StatisticId(1), date)?;
            assert_eq!(value, Some(serde_json::json!(20)));
            Ok(())
        }

        #[test]
        fn upsert_goal_updates_existing() -> Result<()> {
            let repo = setup_repository();
            repo.save_metadata(&integer_stat(1))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 4).unwrap();

            // Save initial goal
            repo.save_goal(
                StatisticId(1),
                date,
                Some(serde_json::json!({"target": 10})),
            )?;

            // Save different goal for the same date
            repo.save_goal(
                StatisticId(1),
                date,
                Some(serde_json::json!({"target": 20})),
            )?;

            // Fetch and verify it was updated
            let goal = repo.find_goal(StatisticId(1), date)?;
            assert!(goal.is_some());
            assert_eq!(goal.unwrap()["target"], 20);
            Ok(())
        }
    }
}
