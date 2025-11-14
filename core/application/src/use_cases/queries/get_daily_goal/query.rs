use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

/// Query to retrieve a goal override for a specific date.
pub struct GetDailyGoalQuery {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

impl Query for GetDailyGoalQuery {
    type Result = GoalQueryResult;
}
