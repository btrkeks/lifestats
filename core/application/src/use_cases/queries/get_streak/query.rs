use crate::cqrs::Query;
use lifestats_core::shared::ids::StatisticId;

/// Query to compute the current streak for a statistic
///
/// A streak represents consecutive days where a goal was achieved.
/// This query contains complex business logic to traverse historical
/// data and evaluate goal achievement.
pub struct GetStreakQuery {
    pub statistic_id: StatisticId,
}

impl Query for GetStreakQuery {
    type Result = u32;
}
