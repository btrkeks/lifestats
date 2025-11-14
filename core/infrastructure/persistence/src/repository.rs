use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::NaiveDate;
use diesel::connection::Connection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_types::{BigInt, Text};
use diesel::SqliteConnection;
use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_application::shared::tags::TagSet;
use lifestats_application::statistic::ports::TagQueryableStatisticRepository;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::error::RepositoryError;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

use crate::db::mappers::{InsertStatisticDto, StatisticDto};
use crate::db::models::{DefaultGoalRaw, GoalDataRaw, LoggingDataRaw};
use crate::db::queries;
use crate::db::schema::{default_goals, goals, logging_data, statistics};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

// Helper function to convert diesel errors to RepositoryError
fn diesel_to_repository_error(err: diesel::result::Error) -> RepositoryError {
    match err {
        diesel::result::Error::NotFound => RepositoryError::NotFound,
        _ => RepositoryError::OperationFailed(err.into()),
    }
}

pub struct SqliteStatisticRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteStatisticRepository {
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

    fn load_statistic_anyhow(
        conn: &mut SqliteConnection,
        id: StatisticId,
    ) -> anyhow::Result<AnyStatistic> {
        let dto = statistics::table
            .find(id.0 as i32)
            .first::<StatisticDto>(conn)?;
        let default_goal = queries::fetch_default_goal(conn, id)?;
        dto.into_any_statistic(default_goal)
    }
}

impl StatisticRepository for SqliteStatisticRepository {
    fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic, RepositoryError> {
        let mut conn = self.connection()?;

        let dto = statistics::table
            .find(id.0 as i32)
            .first::<StatisticDto>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        let default_goal =
            queries::fetch_default_goal(&mut conn, id).map_err(RepositoryError::OperationFailed)?;
        dto.into_any_statistic(default_goal)
            .map_err(RepositoryError::OperationFailed)
    }

    fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>, RepositoryError> {
        let mut conn = self.connection()?;

        let dtos = statistics::table
            .load::<StatisticDto>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        let ids: Vec<i32> = dtos.iter().map(|dto| dto.id).collect();
        let default_goal_map = queries::fetch_default_goals(&mut conn, &ids)
            .map_err(RepositoryError::OperationFailed)?;

        let mut result = Vec::with_capacity(dtos.len());
        for dto in dtos {
            let default_goal = default_goal_map.get(&dto.id).cloned();
            result.push(
                dto.into_any_statistic(default_goal)
                    .map_err(RepositoryError::OperationFailed)?,
            );
        }

        Ok(result)
    }

    fn create_metadata(&self, statistic: AnyStatistic) -> Result<AnyStatistic, RepositoryError> {
        let mut conn = self.connection()?;

        conn.transaction::<_, anyhow::Error, _>(move |conn| {
            let insert_dto = InsertStatisticDto::from_any_statistic(&statistic)?;

            diesel::insert_into(statistics::table)
                .values(&insert_dto)
                .execute(conn)?;

            let new_id: i64 =
                diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?;
            let statistic_id = StatisticId(new_id as u32);
            SqliteStatisticRepository::load_statistic_anyhow(conn, statistic_id)
        })
        .map_err(RepositoryError::OperationFailed)
    }

    fn update_metadata(&self, statistic: AnyStatistic) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        let dto = StatisticDto::from_any_statistic(&statistic)
            .map_err(RepositoryError::OperationFailed)?;

        let StatisticDto {
            id,
            name,
            category,
            statistic_type,
            default_goal_valid_since,
            tags,
            canonical_tags,
        } = dto;

        diesel::update(statistics::table.find(id))
            .set((
                statistics::name.eq(name),
                statistics::category.eq(category),
                statistics::statistic_type.eq(statistic_type),
                statistics::default_goal_valid_since.eq(default_goal_valid_since),
                statistics::tags.eq(tags),
                statistics::canonical_tags.eq(canonical_tags),
            ))
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn create_many_metadata(
        &self,
        statistics_to_create: Vec<AnyStatistic>,
    ) -> Result<Vec<AnyStatistic>, RepositoryError> {
        let mut conn = self.connection()?;

        conn.transaction::<_, anyhow::Error, _>(move |conn| {
            let mut persisted = Vec::with_capacity(statistics_to_create.len());
            for statistic in statistics_to_create {
                let insert_dto = InsertStatisticDto::from_any_statistic(&statistic)?;

                diesel::insert_into(statistics::table)
                    .values(&insert_dto)
                    .execute(conn)?;

                let new_id: i64 =
                    diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?;
                let statistic_id = StatisticId(new_id as u32);
                let stored = SqliteStatisticRepository::load_statistic_anyhow(conn, statistic_id)?;
                persisted.push(stored);
            }

            Ok(persisted)
        })
        .map_err(RepositoryError::OperationFailed)
    }

    /// Filters statistics that contain all provided tag key/value pairs by leveraging
    /// SQLite's `json_each` table-valued function to iterate over the JSON tag map.
    fn find_by_tags(
        &self,
        tags: &HashMap<String, String>,
    ) -> Result<Vec<AnyStatistic>, RepositoryError> {
        let mut conn = self.connection()?;

        let filter_json =
            serde_json::to_string(tags).map_err(|e| RepositoryError::OperationFailed(e.into()))?;

        let query = diesel::sql_query(
            "SELECT id, name, category, statistic_type, default_goal_valid_since, tags, canonical_tags \
             FROM statistics \
             WHERE NOT EXISTS ( \
                 SELECT 1 \
                 FROM json_each(?) AS filter \
                 WHERE json_extract(statistics.tags, '$.' || filter.key) IS NULL \
                   OR json_extract(statistics.tags, '$.' || filter.key) <> filter.value \
             )",
        )
        .bind::<Text, _>(filter_json);

        let dtos: Vec<StatisticDto> = query
            .load::<StatisticDto>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        // Fetch default goals for all found statistics
        let ids: Vec<i32> = dtos.iter().map(|dto| dto.id).collect();
        let default_goal_map = queries::fetch_default_goals(&mut conn, &ids)
            .map_err(RepositoryError::OperationFailed)?;

        // Convert DTOs to domain objects
        let mut result = Vec::with_capacity(dtos.len());
        for dto in dtos {
            let default_goal = default_goal_map.get(&dto.id).cloned();
            result.push(
                dto.into_any_statistic(default_goal)
                    .map_err(RepositoryError::OperationFailed)?,
            );
        }

        Ok(result)
    }

    fn delete_default_goal(&self, id: StatisticId) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        diesel::delete(default_goals::table.find(id.0 as i32))
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;
        Ok(())
    }

    fn delete_metadata(&self, id: StatisticId) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;

        // Delete in a transaction to ensure all related data is removed
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Delete logging data for this statistic
            diesel::delete(logging_data::table.filter(logging_data::statistic_id.eq(id.0 as i32)))
                .execute(conn)?;

            // Delete goals for this statistic
            diesel::delete(goals::table.filter(goals::statistic_id.eq(id.0 as i32)))
                .execute(conn)?;

            // Delete default goal for this statistic
            diesel::delete(default_goals::table.find(id.0 as i32)).execute(conn)?;

            // Finally, delete the statistic itself
            diesel::delete(statistics::table.find(id.0 as i32)).execute(conn)?;

            Ok(())
        })
        .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn find_log_value(
        &self,
        id: StatisticId,
        date: NaiveDate,
    ) -> Result<Option<serde_json::Value>, RepositoryError> {
        let mut conn = self.connection()?;
        let row = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.eq(date.to_string()))
            .first::<LoggingDataRaw>(&mut conn)
            .optional()
            .map_err(diesel_to_repository_error)?;

        row.map(|raw| {
            serde_json::from_str(&raw.value)
                .context("failed to parse log value JSON")
                .map_err(RepositoryError::OperationFailed)
        })
        .transpose()
    }

    fn save_log_value(
        &self,
        id: StatisticId,
        date: NaiveDate,
        value: serde_json::Value,
    ) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        let serialized = serde_json::to_string(&value)
            .context(format!(
                "failed to serialize log value for statistic {} on {}",
                id.0, date
            ))
            .map_err(RepositoryError::OperationFailed)?;

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
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn find_log_values_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>, RepositoryError> {
        let mut conn = self.connection()?;
        let rows = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.ge(range.start().to_string()))
            .filter(logging_data::date.le(range.end().to_string()))
            .order(logging_data::date.asc())
            .load::<LoggingDataRaw>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        rows.into_iter()
            .map(|raw| {
                let date = raw
                    .date
                    .parse::<NaiveDate>()
                    .context(format!("failed to parse date: {}", raw.date))
                    .map_err(RepositoryError::OperationFailed)?;
                let value = serde_json::from_str(&raw.value)
                    .context("failed to parse log value JSON")
                    .map_err(RepositoryError::OperationFailed)?;
                Ok(DatedValue::new(date, value))
            })
            .collect()
    }

    fn find_goal(
        &self,
        id: StatisticId,
        date: NaiveDate,
    ) -> Result<GoalQueryResult, RepositoryError> {
        let mut conn = self.connection()?;
        let row = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.eq(date.to_string()))
            .first::<GoalDataRaw>(&mut conn)
            .optional()
            .map_err(diesel_to_repository_error)?;

        match row {
            Some(raw) => match raw.goal {
                Some(goal_str) => {
                    let goal = serde_json::from_str(&goal_str)
                        .context(format!("failed to parse goal JSON: {}", goal_str))
                        .map_err(RepositoryError::OperationFailed)?;
                    Ok(GoalQueryResult::Goal(goal))
                }
                None => Ok(GoalQueryResult::Suppressed),
            },
            None => Ok(GoalQueryResult::NotSet),
        }
    }

    fn save_goal(
        &self,
        id: StatisticId,
        date: NaiveDate,
        goal: Option<serde_json::Value>,
    ) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;

        // Build the record; goal: Some(..) = override, goal: None = suppressed
        let record = match goal {
            Some(goal_value) => {
                let goal_str = serde_json::to_string(&goal_value)
                    .context("failed to serialize goal to JSON")
                    .map_err(RepositoryError::OperationFailed)?;

                GoalDataRaw {
                    statistic_id: id.0 as i32,
                    date: date.to_string(),
                    goal: Some(goal_str),
                }
            }
            None => GoalDataRaw {
                statistic_id: id.0 as i32,
                date: date.to_string(),
                goal: None, // stored as NULL => "suppressed"
            },
        };

        diesel::insert_into(goals::table)
            .values(&record)
            .on_conflict((goals::statistic_id, goals::date))
            .do_update()
            .set(&record) // will set goal to Some(...) or NULL accordingly
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn find_goals_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>, RepositoryError> {
        let mut conn = self.connection()?;
        let rows = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.ge(range.start().to_string()))
            .filter(goals::date.le(range.end().to_string()))
            .order(goals::date.asc())
            .load::<GoalDataRaw>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        let mut result = Vec::new();
        for row in rows {
            if let Some(goal_str) = row.goal {
                let date = row
                    .date
                    .parse::<NaiveDate>()
                    .context(format!("failed to parse date: {}", row.date))
                    .map_err(RepositoryError::OperationFailed)?;
                let goal = serde_json::from_str(&goal_str)
                    .context(format!("failed to parse goal JSON: {}", goal_str))
                    .map_err(RepositoryError::OperationFailed)?;
                result.push(DatedValue::new(date, goal));
            }
        }
        Ok(result)
    }

    fn find_default_goal(
        &self,
        id: StatisticId,
    ) -> Result<Option<serde_json::Value>, RepositoryError> {
        let mut conn = self.connection()?;
        let row =
            queries::fetch_default_goal(&mut conn, id).map_err(RepositoryError::OperationFailed)?;

        match row {
            Some(raw) => {
                let goal = serde_json::from_str(&raw.goal)
                    .context(format!(
                        "Failed to parse default goal '{}' for statistic_id {}",
                        raw.goal, raw.statistic_id
                    ))
                    .map_err(RepositoryError::OperationFailed)?;
                Ok(Some(goal))
            }
            None => Ok(None),
        }
    }

    fn save_default_goal(
        &self,
        id: StatisticId,
        goal: serde_json::Value,
    ) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        let serialized_goal =
            serde_json::to_string(&goal).map_err(|e| RepositoryError::OperationFailed(e.into()))?;

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
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn delete_goal(&self, id: StatisticId, date: NaiveDate) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        diesel::delete(
            goals::table
                .filter(goals::statistic_id.eq(id.0 as i32))
                .filter(goals::date.eq(date.to_string())),
        )
        .execute(&mut conn)
        .map_err(diesel_to_repository_error)?;
        Ok(())
    }
}

#[async_trait]
impl TagQueryableStatisticRepository for SqliteStatisticRepository {
    async fn find_statistic_id_by_exact_tags(
        &self,
        tags: &TagSet,
    ) -> ApplicationResult<Option<StatisticId>> {
        let canonical = Some(tags.canonical_fingerprint());

        let mut conn = self.connection().map_err(ApplicationError::from)?;

        let ids: Vec<i32> = statistics::table
            .filter(statistics::canonical_tags.eq(canonical))
            .select(statistics::id)
            .load(&mut conn)
            .map_err(|err| ApplicationError::Infrastructure(err.into()))?;

        match ids.len() {
            0 => Ok(None),
            1 => Ok(Some(StatisticId(ids[0] as u32))),
            _ => Err(ApplicationError::Infrastructure(anyhow!(
                "Multiple statistics share canonical tags; expected unique fingerprint"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use anyhow::Result;
    use chrono::NaiveDate;
    use diesel_migrations::MigrationHarness;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use tokio::runtime::Runtime;

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
            "Fitness".to_string(),
            "Push Ups".to_string(),
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
            "Health".to_string(),
            "Brushed Teeth".to_string(),
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
            "Productivity".to_string(),
            "Study Time".to_string(),
        );
        AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date"),
        ))
    }

    fn persist_stat(
        repo: &SqliteStatisticRepository,
        statistic: AnyStatistic,
    ) -> Result<StatisticId> {
        Ok(repo.create_metadata(statistic)?.get_metadata().id())
    }

    fn integer_stat_with_tags(tags: &[(&str, &str)]) -> AnyStatistic {
        let mut metadata = StatisticMetadata::new(
            StatisticId(0),
            "Fitness".to_string(),
            "Push Ups".to_string(),
        );

        for (key, value) in tags {
            metadata = metadata.with_tag((*key).to_string(), (*value).to_string());
        }

        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
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
            let created = repo.create_metadata(integer_stat(0))?;
            let fetched = repo.find_metadata_by_id(created.get_metadata().id())?;
            assert!(matches!(fetched, AnyStatistic::IntegerAmount(_)));
            assert_eq!(fetched.get_metadata().name(), "Push Ups");
            Ok(())
        }

        #[test]
        fn fetch_all_metadata() -> Result<()> {
            let repo = setup_repository();

            repo.create_metadata(integer_stat(0))?;
            repo.create_metadata(habit_stat(0))?;
            repo.create_metadata(time_spans_stat(0))?;

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
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
            let value = serde_json::json!(123);

            repo.save_log_value(statistic_id, date, value.clone())?;

            let fetched = repo.find_log_value(statistic_id, date)?;
            assert_eq!(fetched, Some(value));
            Ok(())
        }

        #[test]
        fn save_and_find_goal() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 3, 2).unwrap();
            let goal = serde_json::json!({
                "target": 100,
                "extra_data1": "some_data"
            });

            repo.save_goal(statistic_id, date, Some(goal.clone()))?;

            let fetched = repo.find_goal(statistic_id, date)?;
            assert!(fetched.is_set());
            let fetched_value = fetched.goal_data().unwrap();
            assert_eq!(fetched_value["target"], 100);
            assert_eq!(fetched_value["extra_data1"], "some_data");
            Ok(())
        }

        #[test]
        fn save_and_find_default_goal() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let goal = serde_json::json!({
                "target": 50,
                "extra_data1": "default_data"
            });

            repo.save_default_goal(statistic_id, goal.clone())?;

            let fetched = repo.find_default_goal(statistic_id)?;
            assert!(fetched.is_some());
            let fetched_value = fetched.unwrap();
            assert_eq!(fetched_value["target"], 50);
            assert_eq!(fetched_value["extra_data1"], "default_data");
            Ok(())
        }
    }

    // ==================== Tag Filtering Tests ====================

    mod tag_filter_tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn find_by_tags_returns_matching_statistics() -> Result<()> {
            let repo = setup_repository();

            persist_stat(
                &repo,
                integer_stat_with_tags(&[
                    ("source", "anki"),
                    ("deck", "spanish"),
                    ("metric", "time_spent"),
                ]),
            )?;

            persist_stat(
                &repo,
                integer_stat_with_tags(&[
                    ("source", "anki"),
                    ("deck", "french"),
                    ("metric", "time_spent"),
                ]),
            )?;

            let mut filter = HashMap::new();
            filter.insert("source".to_string(), "anki".to_string());
            filter.insert("deck".to_string(), "spanish".to_string());

            let results = repo.find_by_tags(&filter)?;
            assert_eq!(results.len(), 1);
            let result_metadata = results[0].get_metadata();
            assert_eq!(
                result_metadata
                    .tags()
                    .get("deck")
                    .map(|value| value.as_str()),
                Some("spanish")
            );

            Ok(())
        }

        #[test]
        fn find_by_tags_returns_empty_when_any_tag_differs() -> Result<()> {
            let repo = setup_repository();

            persist_stat(
                &repo,
                integer_stat_with_tags(&[("source", "anki"), ("metric", "time_spent")]),
            )?;

            let mut filter = HashMap::new();
            filter.insert("source".to_string(), "anki".to_string());
            filter.insert("deck".to_string(), "spanish".to_string());

            let results = repo.find_by_tags(&filter)?;
            assert!(results.is_empty());
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
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap();
            let value = repo.find_log_value(statistic_id, date)?;

            assert!(value.is_none());
            Ok(())
        }

        #[test]
        fn find_goal_not_found_returns_none() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 2).unwrap();
            let goal = repo.find_goal(statistic_id, date)?;

            assert!(matches!(goal, GoalQueryResult::NotSet));
            Ok(())
        }

        #[test]
        fn find_default_goal_not_found_returns_none() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let goal = repo.find_default_goal(statistic_id)?;

            assert!(goal.is_none());
            Ok(())
        }

        #[test]
        fn delete_default_goal_that_exists() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let goal = serde_json::json!({"target": 5});
            repo.save_default_goal(statistic_id, goal)?;

            repo.delete_default_goal(statistic_id)?;

            let fetched = repo.find_default_goal(statistic_id)?;
            assert!(fetched.is_none());
            Ok(())
        }

        #[test]
        fn upsert_log_value_updates_existing() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 3).unwrap();

            // Save initial value
            repo.save_log_value(statistic_id, date, serde_json::json!(10))?;

            // Save different value for the same date
            repo.save_log_value(statistic_id, date, serde_json::json!(20))?;

            // Fetch and verify it was updated
            let value = repo.find_log_value(statistic_id, date)?;
            assert_eq!(value, Some(serde_json::json!(20)));
            Ok(())
        }

        #[test]
        fn upsert_goal_updates_existing() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 4).unwrap();

            // Save initial goal
            repo.save_goal(statistic_id, date, Some(serde_json::json!({"target": 10})))?;

            // Save different goal for the same date
            repo.save_goal(statistic_id, date, Some(serde_json::json!({"target": 20})))?;

            // Fetch and verify it was updated
            let goal = repo.find_goal(statistic_id, date)?;
            assert!(goal.is_set());
            assert_eq!(goal.goal_data().unwrap()["target"], 20);
            Ok(())
        }

        #[test]
        fn delete_goal_removes_existing_goal() -> Result<()> {
            let repo = setup_repository();
            let statistic_id = persist_stat(&repo, integer_stat(0))?;

            let date = NaiveDate::from_ymd_opt(2024, 4, 5).unwrap();

            // Save a goal
            repo.save_goal(statistic_id, date, Some(serde_json::json!({"target": 15})))?;

            // Verify it exists
            let goal = repo.find_goal(statistic_id, date)?;
            assert!(goal.is_set());

            // Delete the goal
            repo.delete_goal(statistic_id, date)?;

            // Verify it's deleted
            let fetched = repo.find_goal(statistic_id, date)?;
            assert!(matches!(fetched, GoalQueryResult::NotSet));
            Ok(())
        }
    }

    mod canonical_tag_tests {
        use super::*;
        use lifestats_application::shared::tags::TagSet;

        #[test]
        fn find_statistic_by_exact_tags() -> Result<()> {
            let repo = setup_repository();
            let statistic = integer_stat_with_tags(&[("source", "fitbit"), ("metric", "steps")]);
            let stored_id = persist_stat(&repo, statistic)?;

            let tag_set = TagSet::new([("source", "fitbit"), ("metric", "steps")]);

            let runtime = Runtime::new().expect("tokio runtime for test");
            let found = runtime
                .block_on(repo.find_statistic_id_by_exact_tags(&tag_set))
                .expect("lookup should succeed");

            assert_eq!(found, Some(stored_id));
            Ok(())
        }

        #[test]
        fn returns_none_when_tags_do_not_match() -> Result<()> {
            let repo = setup_repository();
            let statistic = integer_stat_with_tags(&[("source", "fitbit"), ("metric", "sleep")]);
            persist_stat(&repo, statistic)?;

            let tag_set = TagSet::new([("source", "fitbit"), ("metric", "steps")]);

            let runtime = Runtime::new().expect("tokio runtime for test");
            let found = runtime
                .block_on(repo.find_statistic_id_by_exact_tags(&tag_set))
                .expect("lookup should succeed");

            assert!(found.is_none());
            Ok(())
        }
    }
}
