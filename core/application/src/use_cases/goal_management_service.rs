use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ApplicationError, ApplicationResult};
use chrono::{Local, NaiveDate};
use events::event_port::{Event, EventPublisher};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::{
    amount_goal::AmountGoal, finished_in_time_goal::FinishedInTimeGoal, habit_goal::HabitGoal,
    time_spans_goal::TimeSpansGoal,
};
use serde::Serialize;

/// A private enum to hold strongly-typed goal objects
#[derive(Serialize, Clone)]
#[serde(untagged)] // Allows serializing the inner type directly to JSON
enum TypedGoal {
    Habit(HabitGoal),
    IntegerAmount(AmountGoal<i64>),
    FinishedInTime(FinishedInTimeGoal),
    TimeSpans(TimeSpansGoal),
    SecondsAmount(AmountGoal<Seconds>),
}

#[derive(Clone)]
pub struct GoalManagementService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> GoalManagementService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(statistic_repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            statistic_repository,
            event_publisher,
        }
    }

    //===========================================================================
    // HELPER METHODS
    //===========================================================================

    /// Helper to get a required field from the goal data HashMap
    fn get_required_field<'a>(
        data: &'a HashMap<String, String>,
        key: &str,
    ) -> anyhow::Result<&'a String> {
        data.get(key)
            .ok_or_else(|| anyhow::anyhow!("Missing required goal field: '{}'", key))
    }

    /// Helper to parse a string field into a specific type
    fn parse_field<T: std::str::FromStr>(s: &str, field_name: &str) -> anyhow::Result<T> {
        s.parse::<T>()
            .map_err(|_| anyhow::anyhow!("Invalid format for field '{}'", field_name))
    }

    /// Helper to create a strongly-typed goal from primitive data
    fn create_typed_goal(
        &self,
        statistic: &lifestats_core::statistic::any_statistic::AnyStatistic,
        data: &HashMap<String, String>,
    ) -> anyhow::Result<TypedGoal> {
        use lifestats_core::shared::{percentage::Percentage, time_frame_list::TimeFrameList};
        use lifestats_core::statistic::any_statistic::AnyStatistic;

        match statistic {
            AnyStatistic::IntegerAmount(_) => {
                let target: i64 =
                    Self::parse_field(Self::get_required_field(data, "target")?, "target")?;
                let is_at_least = data
                    .get("is_at_least")
                    .map(|s| Self::parse_field(s, "is_at_least"))
                    .transpose()?
                    .unwrap_or(true);
                let goal = AmountGoal::new(target, is_at_least);
                Ok(TypedGoal::IntegerAmount(goal))
            }
            AnyStatistic::Habit(_) => {
                let occurred: bool =
                    Self::parse_field(Self::get_required_field(data, "occurred")?, "occurred")?;
                let goal = HabitGoal::new(occurred);
                Ok(TypedGoal::Habit(goal))
            }
            AnyStatistic::PointInTime(_) => {
                let deadline_str = Self::get_required_field(data, "deadline")?;
                let deadline = chrono::NaiveTime::parse_from_str(deadline_str, "%H:%M:%S")
                    .or_else(|_| chrono::NaiveTime::parse_from_str(deadline_str, "%H:%M"))
                    .map_err(|_| {
                        anyhow::anyhow!("Invalid deadline format. Use HH:MM or HH:MM:SS")
                    })?;
                let goal = FinishedInTimeGoal::new(deadline);
                Ok(TypedGoal::FinishedInTime(goal))
            }
            AnyStatistic::TimeSpans(_) => {
                let target_str = Self::get_required_field(data, "target")?;
                let target: TimeFrameList = serde_json::from_str(target_str)
                    .map_err(|_| anyhow::anyhow!("Invalid JSON for 'target' TimeFrameList"))?;
                let efficiency: u8 = data
                    .get("efficiency")
                    .map(|s| Self::parse_field(s, "efficiency"))
                    .transpose()?
                    .unwrap_or(100);
                if efficiency > 100 {
                    return Err(anyhow::anyhow!("Efficiency must be between 0 and 100"));
                }
                let goal = TimeSpansGoal::new(target, Percentage::new(efficiency));
                Ok(TypedGoal::TimeSpans(goal))
            }
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                let target_secs: i64 =
                    Self::parse_field(Self::get_required_field(data, "target")?, "target")?;
                let is_at_least = data
                    .get("is_at_least")
                    .map(|s| Self::parse_field(s, "is_at_least"))
                    .transpose()?
                    .unwrap_or(true);
                let goal = AmountGoal::new(Seconds::from_i64(target_secs), is_at_least);
                Ok(TypedGoal::SecondsAmount(goal))
            }
        }
    }

    //===========================================================================
    // POLYMORPHIC METHODS (Generic for all statistic types)
    //===========================================================================

    /// Set a goal for a specific date using primitive data (generic for all types)
    pub fn set_goal(&self, command: SetGoalCommand) -> ApplicationResult<()> {
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 1. Create the typed goal
        let typed_goal = self.create_typed_goal(&statistic, &command.goal_data)?;

        // 2. Serialize it to JSON for persistence
        let goal_json = serde_json::to_value(&typed_goal)
            .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

        self.statistic_repository
            .save_goal(command.statistic_id, command.date, Some(goal_json))?;
        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));
        Ok(())
    }

    /// Set a new default goal using primitive data (generic for all types)
    pub fn set_default_goal_generic(
        &self,
        command: SetDefaultGoalGenericCommand,
    ) -> ApplicationResult<()> {
        use lifestats_core::statistic::any_statistic::AnyStatistic;
        use lifestats_core::statistic::traits::base_statistic::Statistic;

        let mut statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 1. Create the typed goal
        let typed_goal = self.create_typed_goal(&statistic, &command.goal_data)?;

        // 2. Serialize it to JSON for persistence
        let goal_json = serde_json::to_value(&typed_goal)
            .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

        // 3. Clone typed_goal before moving it into the match statement
        let typed_goal_for_domain = typed_goal;

        // 4. Update the domain entity using the typed object
        let start_date = Local::now().date_naive();
        match (&mut statistic, typed_goal_for_domain) {
            (AnyStatistic::IntegerAmount(s), TypedGoal::IntegerAmount(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            (AnyStatistic::Habit(s), TypedGoal::Habit(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            (AnyStatistic::PointInTime(s), TypedGoal::FinishedInTime(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            (AnyStatistic::TimeSpans(s), TypedGoal::TimeSpans(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            (AnyStatistic::Summing(s), TypedGoal::SecondsAmount(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            (AnyStatistic::TimeDifference(s), TypedGoal::SecondsAmount(g)) => s
                .set_new_default_goal(g, start_date)
                .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?,
            _ => {
                return Err(ApplicationError::Infrastructure(anyhow::anyhow!(
                    "Mismatched statistic and goal type during default goal setting."
                )))
            }
        };

        // 5. Save everything
        self.statistic_repository.save_metadata(&statistic)?;
        self.statistic_repository
            .save_default_goal(command.statistic_id, goal_json)?;
        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));
        Ok(())
    }

    /// Delete a goal for a specific date (generic for all types)
    pub fn delete_goal(&self, command: DeleteGoalCommand) -> ApplicationResult<()> {
        self.statistic_repository
            .save_goal(command.statistic_id, command.date, None)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

//===========================================================================
// POLYMORPHIC COMMAND STRUCTURES (Generic for all statistic types)
//===========================================================================

/// Command for setting a goal for a specific date (generic)
#[derive(Debug, Clone)]
pub struct SetGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub goal_data: HashMap<String, String>,
}

/// Command for setting a default goal (generic)
#[derive(Debug, Clone)]
pub struct SetDefaultGoalGenericCommand {
    pub statistic_id: StatisticId,
    pub goal_data: HashMap<String, String>,
}

/// Command for deleting a goal for a specific date (generic)
#[derive(Debug, Clone)]
pub struct DeleteGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use events::event_port::MockEventPublisher;
    use lifestats_core::statistic::{
        any_statistic::AnyStatistic,
        ports::statistic_repository::MockStatisticRepository,
        types::logable_statistics::{
            habit::HabitStatistic, integer_amount::IntegerAmountStatistic,
            point_in_time::PointInTimeStatistic, time_spans::TimeSpansStatistic,
        },
        value_objects::statistic_metadata::StatisticMetadata,
    };

    fn create_test_metadata(id: u32, name: &str) -> StatisticMetadata {
        StatisticMetadata::new(StatisticId(id), "test".into(), name.into())
    }

    #[test]
    fn test_set_goal_integer_amount_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(1);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(1, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_save_goal()
            .times(1)
            .withf(move |id, date, goal| *id == stat_id && *date == test_date && goal.is_some())
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert("target".to_string(), "100".to_string());
        goal_data.insert("is_at_least".to_string(), "true".to_string());

        let command = SetGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            goal_data,
        };

        // Act
        let result = service.set_goal(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_goal_habit_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(2);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(2, "test_habit");
        let stat = AnyStatistic::Habit(HabitStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_save_goal()
            .times(1)
            .withf(move |id, _, _| *id == stat_id)
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert("occurred".to_string(), "true".to_string());

        let command = SetGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            goal_data,
        };

        // Act
        let result = service.set_goal(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_goal_missing_required_field() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(3);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(3, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        // Missing "target" field
        let goal_data = HashMap::new();

        let command = SetGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            goal_data,
        };

        // Act
        let result = service.set_goal(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing required goal field"));
    }

    #[test]
    fn test_set_goal_invalid_field_format() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(4);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(4, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert("target".to_string(), "not_a_number".to_string());

        let command = SetGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            goal_data,
        };

        // Act
        let result = service.set_goal(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid format for field"));
    }

    #[test]
    fn test_set_default_goal_generic_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(5);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(5, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Both save_metadata and save_default_goal should be called
        repo.expect_save_metadata().times(1).return_once(|_| Ok(()));

        repo.expect_save_default_goal()
            .times(1)
            .withf(move |id, goal| *id == stat_id && goal.is_object())
            .return_once(|_, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert("target".to_string(), "100".to_string());

        let command = SetDefaultGoalGenericCommand {
            statistic_id: stat_id,
            goal_data,
        };

        // Act
        let result = service.set_default_goal_generic(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_default_goal_point_in_time() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(6);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(6, "test_point_in_time");
        let stat = AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_save_metadata().times(1).return_once(|_| Ok(()));

        repo.expect_save_default_goal()
            .times(1)
            .return_once(|_, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert("deadline".to_string(), "14:30".to_string());

        let command = SetDefaultGoalGenericCommand {
            statistic_id: stat_id,
            goal_data,
        };

        // Act
        let result = service.set_default_goal_generic(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_default_goal_time_spans() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(7);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(7, "test_time_spans");
        let stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_save_metadata().times(1).return_once(|_| Ok(()));

        repo.expect_save_default_goal()
            .times(1)
            .return_once(|_, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let mut goal_data = HashMap::new();
        goal_data.insert(
            "target".to_string(),
            r#"[{"start":"09:00:00","end":"17:00:00"}]"#.to_string(),
        );
        goal_data.insert("efficiency".to_string(), "85".to_string());

        let command = SetDefaultGoalGenericCommand {
            statistic_id: stat_id,
            goal_data,
        };

        // Act
        let result = service.set_default_goal_generic(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_goal() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(8);
        let test_date = Local::now().date_naive();

        repo.expect_save_goal()
            .times(1)
            .withf(move |id, date, goal| *id == stat_id && *date == test_date && goal.is_none())
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = GoalManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = DeleteGoalCommand {
            statistic_id: stat_id,
            date: test_date,
        };

        // Act
        let result = service.delete_goal(command);

        // Assert
        assert!(result.is_ok());
    }
}
