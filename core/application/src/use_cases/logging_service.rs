use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
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
        let _ = (&self.statistic_repository, &self.event_publisher, command);

        bail!("TODO 2.2: implement logging orchestration")
    }
}

pub struct LogTimeFrameListCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value: TimeFrameList,
    pub update_policy: Option<TimeFrameListUpdatePolicy>,
}
