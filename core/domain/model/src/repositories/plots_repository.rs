use crate::plot::any_plot::AnyPlot;
use anyhow::Result;
use mockall::automock;
use transfer_objects::statistic_id::{PlotId, StatisticId};

#[automock]
pub trait PlotRepositoryTrait: Sync + Send {
    fn get_by_id(&self, id: PlotId) -> Result<Option<AnyPlot>>;
    fn get_all(&self) -> Result<Box<[AnyPlot]>>;
    fn get_for_statistic_id(&self, stat_id: StatisticId) -> Result<Box<[AnyPlot]>>;
    fn save(&self, plot: AnyPlot) -> Result<()>;
}
