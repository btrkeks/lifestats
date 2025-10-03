use crate::shared::ids::StatisticId;
use crate::statistic::any_statistic::AnyStatistic;
use anyhow::Result;

#[cfg_attr(test, mockall::automock)]
pub trait StatisticRepositoryTrait {
    fn find_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
    fn find_all(&self) -> Result<Box<[AnyStatistic]>>;
    fn save(&self, statistic: &AnyStatistic) -> Result<()>;
}
