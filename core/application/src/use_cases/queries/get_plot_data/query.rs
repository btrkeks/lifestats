use crate::cqrs::Query;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;
use plot_domain::plot::data::PlotData;
use plot_domain::plot::specification::PlotType;

/// Query to retrieve plot data for stateless plotting
///
/// This query generates a complete PlotData object on-the-fly without
/// requiring any stored plot configuration. It accepts:
/// - `statistic_id`: Which statistic to plot
/// - `plot_type`: What kind of visualization to create
/// - `date_range`: The time period to include
///
/// The handler builds a fully-populated PlotData domain object with:
/// - Converted DrawableData/DrawableGoal (not raw JSON)
/// - Computed colors based on goal achievement
/// - Calculated Y-axis configuration
///
/// This replaces the old stateful model where PlotId referenced a stored PlotSpecification.
pub struct GetPlotDataQuery {
    pub statistic_id: StatisticId,
    pub plot_type: PlotType,
    pub date_range: DateRange,
}

impl Query for GetPlotDataQuery {
    type Result = PlotData;
}
