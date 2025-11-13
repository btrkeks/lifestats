use crate::plot::axis::YAxis;
use crate::plot::plot_type::PlotType;
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::time_frame_list::TimeFrameList;

/// Represents data that has been processed and is ready for visualization.
///
/// This enum eliminates the need for renderers to parse generic JSON.
/// All values have been converted from their storage format (JSON) into
/// strongly-typed, renderer-agnostic data structures.
///
/// # Variants
/// * `Integer` - Numeric data (counts, amounts, boolean as 0/1)
/// * `TimeFrames` - Duration data (time spans, intervals)
#[derive(Debug, Clone, PartialEq)]
pub enum DrawableData {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

/// Represents a goal that has been processed and is ready for visualization.
///
/// This enum eliminates the need for renderers to parse complex goal JSON.
/// Goals are converted from their storage format into the same type system
/// as DrawableData for consistent rendering.
///
/// # Variants
/// * `Integer` - Numeric goals (target counts, amounts)
/// * `TimeFrames` - Duration goals (target time spans)
#[derive(Debug, Clone, PartialEq)]
pub enum DrawableGoal {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

/// Represents a single data point in a plot.
///
/// Each data point corresponds to one date in the plot's time series.
/// The point includes its value, visualization color, and optional goal for comparison.
///
/// # Fields
/// * `date` - The date this data point represents
/// * `value` - The actual data value (Integer or TimeFrames). `None` means no value was
///   logged for this day; a `DataPoint` may still exist if there is a goal to display.
/// * `color` - RGB color for rendering this point (typically goal-achievement based)
/// * `goal` - Optional goal value for comparison/visualization
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: Option<DrawableData>,
    pub color: [u8; 3],
    pub goal: Option<DrawableGoal>,
}

impl DataPoint {
    pub fn new(
        date: NaiveDate,
        value: Option<DrawableData>,
        color: [u8; 3],
        goal: Option<DrawableGoal>,
    ) -> Self {
        Self {
            date,
            value,
            color,
            goal,
        }
    }
}

/// Holds all the necessary, processed data for rendering a plot.
///
/// This is the central domain object for stateless plotting. It contains everything
/// a renderer needs to produce a visualization, without requiring any external
/// configuration or state. This struct is renderer-agnostic and represents the pure
/// semantic description of "what to draw" (not "how to draw it").
///
/// # Architecture Role
/// * Built by the **application layer** from repository data + business logic
/// * Consumed by the **renderer adapter** (infrastructure) to produce images
/// * Returned by the **web API** for raw plot data endpoints
///
/// # Fields
/// * `title` - Human-readable title for the plot
/// * `points` - Time-series data points with values, colors, and goals
/// * `date_range` - The date range this plot covers
/// * `plot_type` - The type of visualization (bar chart, line chart, etc.)
/// * `y_axis` - Pre-computed Y-axis configuration (min, max, labels)
#[derive(Debug, Clone, PartialEq)]
pub struct PlotData {
    pub title: String,
    pub points: Vec<DataPoint>,
    pub date_range: DateRange,
    pub plot_type: PlotType,
    pub y_axis: YAxis,
}

impl PlotData {
    pub fn new(
        title: String,
        points: Vec<DataPoint>,
        date_range: DateRange,
        plot_type: PlotType,
        y_axis: YAxis,
    ) -> Self {
        Self {
            title,
            points,
            date_range,
            plot_type,
            y_axis,
        }
    }
}
