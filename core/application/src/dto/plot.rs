/// Pure data transfer objects for plotting
///
/// These DTOs contain NO presentation logic - just data.
/// They represent plot data at the application boundary without
/// any knowledge of rendering, colors, or drawable formats.
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::percentage::Percentage;
use serde_json::Value;

/// Result of a plot data query
///
/// Contains all data needed to render a plot, but in pure form.
/// The adapter layer (web_api) is responsible for converting this
/// to presentation-specific formats (colors, drawable types, etc.)
#[derive(Debug, Clone)]
pub struct PlotDataResult {
    pub title: String,
    pub points: Vec<PlotPointDto>,
    pub date_range: DateRange,
}

/// A single point in a plot
///
/// Contains the raw data values as JSON, along with computed
/// goal achievement percentage. No colors or drawable types here!
#[derive(Debug, Clone)]
pub struct PlotPointDto {
    pub date: NaiveDate,
    /// The logged value as JSON (could be integer, bool, time, etc.)
    pub value: Value,
    /// The goal for this day as JSON (if any)
    pub goal: Option<Value>,
    /// Computed achievement percentage (None if no goal)
    pub percentage_achieved: Option<Percentage>,
}

impl PlotPointDto {
    pub fn new(
        date: NaiveDate,
        value: Value,
        goal: Option<Value>,
        percentage_achieved: Option<Percentage>,
    ) -> Self {
        Self {
            date,
            value,
            goal,
            percentage_achieved,
        }
    }
}
