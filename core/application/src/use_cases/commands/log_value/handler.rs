use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::command::{LogValueCommand, LogValuePayload};

/// Handler for LogValueCommand
///
/// This handler orchestrates the domain logic for logging a value:
/// 1. Fetch current state from repository
/// 2. Call pure domain method to calculate new state
/// 3. Save new state if changed
/// 4. Publish event if changed
pub struct LogValueCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> LogValueCommandHandler<R, E>
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

impl<R, E> CommandHandler<LogValueCommand> for LogValueCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: LogValueCommand) -> ApplicationResult<()> {
        use anyhow::{anyhow, Context};

        let LogValueCommand {
            statistic_id,
            date,
            payload,
            statistic,
        } = command;

        // Fetch the current value as JSON
        let current_value_json = self.repository.find_log_value(statistic_id, date)?;

        // 3. Match on payload type and statistic type to execute the appropriate domain logic
        let (new_value_json, changed) = match (statistic, payload) {
            // Integer amount
            (
                AnyStatistic::IntegerAmount(s),
                LogValuePayload::Integer {
                    value,
                    update_policy,
                },
            ) => {
                let current_val: Option<i64> = current_value_json
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Failed to deserialize current IntegerAmount value")?;

                let (result, changed) =
                    s.calculate_new_log_value(current_val, value, update_policy);

                let result_json = serde_json::to_value(result)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

                (result_json, changed)
            }

            // Habit
            (
                AnyStatistic::Habit(s),
                LogValuePayload::Habit {
                    value,
                    update_policy,
                },
            ) => {
                let current_val: Option<bool> = current_value_json
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Failed to deserialize current Habit value")?;

                let (result, changed) =
                    s.calculate_new_log_value(current_val, value, update_policy);

                let result_json = serde_json::to_value(result)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

                (result_json, changed)
            }

            // Point in time
            (
                AnyStatistic::PointInTime(s),
                LogValuePayload::PointInTime {
                    value,
                    update_policy,
                },
            ) => {
                let current_val: Option<chrono::NaiveTime> = current_value_json
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Failed to deserialize current PointInTime value")?;

                let (result, changed) =
                    s.calculate_new_log_value(current_val, value, update_policy);

                let result_json = serde_json::to_value(result)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

                (result_json, changed)
            }

            // Time frames
            (
                AnyStatistic::TimeSpans(s),
                LogValuePayload::TimeFrames {
                    value,
                    update_policy,
                },
            ) => {
                let current_val: Option<lifestats_core::shared::time_frame_list::TimeFrameList> =
                    current_value_json
                        .map(serde_json::from_value)
                        .transpose()
                        .context("Failed to deserialize current TimeSpans value")?;

                let (result, changed) =
                    s.calculate_new_log_value(current_val, value, update_policy);

                let result_json = serde_json::to_value(result)
                    .map_err(|e| ApplicationError::Infrastructure(e.into()))?;

                (result_json, changed)
            }

            // Derived statistics cannot be logged to
            (AnyStatistic::Summing(_), _) | (AnyStatistic::TimeDifference(_), _) => {
                return Err(ApplicationError::Infrastructure(anyhow!(
                    "Cannot log values to a derived statistic."
                )));
            }

            // Mismatched types (e.g., trying to log an integer to a habit statistic)
            _ => {
                return Err(ApplicationError::Infrastructure(anyhow!(
                    "Payload type does not match statistic type."
                )));
            }
        };

        // 4. If changed, save the new JSON state and publish an event
        if changed {
            self.repository
                .save_log_value(statistic_id, date, new_value_json)?;
            self.event_publisher
                .publish_event_async(Event::ValueUpdated(statistic_id));
        }

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
    use lifestats_core::statistic::types::logable_statistics::{
        habit::HabitStatistic, integer_amount::IntegerAmountStatistic,
    };
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use lifestats_core::statistic::value_objects::update_policies::amount::AmountUpdatePolicy;
    use lifestats_core::statistic::value_objects::update_policies::fixed_value::FixedValueUpdatePolicy;

    fn create_test_metadata(id: u32, name: &str) -> StatisticMetadata {
        StatisticMetadata::new(StatisticId(id), "test".into(), name.into())
    }

    #[test]
    fn test_log_value_integer_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(1);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(1, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(Some(serde_json::json!(10))));

        repo.expect_save_log_value()
            .times(1)
            .withf(move |id, date, value| {
                *id == stat_id && *date == test_date && value == &serde_json::json!(15)
            })
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let handler = LogValueCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            payload: LogValuePayload::Integer {
                value: 5,
                update_policy: AmountUpdatePolicy::Sum,
            },
            statistic: stat,
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_value_habit_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(2);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(2, "test_habit");
        let stat = AnyStatistic::Habit(HabitStatistic::new(metadata, None, test_date));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        repo.expect_save_log_value()
            .times(1)
            .withf(move |id, _, value| *id == stat_id && value == &serde_json::json!(true))
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let handler = LogValueCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            payload: LogValuePayload::Habit {
                value: true,
                update_policy: FixedValueUpdatePolicy::Overwrite,
            },
            statistic: stat,
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_value_type_mismatch() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(3);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(3, "test_habit");
        let stat = AnyStatistic::Habit(HabitStatistic::new(metadata, None, test_date));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        let handler = LogValueCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        // Try to log an integer to a habit statistic
        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            payload: LogValuePayload::Integer {
                value: 10,
                update_policy: AmountUpdatePolicy::Overwrite,
            },
            statistic: stat,
        };

        // Act
        let result = handler.handle(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("does not match"));
    }
}
