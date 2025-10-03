use crate::sqlite_database::models::LoggingDataRaw;
use crate::sqlite_database::schema::logging_data;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use lifestats_domain::shared::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::ports::logging_data_repository::LoggingDataRepositoryTrait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

pub struct LoggingDataRepository<T> {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> LoggingDataRepository<T> {
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self {
            pool,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> LoggingDataRepositoryTrait for LoggingDataRepository<T>
where
    T: Send + Sync + DeserializeOwned + Serialize,
{
    type StoredType = T;

    fn get_in_closed_range(
        &self,
        id: StatisticId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DatedValue<Self::StoredType>>> {
        let mut conn = self.pool.get()?;

        let data = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.between(start_date.to_string(), end_date.to_string()))
            .order(logging_data::date.asc())
            .load::<LoggingDataRaw>(&mut conn)
            .with_context(|| {
                format!(
                    "Failed to get data for statistic_id: {} in range {} - {}",
                    id.0, start_date, end_date
                )
            })?;

        Ok(data
            .into_iter()
            .map(|raw_data| raw_data.into_value_logging_data())
            .collect())
    }

    fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Self::StoredType>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let data: Option<LoggingDataRaw> = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .filter(logging_data::date.eq(date.to_string()))
            .first(&mut conn)
            .optional() // This converts a NotFound error into Ok(None)
            .with_context(|| {
                format!(
                    "Failed to get data for statistic_id: {}, date: {}",
                    id.0, date
                )
            })?;

        // TODO: Value should be NON NULL in database
        Ok(data.map(|d| d.into_value_logging_data().value))
    }

    fn insert_or_update(
        &self,
        id: StatisticId,
        date: NaiveDate,
        new_value: Self::StoredType,
    ) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let serialized_value =
            serde_json::to_string(&new_value).context("Failed to serialize data value")?;

        let new_data = LoggingDataRaw {
            statistic_id: id.0 as i32,
            date: date.to_string(),
            value: serialized_value,
        };

        diesel::insert_into(logging_data::table)
            .values(&new_data)
            .on_conflict((logging_data::statistic_id, logging_data::date))
            .do_update()
            .set(&new_data)
            .execute(&mut conn)
            .with_context(|| {
                format!(
                    "Failed to insert or update data for statistic_id: {}, date: {}",
                    id.0, date
                )
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::NaiveTime;
    use diesel_migrations::MigrationHarness;
    use lifestats_domain::shared::seconds::MyDuration;
    use lifestats_domain::shared::time_frame_list::{TimeFrame, TimeFrameList};

    fn create_test_pool() -> Arc<Pool<ConnectionManager<SqliteConnection>>> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Arc::new(
            Pool::builder()
                .max_size(1) // Ensure single connection for tests
                .build(manager)
                .expect("Failed to create connection pool"),
        );

        // Run migrations on the test database
        pool.get()
            .expect("Failed to get connection")
            .run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        pool
    }

    #[test]
    fn test_insert_and_get_logging_data() -> Result<()> {
        let pool = create_test_pool();
        let repo = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let value = 123;

        repo.insert_or_update(stat_id, date, Some(value))?;

        let retrieved_data = repo.get(stat_id, date)?.unwrap().unwrap();
        assert_eq!(retrieved_data, value);

        Ok(())
    }

    #[test]
    fn test_get_nonexistent_logging_data() -> Result<()> {
        let pool = create_test_pool();
        let repo: LoggingDataRepository<i64> = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(999); // Assuming this ID doesn't exist
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        let retrieved_data = repo.get(stat_id, date)?;

        assert!(retrieved_data.is_none());

        Ok(())
    }

    #[test]
    fn test_update_logging_data() -> Result<()> {
        let pool = create_test_pool();
        let repo = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(2);
        let date = NaiveDate::from_ymd_opt(2023, 1, 2).unwrap();
        let initial_value = 456;
        let updated_value = 789;

        repo.insert_or_update(stat_id, date, Some(initial_value))?;
        let first_retrieved_value = repo.get(stat_id, date)?.unwrap().unwrap(); // Ensure the data is there
        repo.insert_or_update(stat_id, date, Some(updated_value))?;
        let second_retrieved_value = repo.get(stat_id, date)?.unwrap().unwrap();

        assert_eq!(first_retrieved_value, initial_value);
        assert_eq!(second_retrieved_value, updated_value);

        Ok(())
    }

    #[test]
    fn test_get_logging_data_in_range() -> Result<()> {
        let pool = create_test_pool();
        let repo = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(3);
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 1, 5).unwrap();

        // Insert some data
        for i in 0..5 {
            let date = start_date + chrono::Duration::days(i);
            let value = i * 10;
            repo.insert_or_update(stat_id, date, Some(value))?;
        }

        let retrieved_data = repo.get_in_closed_range(stat_id, start_date, end_date)?;
        assert_eq!(retrieved_data.len(), 5);

        for (i, data) in retrieved_data.iter().enumerate() {
            assert_eq!(data.value, Some((i as i64) * 10));
        }

        Ok(())
    }

    #[test]
    fn test_insert_and_get_duration() -> Result<()> {
        let pool = create_test_pool();
        let repo = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(4);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let value = MyDuration::from_mins(30);

        repo.insert_or_update(stat_id, date, Some(value))?;

        let retrieved_value = repo.get(stat_id, date)?.unwrap().unwrap();
        assert_eq!(retrieved_value, value);

        Ok(())
    }

    #[test]
    fn test_insert_and_get_time_frame_list() -> Result<()> {
        let pool = create_test_pool();
        let repo = LoggingDataRepository::new(pool);

        let stat_id = StatisticId(5);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let start_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();

        let time_frame = TimeFrame::new(start_time, end_time).unwrap();
        let mut value = TimeFrameList::default();
        value.append_frame(time_frame)?;

        repo.insert_or_update(stat_id, date, Some(value.clone()))?;

        let retrieved_value = repo.get(stat_id, date)?.unwrap().unwrap();
        assert_eq!(retrieved_value, value);

        Ok(())
    }
}
