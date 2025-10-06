// domain/src/plot/data.rs
// NEW FILE

use crate::value_objects::Percentage;
use chrono::NaiveDate;

/// Domain representation of data to be plotted.
///
/// This structure contains the MEANING of what should be shown,
/// not HOW it should be drawn. It's the domain's way of saying
/// "here's the statistical data that needs visualization."
///
/// Note: This is separate from rendering concerns like pixel coordinates,
/// colors, or specific chart libraries.
#[derive(Debug, Clone)]
pub struct PlotData {
    /// The actual data points to plot
    pub points: Vec<DataPoint>,

    /// Optional goal line to display
    pub goal: Option<GoalLine>,

    /// Plot metadata (title, axis labels, etc.)
    pub metadata: PlotMetadata,

    /// The type of visualization appropriate for this data
    pub visualization_type: VisualizationType,
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub date: NaiveDate,
    pub value: f64,
    /// Optional: percentage of goal achieved (domain calculation)
    pub goal_percentage: Option<Percentage>,
}

#[derive(Debug, Clone)]
pub struct GoalLine {
    pub value: f64,
    pub label: String,
}

/// What KIND of visualization the domain thinks is appropriate.
/// The infrastructure decides the specific rendering, but domain
/// knows which type makes semantic sense for the statistic.
#[derive(Debug, Clone, Copy)]
pub enum VisualizationType {
    /// For discrete events over time (habits, counts)
    BarChart,

    /// For continuous trends (time amounts, cumulative data)
    LineChart,

    /// For showing progress toward a goal (percentages)
    CircularProgress,

    /// For showing time periods (when activities occurred)
    TimeFrames,
}

impl PlotData {
    /// Domain logic: construct plot data from statistic logs and goals.
    ///
    /// This is where business rules live:
    /// - How to aggregate data
    /// - How to calculate goal percentages
    /// - Which visualization type is appropriate
    pub fn from_statistic_logs(
        logs: Vec<LogEntry>,
        goal: Option<Goal>,
        date_range: DateRange,
    ) -> Result<Self> {
        // TODO: Implement domain logic for data preparation
        todo!("Domain business logic goes here")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_data_includes_goal_percentage() {
        // Test domain logic without any rendering
        let logs = vec![/* sample logs */];
        let goal = Some(/* sample goal */);

        let plot_data =
            PlotData::from_statistic_logs(logs, goal, DateRange::last_30_days()).unwrap();

        // Assert business rules are applied correctly
        assert!(plot_data.points[0].goal_percentage.is_some());
        assert_eq!(plot_data.visualization_type, VisualizationType::BarChart);
    }
}
