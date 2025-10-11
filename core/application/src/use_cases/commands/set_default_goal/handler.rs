use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use anyhow::anyhow;
use chrono::Local;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::traits::base_statistic::Statistic;
use lifestats_core::statistic::value_objects::goals::{
    amount_goal::AmountGoal, finished_in_time_goal::FinishedInTimeGoal, habit_goal::HabitGoal,
    time_spans_goal::TimeSpansGoal,
};

use super::super::set_daily_goal::GoalPayload;
use super::command::SetDefaultGoalCommand;

/// Handler for SetDefaultGoalCommand
///
/// This handler updates both the domain entity's default goal and
/// persists it to the repository.
pub struct SetDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> SetDefaultGoalCommandHandler<R, E>
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

impl<R, E> CommandHandler<SetDefaultGoalCommand> for SetDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: SetDefaultGoalCommand) -> ApplicationResult<()> {
        // 1. Fetch the statistic
        let mut statistic = self.repository.find_metadata_by_id(command.statistic_id)?;

        let start_date = Local::now().date_naive();

        // 2. Create typed goal and update the domain entity
        let goal_json = match (&mut statistic, command.goal) {
            (
                AnyStatistic::IntegerAmount(s),
                GoalPayload::IntegerAmount {
                    target,
                    is_at_least,
                },
            ) => {
                let goal = AmountGoal::new(target, is_at_least);
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            (AnyStatistic::Habit(s), GoalPayload::Habit { occurred }) => {
                let goal = HabitGoal::new(occurred);
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            (AnyStatistic::PointInTime(s), GoalPayload::FinishedInTime { deadline }) => {
                let goal = FinishedInTimeGoal::new(deadline);
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            (AnyStatistic::TimeSpans(s), GoalPayload::TimeSpans { target, efficiency }) => {
                let goal =
                    TimeSpansGoal::new(target, efficiency).map_err(ApplicationError::Domain)?;
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            (
                AnyStatistic::Summing(s),
                GoalPayload::SecondsAmount {
                    target,
                    is_at_least,
                },
            ) => {
                let goal = AmountGoal::new(target, is_at_least);
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            (
                AnyStatistic::TimeDifference(s),
                GoalPayload::SecondsAmount {
                    target,
                    is_at_least,
                },
            ) => {
                let goal = AmountGoal::new(target, is_at_least);
                s.set_new_default_goal(goal.clone(), start_date)
                    .map_err(|e| ApplicationError::Infrastructure(anyhow::Error::new(e)))?;
                serde_json::to_value(&goal)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?
            }

            // Mismatched types
            _ => {
                return Err(ApplicationError::Infrastructure(anyhow!(
                    "Goal type does not match statistic type"
                )));
            }
        };

        // 3. Save updated metadata and default goal
        self.repository.update_metadata(statistic)?;
        self.repository
            .save_default_goal(command.statistic_id, goal_json)?;

        // 4. Publish event
        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    fn create_test_metadata(id: u32, name: &str) -> StatisticMetadata {
        StatisticMetadata::new(StatisticId(id), "test".into(), name.into())
    }

    #[test]
    fn test_set_default_goal_happy_path() {
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

        repo.expect_update_metadata()
            .times(1)
            .return_once(|_| Ok(()));

        repo.expect_save_default_goal()
            .times(1)
            .withf(move |id, goal| *id == stat_id && goal.is_object())
            .return_once(|_, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let handler = SetDefaultGoalCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = SetDefaultGoalCommand {
            statistic_id: stat_id,
            goal: GoalPayload::IntegerAmount {
                target: 100,
                is_at_least: true,
            },
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_ok());
    }
}
