use crate::shared::ids::{PlotId, StatisticId};
use serde::{Deserialize, Serialize};

/// Represents the type of plot to be rendered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlotType {
    BarChart,
    LineChart,
    TimeFrames,
    Scatter,
    CircularProgress,
}

/// Represents a saved plot configuration
/// This is a pure data structure that describes WHAT to plot, not HOW to plot it
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlotSpecification {
    pub id: PlotId,
    pub title: String,
    pub statistic_id: StatisticId,
    pub plot_type: PlotType,
}

impl PlotSpecification {
    pub fn new(id: PlotId, title: String, statistic_id: StatisticId, plot_type: PlotType) -> Self {
        Self {
            id,
            title,
            statistic_id,
            plot_type,
        }
    }
}
