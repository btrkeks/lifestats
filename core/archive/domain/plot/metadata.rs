use serde::{Deserialize, Serialize};
use transfer_objects::statistic_id::{PlotId, StatisticId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlotMetadata {
    pub id: PlotId,
    pub title: String,
    pub statistic_id: StatisticId,
}
