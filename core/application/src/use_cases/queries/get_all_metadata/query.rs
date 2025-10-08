use crate::cqrs::Query;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

/// Query object to retrieve all statistic metadata entries.
pub struct GetAllMetadataQuery;

impl Query for GetAllMetadataQuery {
    type Result = Vec<StatisticMetadata>;
}
