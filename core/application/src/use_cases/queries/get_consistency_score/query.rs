use lifestats_core::shared::ids::StatisticId;

use crate::cqrs::Query;

/// Query to get consistency score for a statistic over the last N days.
///
/// The consistency score indicates what percentage of days (with goals) were achieved
/// in the specified lookback period.
#[derive(Debug, Clone)]
pub struct GetConsistencyScoreQuery {
    pub statistic_id: StatisticId,
    /// Number of calendar days to evaluate, including today. Range: [1..=366]
    pub days: u32,
}

impl Query for GetConsistencyScoreQuery {
    type Result = ConsistencyScoreResult;
}

/// Result of a consistency score query
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyScoreResult {
    pub statistic_id: StatisticId,
    /// Number of days in the range that had a goal (excludes days with no goal)
    pub days_with_goal: u32,
    /// Number of days (with goal) where the goal was achieved
    pub days_achieved: u32,
}
