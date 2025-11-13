use lifestats_core::shared::ids::{PlotId, StatisticId};
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
///
/// # Deprecation Notice
/// **This struct is deprecated and will be removed in Phase 4 of the plotting refactor.**
///
/// The plotting system is moving to a stateless model where plots are generated on-the-fly
/// without storing configurations. Instead of using stored `PlotSpecification`, the new
/// approach is to:
/// 1. Accept `statistic_id`, `plot_type`, and `date_range` as request parameters
/// 2. Build `PlotData` dynamically in the application layer
/// 3. Render plots without requiring pre-stored configurations
///
/// **Migration Guide:**
/// - Old: Store PlotSpecification → query by PlotId → render
/// - New: Provide statistic_id + plot_type + date_range → build PlotData → render
///
/// See `PlotData` in `crate::plot::data` for the new stateless model.
#[deprecated(
    since = "0.1.0",
    note = "Use stateless plotting with PlotData instead. Will be removed in Phase 4."
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlotSpecification {
    pub id: PlotId,
    pub title: String,
    pub statistic_id: StatisticId,
    pub plot_type: PlotType,
}

#[allow(deprecated)]
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
