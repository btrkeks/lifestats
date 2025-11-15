use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;

use crate::cqrs::Query;
use crate::dto::goal_performance::GoalPerformanceDto;

/// Query to analyze goal performance over a date range
///
/// This query calculates:
/// - Overall achievement score (average percentage across the range)
/// - Trend indicator based on EWMA analysis (improving, declining, stable, or insufficient data)
#[derive(Debug, Clone)]
pub struct GetGoalPerformanceQuery {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
}

impl Query for GetGoalPerformanceQuery {
    type Result = GoalPerformanceDto;
}
