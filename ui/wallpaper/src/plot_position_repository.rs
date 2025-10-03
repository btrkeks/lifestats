use crate::plot_position::PlotPosition;
use anyhow::Result;
use transfer_objects::statistic_id::PlotId;

pub trait PlotPositionRepository {
    fn get_position_for_id(&self, plot_id: PlotId) -> Result<Option<PlotPosition>>;
}
