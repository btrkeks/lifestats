use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use lifestats_domain::shared::date_range::DateRange;
use lifestats_domain::shared::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::any_statistic::AnyStatistic;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

use crate::hydration::statistic_dto::StatisticDto;
use crate::sqlite_database::models::{DefaultGoalRaw, GoalDataRaw, LoggingDataRaw};
use crate::sqlite_database::schema::{default_goals, goals, logging_data, statistics};

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

    fn fetch_default_goal(
        conn: &mut SqliteConnection,
        statistic_id: StatisticId,
    ) -> Result<Option<DefaultGoalRaw>> {
        default_goals::table
            .find(statistic_id.0 as i32)
            .first::<DefaultGoalRaw>(conn)
            .optional()
            .context("failed to fetch default goal")
    }

    fn fetch_default_goals(
        conn: &mut SqliteConnection,
        statistic_ids: &[i32],
    ) -> Result<HashMap<i32, DefaultGoalRaw>> {
        if statistic_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = default_goals::table
            .filter(default_goals::statistic_id.eq_any(statistic_ids))
            .load::<DefaultGoalRaw>(conn)
            .context("failed to fetch default goals batch")?;

        Ok(rows
            .into_iter()
            .map(|raw| (raw.statistic_id, raw))
            .collect())
    }
}

impl StatisticRepository for SqliteStatisticRepository {
    fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic> {
        let mut conn = self.connection()?;

        let dto = statistics::table
            .find(id.0 as i32)
            .first::<StatisticDto>(&mut conn)
            .with_context(|| format!("statistic {} not found", id.0))?;

        let default_goal = Self::fetch_default_goal(&mut conn, id)?;
        dto.into_any_statistic(default_goal)
    }

    fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>> {
        let mut conn = self.connection()?;

        let dtos = statistics::table
            .load::<StatisticDto>(&mut conn)
            .context("failed to load statistics")?;

        let ids: Vec<i32> = dtos.iter().map(|dto| dto.id).collect();
        let default_goal_map = Self::fetch_default_goals(&mut conn, &ids)?;

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

        if statistic.get_default_goal().is_some() {
            let start_date = statistic.get_default_goal_valid_since().to_string();
            diesel::update(default_goals::table.filter(default_goals::statistic_id.eq(dto.id)))
                .set(default_goals::extra_data1.eq(Some(start_date)))
                .execute(&mut conn)
                .context("failed to update default goal start date")?;
        }

        Ok(())
    }

    fn find_log_value<V>(&self, id: StatisticId, date: NaiveDate) -> Result<Option<V>>
    where
        V: serde::de::DeserializeOwned,
    {
        let mut conn = self.connection()?;
        let row = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.eq(date.to_string()))
            .first::<LoggingDataRaw>(&mut conn)
            .optional()
            .context("failed to fetch log value")?;

        row.map(|raw| raw.into_dated_value::<V>().map(|dated| dated.value))
            .transpose()
    }

    fn find_log_values_in_range<V>(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<V>>>
    where
        V: serde::de::DeserializeOwned,
    {
        let mut conn = self.connection()?;
        let rows = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.ge(range.start().to_string()))
            .filter(logging_data::date.le(range.end().to_string()))
            .order(logging_data::date.asc())
            .load::<LoggingDataRaw>(&mut conn)
            .context("failed to fetch logging data range")?;

        rows.into_iter()
            .map(|raw| raw.into_dated_value::<V>())
            .collect()
    }

    fn save_log_value<V>(&self, id: StatisticId, date: NaiveDate, value: V) -> Result<()>
    where
        V: serde::Serialize,
    {
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

    fn find_goal<G>(&self, id: StatisticId, date: NaiveDate) -> Result<Option<G>>
    where
        G: serde::de::DeserializeOwned,
    {
        let mut conn = self.connection()?;
        let row = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.eq(date.to_string()))
            .first::<GoalDataRaw>(&mut conn)
            .optional()
            .context("failed to fetch goal")?;

        match row {
            Some(raw) => raw.into_optional_goal::<G>(),
            None => Ok(None),
        }
    }

    fn find_goals_in_range<G>(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<G>>>
    where
        G: serde::de::DeserializeOwned,
    {
        let mut conn = self.connection()?;
        let rows = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.ge(range.start().to_string()))
            .filter(goals::date.le(range.end().to_string()))
            .order(goals::date.asc())
            .load::<GoalDataRaw>(&mut conn)
            .context("failed to fetch goals range")?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let dated = row.into_dated_goal::<G>()?;
            if let Some(goal) = dated.value {
                result.push(DatedValue::new(dated.date, goal));
            }
        }

        Ok(result)
    }

    fn save_goal<G>(&self, id: StatisticId, date: NaiveDate, goal: Option<G>) -> Result<()>
    where
        G: serde::Serialize,
    {
        let mut conn = self.connection()?;

        if let Some(goal) = goal {
            let record = GoalDataRaw::from_goal(id, date, Some(goal))?;
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

    fn find_default_goal<G>(&self, id: StatisticId) -> Result<Option<G>>
    where
        G: serde::de::DeserializeOwned,
    {
        let mut conn = self.connection()?;
        let row = Self::fetch_default_goal(&mut conn, id)?;
        row.map(|raw| raw.into_goal::<G>()).transpose()
    }

    fn save_default_goal<G>(&self, id: StatisticId, goal: G) -> Result<()>
    where
        G: serde::Serialize,
    {
        let mut conn = self.connection()?;
        let record = DefaultGoalRaw::from_goal(id, goal, default_goal_start_fallback())?;

        let target_value = record.target_value.clone();
        let extra_data1 = record.extra_data1.clone();

        diesel::insert_into(default_goals::table)
            .values(&record)
            .on_conflict(default_goals::statistic_id)
            .do_update()
            .set((
                default_goals::target_value.eq(target_value),
                default_goals::extra_data1.eq(extra_data1),
            ))
            .execute(&mut conn)
            .context("failed to upsert default goal")?;

        Ok(())
    }

    fn delete_default_goal(&self, id: StatisticId) -> Result<()> {
        let mut conn = self.connection()?;
        diesel::delete(default_goals::table.find(id.0 as i32))
            .execute(&mut conn)
            .context("failed to delete default goal")?;
        Ok(())
    }

    // --- JSON-based Operations (Polymorphic Interface) ---
    fn find_log_value_json(
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
            .context("failed to fetch log value as JSON")?;

        row.map(|raw| serde_json::from_str(&raw.value).context("failed to parse log value JSON"))
            .transpose()
    }

    fn save_log_value_json(
        &self,
        id: StatisticId,
        date: NaiveDate,
        value: serde_json::Value,
    ) -> Result<()> {
        let mut conn = self.connection()?;
        let serialized = serde_json::to_string(&value).with_context(|| {
            format!(
                "failed to serialize JSON log value for statistic {} on {}",
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
            .context("failed to upsert JSON log value")?;

        Ok(())
    }

    fn find_log_values_in_range_json(
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
            .context("failed to fetch JSON logging data range")?;

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

    fn find_goal_json(
        &self,
        id: StatisticId,
        date: NaiveDate,
    ) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection()?;
        let row = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.eq(date.to_string()))
            .first::<GoalDataRaw>(&mut conn)
            .optional()
            .context("failed to fetch goal as JSON")?;

        match row {
            Some(raw) => {
                if let Some(target_value) = raw.target_value {
                    let mut map = serde_json::Map::new();
                    map.insert("target".to_string(), serde_json::from_str(&target_value)?);
                    if let Some(extra) = raw.extra_data1 {
                        map.insert("extra_data1".to_string(), serde_json::Value::String(extra));
                    }
                    Ok(Some(serde_json::Value::Object(map)))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    fn save_goal_json(
        &self,
        id: StatisticId,
        date: NaiveDate,
        goal: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut conn = self.connection()?;

        if let Some(goal_value) = goal {
            let (target_value, extra_data1) = if let serde_json::Value::Object(map) = &goal_value {
                let target = map
                    .get("target")
                    .ok_or_else(|| anyhow::anyhow!("goal JSON missing 'target' field"))?;
                let target_str = serde_json::to_string(target)?;
                let extra = map
                    .get("extra_data1")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                (target_str, extra)
            } else {
                // If it's not an object, serialize the whole thing as target
                (serde_json::to_string(&goal_value)?, None)
            };

            let record = GoalDataRaw {
                statistic_id: id.0 as i32,
                date: date.to_string(),
                target_value: Some(target_value),
                extra_data1,
            };

            diesel::insert_into(goals::table)
                .values(&record)
                .on_conflict((goals::statistic_id, goals::date))
                .do_update()
                .set(&record)
                .execute(&mut conn)
                .context("failed to upsert JSON goal")?;
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

    fn find_goals_in_range_json(
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
            .context("failed to fetch JSON goals range")?;

        let mut result = Vec::new();
        for row in rows {
            if let Some(target_value) = row.target_value {
                let date = row
                    .date
                    .parse::<NaiveDate>()
                    .with_context(|| format!("failed to parse date: {}", row.date))?;
                let mut map = serde_json::Map::new();
                map.insert("target".to_string(), serde_json::from_str(&target_value)?);
                if let Some(extra) = row.extra_data1 {
                    map.insert("extra_data1".to_string(), serde_json::Value::String(extra));
                }
                result.push(DatedValue::new(date, serde_json::Value::Object(map)));
            }
        }
        Ok(result)
    }

    fn find_default_goal_json(&self, id: StatisticId) -> Result<Option<serde_json::Value>> {
        let mut conn = self.connection()?;
        let row = Self::fetch_default_goal(&mut conn, id)?;

        match row {
            Some(raw) => {
                let mut map = serde_json::Map::new();
                map.insert(
                    "target".to_string(),
                    serde_json::from_str(&raw.target_value)?,
                );
                if let Some(extra) = raw.extra_data1 {
                    map.insert("extra_data1".to_string(), serde_json::Value::String(extra));
                }
                Ok(Some(serde_json::Value::Object(map)))
            }
            None => Ok(None),
        }
    }

    fn save_default_goal_json(&self, id: StatisticId, goal: serde_json::Value) -> Result<()> {
        let mut conn = self.connection()?;

        let (target_value, extra_data1) = if let serde_json::Value::Object(map) = &goal {
            let target = map
                .get("target")
                .ok_or_else(|| anyhow::anyhow!("goal JSON missing 'target' field"))?;
            let target_str = serde_json::to_string(target)?;
            let extra = map
                .get("extra_data1")
                .and_then(|v| v.as_str())
                .map(String::from);
            (target_str, extra)
        } else {
            // If it's not an object, serialize the whole thing as target
            (serde_json::to_string(&goal)?, None)
        };

        let record = DefaultGoalRaw {
            statistic_id: id.0 as i32,
            target_value,
            extra_data1,
        };

        diesel::insert_into(default_goals::table)
            .values(&record)
            .on_conflict(default_goals::statistic_id)
            .do_update()
            .set((
                default_goals::target_value.eq(&record.target_value),
                default_goals::extra_data1.eq(&record.extra_data1),
            ))
            .execute(&mut conn)
            .context("failed to upsert JSON default goal")?;

        Ok(())
    }
}

fn default_goal_start_fallback() -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::NaiveDate;
    use diesel_migrations::MigrationHarness;
    use lifestats_domain::shared::date_range::DateRange;
    use lifestats_domain::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_domain::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;

    fn setup_pool() -> Arc<SqlitePool> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("failed to create pool");
        {
            let mut conn = pool.get().expect("failed to get connection for migrations");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");
        }
        Arc::new(pool)
    }

    fn integer_stat() -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(1),
            "Fitness".to_string().into_boxed_str(),
            "Push Ups".to_string().into_boxed_str(),
        );
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata,
            None,
            default_goal_start_fallback(),
        ))
    }

    #[test]
    fn save_and_fetch_metadata() -> Result<()> {
        let repo = SqliteStatisticRepository::new(setup_pool());
        let stat = integer_stat();

        repo.save_metadata(&stat)?;

        let fetched = repo.find_metadata_by_id(StatisticId(1))?;
        assert!(matches!(fetched, AnyStatistic::IntegerAmount(_)));
        assert_eq!(fetched.get_metadata().name(), "Push Ups");
        Ok(())
    }

    #[test]
    fn log_roundtrip() -> Result<()> {
        let repo = SqliteStatisticRepository::new(setup_pool());
        repo.save_metadata(&integer_stat())?;

        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        repo.save_log_value(StatisticId(1), date, 42_i64)?;

        let value: Option<i64> = repo.find_log_value(StatisticId(1), date)?;
        assert_eq!(value, Some(42));

        let range = DateRange::new(date, date)?;
        let values: Vec<DatedValue<i64>> = repo.find_log_values_in_range(StatisticId(1), range)?;
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].value, 42);
        Ok(())
    }

    #[test]
    fn goal_roundtrip() -> Result<()> {
        let repo = SqliteStatisticRepository::new(setup_pool());
        repo.save_metadata(&integer_stat())?;

        let date = NaiveDate::from_ymd_opt(2024, 2, 2).unwrap();
        let goal = AmountGoal::new(10_i64, true);
        repo.save_goal(StatisticId(1), date, Some(goal.clone()))?;

        let fetched: Option<AmountGoal<i64>> = repo.find_goal(StatisticId(1), date)?;
        assert!(fetched.is_some());

        repo.save_goal::<AmountGoal<i64>>(StatisticId(1), date, None)?;
        let fetched_after: Option<AmountGoal<i64>> = repo.find_goal(StatisticId(1), date)?;
        assert!(fetched_after.is_none());
        Ok(())
    }

    #[test]
    fn default_goal_roundtrip() -> Result<()> {
        let repo = SqliteStatisticRepository::new(setup_pool());
        repo.save_metadata(&integer_stat())?;

        let goal = AmountGoal::new(5_i64, true);
        repo.save_default_goal(StatisticId(1), goal.clone())?;

        let fetched: Option<AmountGoal<i64>> = repo.find_default_goal(StatisticId(1))?;
        assert!(fetched.is_some());

        repo.delete_default_goal(StatisticId(1))?;
        let fetched_after: Option<AmountGoal<i64>> = repo.find_default_goal(StatisticId(1))?;
        assert!(fetched_after.is_none());
        Ok(())
    }
}
