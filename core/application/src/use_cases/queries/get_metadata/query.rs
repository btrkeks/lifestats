use crate::cqrs::Query;
use crate::dto::StatisticSummary;
use lifestats_core::shared::ids::StatisticId;

/// Query to retrieve metadata with type information for a specific statistic.
pub struct GetMetadataQuery {
    pub statistic_id: StatisticId,
}

impl Query for GetMetadataQuery {
    type Result = StatisticSummary;
}
