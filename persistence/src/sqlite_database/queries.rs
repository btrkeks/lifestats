// use std::fmt::Debug;
// use diesel::prelude::*;
// use chrono::{NaiveDate};
// use anyhow::{Result, Context};
// use serde::de::DeserializeOwned;
// use serde::Serialize;
// use domain_models::statistic::objects::goal::Goal;
// use transfer_objects::statistic_id::StatisticId;
// use crate::sqlite_database::models::{LoggingData, LoggingDataRaw};
// use crate::sqlite_database::schema::logging_data::date;
// use crate::sqlite_database::schema::logging_data::dsl::logging_data;
// use crate::sqlite_database::schema::plots::statistic_id;
//
// /// Core query operations for statistics data
// pub struct StatisticQueries;
//
// impl StatisticQueries {
//     /// Get a single row for a specific date
//     pub fn get_row<T, G>(
//         conn: &mut SqliteConnection,
//         stat_id_param: StatisticId,
//         row_date: NaiveDate,
//     ) -> Result<Option<LoggingDataRaw>>
//     where
//         T: Serialize + DeserializeOwned + Debug,
//         G: Goal<T> + Serialize + DeserializeOwned + Debug,
//     {
//         let data_raw = logging_data
//             .filter(statistic_id.eq(stat_id_param.0 as i32))
//             .filter(date.eq(row_date.to_string()))
//             .first(conn)
//             .optional()
//             .context("Failed to fetch logging data row")?;
//
//         data_raw.map(|raw| LoggingData::from_raw(raw)).transpose()
//     }
//
//     /// Get all rows within a date range (inclusive)
//     /// If there is no entry for a given date, an empty row will be included
//     pub fn get_rows_in_range<T, G>(
//         conn: &mut SqliteConnection,
//         stat_id: StatisticId,
//         start_date: NaiveDate,
//         end_date: NaiveDate,
//     ) -> Result<Vec<LoggingData<T, G>>>
//     where
//         T: Serialize + DeserializeOwned + Debug,
//         G: Goal<T> + Serialize + DeserializeOwned + Debug,
//     {
//         let raw_data = logging_data
//             .filter(statistic_id.eq(stat_id.0 as i32))
//             .filter(date.ge(start_date.to_string()))
//             .filter(date.le(end_date.to_string()))
//             .order_by(date.asc())
//             .load(conn)
//             .context("Failed to fetch logging data range")?;
//
//         raw_data.into_iter()
//             .map(LoggingData::from_raw)
//             .collect()
//     }
//
//     /// Insert or update a row
//     pub fn insert_or_update_row<T, G>(
//         conn: &mut SqliteConnection,
//         row: LoggingData<T, G>,
//     ) -> Result<()>
//     where
//         T: Serialize + DeserializeOwned + Debug,
//         G: Goal<T> + Serialize + DeserializeOwned + Debug,
//     {
//         let stat_id = row.statistic_id;
//
//         let raw_data = row.into_raw()?;
//
//         diesel::insert_into(logging_data)
//             .values(&raw_data)
//             .on_conflict((statistic_id, date))
//             .do_update()
//             .set(&raw_data)
//             .execute(conn)
//             .context("Failed to insert or update logging data")?;
//
//         publish_db_event(Event::ValueUpdated(stat_id));
//         Ok(())
//     }
//
//     pub fn insert_or_update_value<T, G>(
//         conn: &mut SqliteConnection,
//         stat_id: StatisticId,
//         row_date: NaiveDate,
//         new_value: T,
//     ) -> Result<()>
//     where
//         T: Serialize + DeserializeOwned + Debug,
//         G: Goal<T> + Serialize + DeserializeOwned + Debug,
//     {
//         // Get existing row to preserve goal
//         let mut row = Self::get_row::<T, G>(conn, stat_id, row_date)?
//             .unwrap_or_else(|| LoggingData::new(stat_id, row_date, None, None));
//
//         // Update only the value
//         row.value = Some(new_value);
//
//         let raw_data = row.into_raw()?;
//
//         diesel::insert_into(logging_data)
//             .values(&raw_data)
//             .on_conflict((statistic_id, date))
//             .do_update()
//             .set(&raw_data)
//             .execute(conn)
//             .context("Failed to insert or update logging data value")?;
//
//         publish_db_event(Event::ValueUpdated(stat_id));
//         Ok(())
//     }
//
//     /// Insert or update just the goal for a specific statistic and date
//     pub fn insert_or_update_goal<T, G>(
//         conn: &mut SqliteConnection,
//         stat_id: StatisticId,
//         row_date: NaiveDate,
//         new_goal: Option<G>,
//     ) -> Result<()>
//     where
//         T: Serialize + DeserializeOwned + Debug,
//         G: Goal<T> + Serialize + DeserializeOwned + Debug,
//     {
//         // Get existing row to preserve value
//         let mut row = Self::get_row::<T, G>(conn, stat_id, row_date)?
//             .unwrap_or_else(|| LoggingData::new(stat_id, row_date, None, None));
//
//         // Update only the goal
//         row.goal = new_goal;
//
//         let raw_data = row.into_raw()?;
//
//         diesel::insert_into(logging_data)
//             .values(&raw_data)
//             .on_conflict((statistic_id, date))
//             .do_update()
//             .set(&raw_data)
//             .execute(conn)
//             .context("Failed to insert or update logging data goal")?;
//
//         publish_db_event(Event::GoalUpdated(stat_id));
//         Ok(())
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::connection::tests::establish_test_connection;
//     use crate::models::LoggingData;
//     use crate::statistics::type_definitions::integer_amount::IntegerAmountGoal;
//
//     #[test]
//     fn test_basic_crud_operations() -> Result<()> {
//         let mut conn = establish_test_connection()?;
//         let stat_id = StatisticId(1);
//         let test_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
//         let test_value: i64 = 51235;
//         let amount_goal: IntegerAmountGoal = IntegerAmountGoal::new(50000, true);
//
//         let test_row = LoggingData::new(
//             stat_id,
//             test_date,
//             Some(test_value),
//             Some(amount_goal),
//         );
//
//         StatisticQueries::insert_or_update_row(&mut conn, test_row)?;
//
//         let fetched: LoggingData<i64, IntegerAmountGoal> = StatisticQueries::get_row(&mut conn, stat_id, test_date)?
//             .expect("Row should exist");
//         assert_eq!(fetched.value, Some(test_value));
//
//         Ok(())
//     }
//
//     #[ignore]
//     #[test]
//     fn test_get_rows_includes_empty_rows() -> Result<()> {
//         let mut conn = establish_test_connection()?;
//         let stat_id = StatisticId(1);
//         let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
//         let end_date = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
//
//         let rows = StatisticQueries::get_rows_in_range::<i64, IntegerAmountGoal>(
//             &mut conn,
//             stat_id,
//             start_date,
//             end_date,
//         )?;
//
//         assert_eq!(rows.len(), 3, "Should have 3 days of data");
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_update_value_preserves_goal() -> Result<()> {
//         let mut conn = establish_test_connection()?;
//         let stat_id = StatisticId(1);
//         let test_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
//
//         // First set up a goal
//         let amount_goal = IntegerAmountGoal::new(50000, true);
//         StatisticQueries::insert_or_update_goal::<i64, _>(
//             &mut conn,
//             stat_id,
//             test_date,
//             Some(amount_goal),
//         )?;
//
//         // Then update the value
//         let test_value: i64 = 51235;
//         StatisticQueries::insert_or_update_value::<i64, IntegerAmountGoal>(
//             &mut conn,
//             stat_id,
//             test_date,
//             test_value,
//         )?;
//
//         // Verify both value and goal are preserved
//         let row = StatisticQueries::get_row::<i64, IntegerAmountGoal>(&mut conn, stat_id, test_date)?
//             .expect("Row should exist");
//         assert_eq!(row.value, Some(test_value));
//         assert_eq!(row.goal, Some(amount_goal));
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_update_goal_preserves_value() -> Result<()> {
//         let mut conn = establish_test_connection()?;
//         let stat_id = StatisticId(1);
//         let test_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
//
//         // First set a value
//         let test_value: i64 = 51235;
//         StatisticQueries::insert_or_update_value::<i64, IntegerAmountGoal>(
//             &mut conn,
//             stat_id,
//             test_date,
//             test_value,
//         )?;
//
//         // Then update the goal
//         let amount_goal = IntegerAmountGoal::new(50000, true);
//         StatisticQueries::insert_or_update_goal::<i64, _>(
//             &mut conn,
//             stat_id,
//             test_date,
//             Some(amount_goal),
//         )?;
//
//         // Verify both value and goal are preserved
//         let row = StatisticQueries::get_row::<i64, IntegerAmountGoal>(&mut conn, stat_id, test_date)?
//             .expect("Row should exist");
//         assert_eq!(row.value, Some(test_value));
//         assert_eq!(row.goal, Some(amount_goal));
//
//         Ok(())
//     }
// }
