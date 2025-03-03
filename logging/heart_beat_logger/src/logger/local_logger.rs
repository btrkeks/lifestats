use crate::logger::logger_trait::TimeFrameLogger;
use anyhow::Result;
use chrono::{Local, NaiveTime};
use lifestats_core::app_context::AppContext;
use lifestats_core::log::log_time_frame;
use lifestats_core::TimeFrameListUpdatePolicy;
use std::sync::Arc;
use transfer_objects::statistic_id::StatisticId;

#[derive(Clone)]
pub struct LocalLogger {
    statistic_id: StatisticId,
    app_context: Arc<AppContext>,
}

impl LocalLogger {
    pub fn new(statistic_id: StatisticId, app_context: Arc<AppContext>) -> Self {
        Self {
            statistic_id,
            app_context,
        }
    }
}

impl TimeFrameLogger for LocalLogger {
    fn log(&self, start_time: NaiveTime, end_time: NaiveTime) -> Result<()> {
        let today = Local::now().date_naive();
        log_time_frame(
            &self.app_context,
            self.statistic_id,
            today,
            start_time,
            end_time,
            Some(TimeFrameListUpdatePolicy::UpdateEndValue),
        )
    }
}
