use crate::plot::{AnyPlot, Plot};
use anyhow::Result;

pub trait PlotRepository {
    fn get_by_id(&self, id: u64) -> Result<Option<AnyPlot>>;
    fn get_all(&self) -> Result<Vec<AnyPlot>>;
    fn get_for_statistic_id(&self, stat_id: u64) -> Result<Vec<AnyPlot>>;
    fn save(&self, plot: AnyPlot) -> Result<()>;
}