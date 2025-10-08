use crate::cqrs::Query;
use lifestats_core::statistic::any_statistic::AnyStatistic;

/// Query to retrieve all statistics metadata
///
/// This is a simple read-only query that returns all statistic definitions
/// in the system without any filtering or transformation.
pub struct GetAllStatisticsQuery;

impl Query for GetAllStatisticsQuery {
    type Result = Vec<AnyStatistic>;
}
