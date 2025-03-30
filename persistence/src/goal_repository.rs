use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use domain_models::repositories::goal_repository::GoalRepositoryTrait;
use domain_models::objects::goal::Goal;
use domain_models::objects::logging_data::{DateValue, StatisticData};
use serde::Serialize;
use std::marker::PhantomData;
use std::sync::Arc;
use transfer_objects::statistic_id::StatisticId;

use crate::sqlite_database::models::{DefaultGoalRaw, GoalDataRaw};
use crate::sqlite_database::schema::default_goals;
use crate::sqlite_database::schema::goals;
use diesel::r2d2::{ConnectionManager, Pool};
use serde::de::DeserializeOwned;

pub struct GoalRepositoryTemplate<T, G: Goal<T>>
where
    T: Send + Sync,
    G: Send + Sync,
{
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    _phantom: PhantomData<(T, G)>,
}

impl<T, G: Goal<T>> GoalRepositoryTemplate<T, G>
where
    T: Send + Sync,
    G: Send + Sync,
{
    pub fn new(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self {
            pool,
            _phantom: Default::default(),
        }
    }
}

impl<T, G: Goal<T>> GoalRepositoryTrait for GoalRepositoryTemplate<T, G>
where
    T: Send + Sync + DeserializeOwned + Serialize,
    G: Send + Sync,
{
    type StoredValue = T;
    type GoalType = G;

    fn get_for_closed_range(
        &self,
        id: StatisticId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<Self::GoalType>>>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection from pool")?;

        let data = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.between(start_date.to_string(), end_date.to_string()))
            .order(goals::date.asc())
            .load::<GoalDataRaw>(&mut conn)
            .context(format!(
                "Failed to get goals for statistic_id: {} in range {} - {}",
                id.0, start_date, end_date
            ))?;

        Ok(data
            .into_iter()
            .map(|logging_data| logging_data.into_goal_date_value())
            .collect())
    }

    fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Option<Self::GoalType>>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection from pool")?;

        let data: Option<GoalDataRaw> = goals::table
            .filter(goals::statistic_id.eq(id.0 as i32))
            .filter(goals::date.eq(date.to_string()))
            .first(&mut conn)
            .optional() // This converts a NotFound error into Ok(None)
            .context(format!(
                "Failed to get goal for statistic_id: {}, date: {}",
                id.0, date
            ))?;

        Ok(data.map(|d| d.into_goal_date_value().value))
    }

    fn set_for_date(
        &self,
        id_param: StatisticId,
        date_param: NaiveDate,
        new_goal: Option<Self::GoalType>,
    ) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection from pool")?;

        let raw_goal = GoalDataRaw::from_goal_logging_data(StatisticData {
            statistic_id: id_param,
            date: date_param,
            value: new_goal,
        })?;

        use crate::sqlite_database::schema::goals::dsl::*;

        diesel::insert_into(goals)
            .values(&raw_goal)
            .on_conflict((statistic_id, date))
            .do_update()
            .set(&raw_goal)
            .execute(&mut conn)
            .context(format!(
                "Failed to set goal for statistic_id: {}, date: {:?}",
                id_param.0, date_param
            ))?;

        Ok(())
    }

    fn get_default(&self, id: StatisticId) -> Result<Option<Self::GoalType>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection from pool")?;

        let goal: Option<DefaultGoalRaw> = default_goals::table
            .filter(default_goals::statistic_id.eq(id.0 as i32))
            .first(&mut conn)
            .optional()
            .context(format!(
                "Failed to get default goal for statistic_id: {}",
                id.0
            ))?;

        goal.map(|g| g.into_goal()).transpose()
    }

    fn set_default(&self, id_param: StatisticId, new_goal: Self::GoalType) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection from pool")?;

        let raw_goal = DefaultGoalRaw::from_goal(id_param, new_goal);

        diesel::insert_into(default_goals::table)
            .values(&raw_goal)
            .on_conflict(default_goals::statistic_id)
            .do_update()
            .set(&raw_goal)
            .execute(&mut conn)
            .context(format!(
                "Failed to set default goal for statistic_id: {}",
                id_param.0
            ))?;

        Ok(())
    }

    fn delete_default(&self, id: StatisticId) -> Result<()> {
        let pool = self.pool.clone();
        let mut conn = pool.get()?;

        // Delete the default goal for the specified statistic
        diesel::delete(default_goals::table)
            .filter(default_goals::statistic_id.eq(id.0 as i32))
            .execute(&mut conn)
            .map_err(|e| anyhow!("Failed to delete default goal for statistic {}: {}", id, e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::NaiveTime;
    use diesel_migrations::MigrationHarness;
    use domain_models::objects::seconds::MyDuration;
    use domain_models::objects::time_frame_list::{TimeFrame, TimeFrameList};
    use domain_models::statistic::statistic_types::integer_amount::IntegerAmountGoal;
    use domain_models::statistic::statistic_types::time_amount::TimeAmountGoal;
    use domain_models::statistic::statistic_types::time_spans::TimeSpansGoal;

    fn create_test_pool() -> Arc<Pool<ConnectionManager<SqliteConnection>>> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");

        pool.get()
            .expect("Failed to get connection from pool")
            .run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        Arc::new(pool)
    }

    #[test]
    fn test_set_and_get_goal() -> Result<()> {
        let pool = create_test_pool();

        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let goal = IntegerAmountGoal::new(10, true);

        // Set a goal
        goal_repo.set_for_date(stat_id, date, Some(goal))?;

        // Get the goal
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(Some(goal)));

        Ok(())
    }

    #[test]
    fn test_set_and_get_none_goal() -> Result<()> {
        let pool = create_test_pool();

        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        // Set a goal
        goal_repo.set_for_date(stat_id, date, None)?;

        // Get the goal
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(None));

        Ok(())
    }

    #[test]
    fn test_get_non_existing_goal() -> Result<()> {
        let pool = create_test_pool();

        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        // Get the goal
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, None);

        Ok(())
    }

    #[test]
    fn test_get_goal_range() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);

        let stat_id = StatisticId(1);
        let start_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2023, 1, 5).unwrap();
        let goal1 = IntegerAmountGoal::new(10, true);
        let goal3 = IntegerAmountGoal::new(20, false);

        // Set goals for some dates
        goal_repo.set_for_date(stat_id, start_date, Some(goal1))?;
        goal_repo.set_for_date(
            stat_id,
            NaiveDate::from_ymd_opt(2023, 1, 3).unwrap(),
            Some(goal3),
        )?;

        // Get goals for the range
        let goals = goal_repo.get_for_closed_range(stat_id, start_date, end_date)?;

        assert_eq!(goals[0].value, Some(goal1));
        assert_eq!(goals[1].value, Some(goal3));

        Ok(())
    }

    #[test]
    fn test_update_goal() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let initial_goal = IntegerAmountGoal::new(10, true);
        let updated_goal = IntegerAmountGoal::new(20, false);

        // Set initial goal
        goal_repo.set_for_date(stat_id, date, Some(initial_goal))?;

        // Update the goal
        goal_repo.set_for_date(stat_id, date, Some(updated_goal))?;

        // Get the updated goal
        let retrieved_goal: Option<Option<IntegerAmountGoal>> = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(Some(updated_goal)));

        Ok(())
    }

    #[test]
    fn test_get_nonexistent_goal() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(999); // Assuming this ID doesn't exist
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        // Attempt to get a goal that doesn't exist
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert!(retrieved_goal.is_none());

        Ok(())
    }

    #[test]
    fn test_set_goal_for_nonexistent_date() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);
        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2099, 1, 1).unwrap(); // A date far in the future
        let goal = IntegerAmountGoal::new(100, true);

        // Set goal for a nonexistent date (should insert a new row)
        goal_repo.set_for_date(stat_id, date, Some(goal))?;

        // Verify that the goal was set
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(Some(goal)));

        Ok(())
    }

    #[test]
    fn test_update_goal_with_none() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<i64, IntegerAmountGoal>::new(pool);

        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let goal = IntegerAmountGoal::new(5, true);

        // Set initial goal with some value
        goal_repo.set_for_date(stat_id, date, Some(goal))?;

        // Update goal with None
        goal_repo.set_for_date(stat_id, date, None)?;

        // Verify that the goal is now None
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(None));

        Ok(())
    }

    #[test]
    fn test_set_and_get_time_amount_goal() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<MyDuration, TimeAmountGoal>::new(pool);

        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let goal = TimeAmountGoal::new(MyDuration::from_mins(30), true);

        // Set a goal
        goal_repo.set_for_date(stat_id, date, Some(goal.clone()))?;

        // Get the goal
        let retrieved_goal = goal_repo.get(stat_id, date)?;
        assert_eq!(retrieved_goal, Some(Some(goal)));

        Ok(())
    }

    #[test]
    fn test_set_and_get_time_spans_goal() -> Result<()> {
        let pool = create_test_pool();
        let goal_repo = GoalRepositoryTemplate::<TimeFrameList, TimeSpansGoal>::new(pool);

        let stat_id = StatisticId(1);
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let time_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )?;
        let mut time_frame_list = TimeFrameList::default();
        time_frame_list.append_frame(time_frame)?;
        let goal = TimeSpansGoal::new(time_frame_list, 50)?;

        // Set and retrieve goal
        goal_repo.set_for_date(stat_id, date, Some(goal.clone()))?;
        let retrieved_goal = goal_repo.get(stat_id, date)?;

        assert_eq!(retrieved_goal, Some(Some(goal)));

        Ok(())
    }
}
