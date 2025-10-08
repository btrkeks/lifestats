use crate::cqrs::Query;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

/// Query to retrieve metadata for a specific statistic.
pub struct GetMetadataQuery {
    pub statistic_id: StatisticId,
}

impl Query for GetMetadataQuery {
    type Result = StatisticMetadata;
}
