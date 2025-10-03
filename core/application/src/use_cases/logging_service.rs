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
}

/// Command for the generic polymorphic log_value method
pub struct LogValueCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value_str: String,
    pub update_policy_str: Option<String>,
}
