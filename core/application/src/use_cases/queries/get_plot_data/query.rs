use crate::cqrs::Query;
use crate::dto::PlotDataResult;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::PlotId;

/// Query to retrieve plot data
///
/// This query returns pure data (PlotDataResult) without any
/// presentation concerns. The adapter layer is responsible for
/// converting this to renderable formats.
pub struct GetPlotDataQuery {
    pub plot_id: PlotId,
    pub date_range: DateRange,
}

impl Query for GetPlotDataQuery {
    type Result = PlotDataResult;
}
