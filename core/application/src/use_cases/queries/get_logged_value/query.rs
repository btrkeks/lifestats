use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

/// Query to retrieve a logged value for a specific statistic and date.
pub struct GetLoggedValueQuery {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

impl Query for GetLoggedValueQuery {
    type Result = Option<serde_json::Value>;
}
