use crate::statistic::statistic_types::any_statistic::AnyStatistic;
use crate::statistic::statistic_types::time_spans::TimeSpansStatistic;
use anyhow::Result;
use mockall::automock;
use transfer_objects::statistic_id::StatisticId;

#[automock]
pub trait StatisticRepositoryTrait: Sync + Send {
    fn get_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
    fn save(&self, statistic: AnyStatistic) -> Result<()>;
    fn get_all(&self) -> Result<Box<[AnyStatistic]>>;
    fn get_all_time_spans_statistics(&self) -> Result<Box<[TimeSpansStatistic]>>;
}
