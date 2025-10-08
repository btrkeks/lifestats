use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

/// Query to retrieve a goal override for a specific date.
pub struct GetDailyGoalQuery {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

impl Query for GetDailyGoalQuery {
    type Result = Option<serde_json::Value>;
}
