use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

#[derive(Clone)]
pub struct LoggingService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> LoggingService<R, E>
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

    /// Generic helper to parse and calculate new log values for statistics with FromStr parsing
    ///
    /// This helper encapsulates the common pattern of:
    /// 1. Deserializing the current value from JSON
    /// 2. Parsing the new value from a string
    /// 3. Parsing the update policy
    /// 4. Calling the pure domain calculation
    /// 5. Serializing the result back to JSON
    fn parse_and_calculate<
        S: lifestats_core::statistic::traits::logable_statistic::LogableStatistic,
    >(
        &self,
        stat: &S,
        current_value_json: Option<serde_json::Value>,
        value_str: &str,
        update_policy_str: Option<&str>,
    ) -> Result<(serde_json::Value, bool)>
    where
        S::LogValueType: serde::de::DeserializeOwned + serde::Serialize + std::str::FromStr,
        <S::LogValueType as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        use anyhow::Context;

        let current_val: Option<S::LogValueType> = current_value_json
            .map(serde_json::from_value)
            .transpose()
            .context("Failed to deserialize current value")?;

        let new_val: S::LogValueType = value_str.parse().context("Could not parse input string")?;

        // Use the domain helper to parse the update policy string
        let update_policy = stat.parse_update_policy(update_policy_str)?;

        // Call the pure domain logic
        let (result, changed) = stat.calculate_new_log_value(current_val, new_val, update_policy);

        // Serialize the pure domain result back to JSON for storage
        let result_json = serde_json::to_value(result).context("Failed to serialize result")?;

        Ok((result_json, changed))
    }

    /// Generic polymorphic logging method that works with any statistic type
    ///
    /// This method acts as the Application Layer translator, parsing primitives
    /// into domain types before calling pure domain logic.
    pub fn log_value(&self, command: LogValueCommand) -> Result<()> {
        use anyhow::{anyhow, Context};
        use lifestats_core::statistic::any_statistic::AnyStatistic;
        use lifestats_core::statistic::traits::logable_statistic::LogableStatistic;

        // 1. Fetch the domain entity
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 2. Fetch the current value as JSON
        let current_value_json = self
            .statistic_repository
            .find_log_value(command.statistic_id, command.date)?;

        // 3. APPLICATION LAYER LOGIC: Match on the statistic type to parse and call pure domain logic.
        let (new_value_json, changed) = match &statistic {
            AnyStatistic::IntegerAmount(s) => self.parse_and_calculate(
                s,
                current_value_json,
                &command.value_str,
                command.update_policy_str.as_deref(),
            )?,
            AnyStatistic::Habit(s) => self.parse_and_calculate(
                s,
                current_value_json,
                &command.value_str,
                command.update_policy_str.as_deref(),
            )?,
            AnyStatistic::PointInTime(s) => {
                use lifestats_core::shared::my_time::MyTime;
                use std::str::FromStr;
                let current_val: Option<chrono::NaiveTime> = current_value_json
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Failed to deserialize current PointInTime value")?;
                let new_val_wrapper = MyTime::from_str(&command.value_str)
                    .context("Could not parse input string as a time")?;
                let update_policy = s.parse_update_policy(command.update_policy_str.as_deref())?;

                let (result, changed) = s.calculate_new_log_value(
                    current_val,
                    new_val_wrapper.into_inner(),
                    update_policy,
                );
                (serde_json::to_value(result)?, changed)
            }
            AnyStatistic::TimeSpans(s) => {
                use lifestats_core::shared::time_frame_list::TimeFrameList;
                let current_val: Option<TimeFrameList> = current_value_json
                    .map(serde_json::from_value)
                    .transpose()
                    .context("Failed to deserialize current TimeSpans value")?;
                // TimeFrameList parsing is from JSON string, not a simple value.
                let new_val: TimeFrameList = serde_json::from_str(&command.value_str)
                    .context("Could not parse input string as a TimeFrameList JSON")?;
                let update_policy = s.parse_update_policy(command.update_policy_str.as_deref())?;

                let (result, changed) =
                    s.calculate_new_log_value(current_val, new_val, update_policy);
                (serde_json::to_value(result)?, changed)
            }
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                return Err(anyhow!("Cannot log values to a derived statistic."));
            }
        };

        // 4. If changed, save the new JSON state and publish an event
        if changed {
            self.statistic_repository.save_log_value(
                command.statistic_id,
                command.date,
                new_value_json,
            )?;
            self.event_publisher
                .publish_event_async(Event::ValueUpdated(command.statistic_id));
        }

        Ok(())
    }

    /// Type-safe method for logging a time frame to a TimeSpans statistic
    ///
    /// This method provides type safety by accepting structured time values
    /// rather than strings, preventing parsing errors at the domain layer.
    pub fn log_time_frame(&self, command: LogTimeFrameCommand) -> Result<()> {
        use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
        use lifestats_core::statistic::any_statistic::AnyStatistic;
        use lifestats_core::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;

        // 1. Fetch the domain entity
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 2. Verify it's a TimeSpans statistic
        let time_spans_stat = match &statistic {
            AnyStatistic::TimeSpans(s) => s,
            _ => {
                return Err(anyhow::anyhow!(
                    "Statistic {} is not a TimeSpans statistic",
                    command.statistic_id.0
                ));
            }
        };

        // 3. Create the TimeFrame from the command
        let time_frame = TimeFrame::new(command.start_time, command.end_time)?;

        // 4. Create a TimeFrameList containing the single frame
        let mut new_time_frame_list = TimeFrameList::default();
        new_time_frame_list.append_frame(time_frame)?;

        // 5. Fetch the current value
        let current_value_json = self
            .statistic_repository
            .find_log_value(command.statistic_id, command.date)?;

        let current_value: Option<TimeFrameList> = current_value_json
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| anyhow::anyhow!("Failed to deserialize current value: {}", e))?;

        // 6. Determine update policy (default to Append)
        let update_policy = command
            .update_policy
            .unwrap_or(TimeFrameListUpdatePolicy::Append);

        // 7. Calculate the new value using domain logic
        let (new_value, changed) = time_spans_stat.calculate_new_log_value(
            current_value,
            new_time_frame_list,
            update_policy,
        );

        // 8. If changed, save and publish event
        if changed {
            let new_value_json = serde_json::to_value(&new_value)
                .map_err(|e| anyhow::anyhow!("Failed to serialize new value: {}", e))?;

            self.statistic_repository.save_log_value(
                command.statistic_id,
                command.date,
                new_value_json,
            )?;

            self.event_publisher
                .publish_event_async(Event::ValueUpdated(command.statistic_id));
        }

        Ok(())
    }
}

/// Command for the generic polymorphic log_value method
pub struct LogValueCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value_str: String,
    pub update_policy_str: Option<String>,
}

/// Command for logging a time frame to a TimeSpans statistic
pub struct LogTimeFrameCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub update_policy: Option<lifestats_core::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy>,
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
    fn test_log_value_integer_amount_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(1);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(1, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        // Expect repository calls
        repo.expect_find_metadata_by_id()
            .times(1)
            .withf(move |id| *id == stat_id)
            .return_once(move |_| Ok(stat));

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

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "5".to_string(),
            update_policy_str: Some("sum".to_string()),
        };

        // Act
        let result = service.log_value(command);

        // Assert
        if let Err(e) = &result {
            eprintln!("Error in test_log_value_integer_amount_happy_path: {:?}", e);
        }
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

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

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

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "true".to_string(),
            update_policy_str: None,
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_value_parsing_failure() {
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

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "not_a_number".to_string(),
            update_policy_str: None,
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Could not parse input string"));
    }

    #[test]
    fn test_log_value_derived_statistic_fails() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(4);
        let test_date = Local::now().date_naive();

        // Create a derived statistic (Summing)
        use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
        let metadata = create_test_metadata(4, "test_summing");

        // Create source statistics for the summing statistic
        let source1 = TimeSpansStatistic::new(create_test_metadata(10, "source1"), None, test_date);
        let source2 = TimeSpansStatistic::new(create_test_metadata(11, "source2"), None, test_date);

        let stat = AnyStatistic::Summing(SummingStatistic::new(
            metadata,
            vec![source1, source2],
            None,
            test_date,
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "100".to_string(),
            update_policy_str: None,
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot log values to a derived statistic"));
    }

    #[test]
    fn test_log_value_point_in_time_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(5);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(5, "test_point_in_time");
        let stat = AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        repo.expect_save_log_value()
            .times(1)
            .withf(move |id, _, _| *id == stat_id)
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "14:30".to_string(),
            update_policy_str: None,
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_value_time_spans_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let mut event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(6);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(6, "test_time_spans");
        let stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        repo.expect_save_log_value()
            .times(1)
            .withf(move |id, _, _| *id == stat_id)
            .return_once(|_, _, _| Ok(()));

        event_publisher
            .expect_publish_event_async()
            .times(1)
            .return_once(|_| ());

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        // Valid JSON for TimeFrameList
        let time_frames_json = r#"[{"start":"14:30:00","end":"15:30:00"}]"#;

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: time_frames_json.to_string(),
            update_policy_str: Some("append".to_string()),
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_log_value_no_change_no_save() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(7);
        let test_date = Local::now().date_naive();
        let metadata = create_test_metadata(7, "test_integer");
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(Some(serde_json::json!(10))));

        // No save should happen since value doesn't change
        repo.expect_save_log_value().times(0);

        let service = LoggingService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = LogValueCommand {
            statistic_id: stat_id,
            date: test_date,
            value_str: "10".to_string(),
            update_policy_str: Some("overwrite".to_string()),
        };

        // Act
        let result = service.log_value(command);

        // Assert
        assert!(result.is_ok());
    }
}
