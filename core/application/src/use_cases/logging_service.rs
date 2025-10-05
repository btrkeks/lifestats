use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

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

    /// Generic polymorphic logging method that works with any statistic type
    ///
    /// This method uses the "Tell, Don't Ask" principle - it passes primitives
    /// to the domain object and lets it handle all type-specific logic.
    pub fn log_value(&self, command: LogValueCommand) -> Result<()> {
        // 1. Fetch the domain entity
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 2. Fetch the current value as JSON
        let current_value_json = self
            .statistic_repository
            .find_log_value(command.statistic_id, command.date)?;

        // 3. TELL, DON'T ASK: Ask the domain object to calculate its new state from primitives
        let (new_value_json, changed) = statistic.calculate_new_log_value_from_primitives(
            current_value_json,
            &command.value_str,
            command.update_policy_str.as_deref(),
        )?;

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
        use lifestats_domain::shared::time_frame_list::{TimeFrame, TimeFrameList};
        use lifestats_domain::statistic::any_statistic::AnyStatistic;
        use lifestats_domain::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;

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
    pub update_policy: Option<lifestats_domain::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy>,
}
