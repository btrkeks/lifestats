use crate::plot::axis::YAxis;
use crate::plot::specification::PlotType;
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::time_frame_list::TimeFrameList;

/// Represents data that has been processed and is ready for visualization
/// This enum eliminates the need for renderers to parse generic JSON
#[derive(Debug, Clone, PartialEq)]
pub enum DrawableData {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

/// Represents a goal that has been processed and is ready for visualization
/// This enum eliminates the need for renderers to parse complex goal JSON
#[derive(Debug, Clone, PartialEq)]
pub enum DrawableGoal {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

/// Represents a single data point in a plot
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: DrawableData,
    pub color: [u8; 3],
    pub goal: Option<DrawableGoal>,
}

impl DataPoint {
    pub fn new(
        date: NaiveDate,
        value: DrawableData,
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

/// Holds all the necessary, processed data for rendering a plot
/// This is renderer-agnostic and represents the pure data to be visualized
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
