use crate::cqrs::Query;
use lifestats_core::shared::ids::StatisticId;

/// Query to retrieve the default goal for a statistic.
pub struct GetDefaultGoalQuery {
    pub statistic_id: StatisticId,
}

impl Query for GetDefaultGoalQuery {
    type Result = Option<serde_json::Value>;
}
