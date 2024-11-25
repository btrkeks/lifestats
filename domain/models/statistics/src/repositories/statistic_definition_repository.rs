use crate::objects::statistic_metadata::StatisticId;
use crate::statistic::{AnyStatistic};
use anyhow::Result;

pub trait StatisticDefinitionRepository {
    fn get_by_id(&self, id: StatisticId) -> Result<Option<AnyStatistic>>;
    fn save(&self, statistic: AnyStatistic) -> Result<()>;
}