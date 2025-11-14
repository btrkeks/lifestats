use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use anyhow::anyhow;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::{
    amount_goal::AmountGoal, finished_in_time_goal::FinishedInTimeGoal, habit_goal::HabitGoal,
    time_spans_goal::TimeSpansGoal,
};

use super::command::{DailyGoalOverride, GoalPayload, SetDailyGoalCommand};

/// Handler for SetDailyGoalCommand
///
/// This handler creates strongly-typed goal objects and persists them.
/// No HashMap parsing required - all types are validated at compile time.
pub struct SetDailyGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> SetDailyGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

fn suppress_goal_for_day<R>(
    repository: &Arc<R>,
    statistic_id: lifestats_core::shared::ids::StatisticId,
    date: chrono::NaiveDate,
) -> ApplicationResult<()>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository.save_goal(statistic_id, date, None)?;
    Ok(())
}

fn store_goal<R>(
    repository: &Arc<R>,
    statistic_id: lifestats_core::shared::ids::StatisticId,
    date: chrono::NaiveDate,
    goal_json: serde_json::Value,
) -> ApplicationResult<()>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository.save_goal(statistic_id, date, Some(goal_json))?;
    Ok(())
}

impl<R, E> CommandHandler<SetDailyGoalCommand> for SetDailyGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: SetDailyGoalCommand) -> ApplicationResult<()> {
        // 1. Fetch the statistic to verify it exists and get its type
        let statistic = self.repository.find_metadata_by_id(command.statistic_id)?;

        match command.override_kind {
            super::command::DailyGoalOverride::Goal(goal_payload) => {
                let goal_json = parse_goal_payload_to_json(goal_payload, statistic)?;
                store_goal(
                    &self.repository,
                    command.statistic_id,
                    command.date,
                    goal_json,
                )?;
            }
            super::command::DailyGoalOverride::Suppressed => {
                suppress_goal_for_day(&self.repository, command.statistic_id, command.date)?;
            }
        }

        // Publish event
        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

fn parse_goal_payload_to_json(
    goal_payload: GoalPayload,
    statistic: AnyStatistic,
) -> Result<serde_json::Value, ApplicationError> {
    let goal_json = match (&statistic, goal_payload) {
        (
            AnyStatistic::IntegerAmount(_),
            GoalPayload::IntegerAmount {
                target,
                is_at_least,
            },
        ) => {
            let goal = AmountGoal::new(target, is_at_least);
            serde_json::to_value(&goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?
        }

        (AnyStatistic::Habit(_), GoalPayload::Habit { occurred }) => {
            let goal = HabitGoal::new(occurred);
            serde_json::to_value(&goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?
        }

        (AnyStatistic::PointInTime(_), GoalPayload::FinishedInTime { deadline }) => {
            let goal = FinishedInTimeGoal::new(deadline);
            serde_json::to_value(&goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?
        }

        (AnyStatistic::TimeSpans(_), GoalPayload::TimeSpans { target, efficiency }) => {
            let goal = TimeSpansGoal::new(target, efficiency).map_err(ApplicationError::Domain)?;
            serde_json::to_value(&goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?
        }

        (
            AnyStatistic::Summing(_),
            GoalPayload::SecondsAmount {
                target,
                is_at_least,
            },
        )
        | (
            AnyStatistic::TimeDifference(_),
            GoalPayload::SecondsAmount {
                target,
                is_at_least,
            },
        ) => {
            let goal = AmountGoal::new(target, is_at_least);
            serde_json::to_value(&goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?
        }

        // Mismatched types
        _ => {
            return Err(ApplicationError::Infrastructure(anyhow!(
                "Goal type does not match statistic type"
            )));
        }
    };
    Ok(goal_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::{
        habit::HabitStatistic, integer_amount::IntegerAmountStatistic,
    };
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    fn create_test_metadata(id: u32, name: &str) -> StatisticMetadata {
        StatisticMetadata::new(StatisticId(id), "test".into(), name.into())
    }

    #[test]
    fn test_set_daily_goal_integer_happy_path() {
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

        let handler = SetDailyGoalCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = SetDailyGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            override_kind: DailyGoalOverride::Goal(GoalPayload::IntegerAmount {
                target: 100,
                is_at_least: true,
            }),
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_daily_goal_habit_happy_path() {
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

        let handler = SetDailyGoalCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = SetDailyGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            override_kind: DailyGoalOverride::Goal(GoalPayload::Habit { occurred: true }),
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_daily_goal_type_mismatch() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(3);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(3, "test_habit");
        let stat = AnyStatistic::Habit(HabitStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let handler = SetDailyGoalCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        // Try to set an integer goal on a habit statistic
        let command = SetDailyGoalCommand {
            statistic_id: stat_id,
            date: test_date,
            override_kind: DailyGoalOverride::Goal(GoalPayload::IntegerAmount {
                target: 100,
                is_at_least: true,
            }),
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("does not match"));
    }
}
