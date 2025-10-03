use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_domain::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;

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

    pub fn log_timeframe_list(&self, command: LogTimeFrameListCommand) -> Result<()> {
        let LogTimeFrameListCommand {
            statistic_id,
            date,
            value,
            update_policy,
        } = command;

        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;

        let time_spans = statistic.as_time_spans().ok_or_else(|| {
            anyhow!(
                "Statistic {} does not support time span logging",
                statistic_id
            )
        })?;

        let current_value = self
            .statistic_repository
            .find_log_value::<TimeFrameList>(statistic_id, date)?;

        let policy = update_policy.unwrap_or_default();
        let (new_value, changed) = time_spans.calculate_new_log_value(current_value, value, policy);

        if !changed {
            return Ok(());
        }

        self.statistic_repository
            .save_log_value(statistic_id, date, new_value)?;

        self.event_publisher
            .publish_event_async(Event::ValueUpdated(statistic_id));

        Ok(())
    }
}

pub struct LogTimeFrameListCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value: TimeFrameList,
    pub update_policy: Option<TimeFrameListUpdatePolicy>,
}
