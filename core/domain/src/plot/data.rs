use crate::plot::specification::PlotType;
use crate::shared::date_range::DateRange;
use chrono::NaiveDate;
use serde_json::Value;

/// Represents a single data point in a plot
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: Value,
    pub goal: Option<Value>,
}

impl DataPoint {
    pub fn new(date: NaiveDate, value: Value, goal: Option<Value>) -> Self {
        Self { date, value, goal }
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
}

impl PlotData {
    pub fn new(
        title: String,
        points: Vec<DataPoint>,
        date_range: DateRange,
        plot_type: PlotType,
    ) -> Self {
        Self {
            title,
            points,
            date_range,
            plot_type,
        }
    }
}
