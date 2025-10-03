use crate::shared::ids::StatisticId;
use crate::statistic::entities::logable_statistics::time_spans::TimeSpansStatistic;
use anyhow::Result;

pub trait StatisticRepository {
    fn load_time_spans(&self, id: StatisticId) -> Result<Option<TimeSpansStatistic>>;
    fn save_time_spans(&self, time_spans: TimeSpansStatistic) -> Result<TimeSpansStatistic>;
}