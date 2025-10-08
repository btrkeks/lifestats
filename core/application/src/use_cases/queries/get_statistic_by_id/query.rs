use crate::cqrs::Query;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;

/// Query to retrieve a single statistic by its ID
pub struct GetStatisticByIdQuery {
    pub statistic_id: StatisticId,
}

impl Query for GetStatisticByIdQuery {
    type Result = AnyStatistic;
}
