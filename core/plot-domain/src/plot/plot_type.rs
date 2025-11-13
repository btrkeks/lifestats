use serde::{Deserialize, Serialize};

/// Represents the type of plot visualization to render
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlotType {
    BarChart,
    LineChart,
    TimeFrames,
    Scatter,
    CircularProgress,
}
