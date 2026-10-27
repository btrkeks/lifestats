use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

/// Query to retrieve the current default goal for a statistic.
pub struct GetDefaultGoalQuery {
    pub statistic_id: StatisticId,
}

/// The latest default goal and the first day it applies to.
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultGoalResult {
    pub goal_data: serde_json::Value,
    pub valid_since: NaiveDate,
}

impl Query for GetDefaultGoalQuery {
    /// `None` when the statistic has no default goal (or it has been removed).
    type Result = Option<DefaultGoalResult>;
}
