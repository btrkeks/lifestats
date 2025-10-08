use crate::cqrs::Query;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

/// Query to fetch metadata for all TimeSpans statistics.
///
/// This query returns only the metadata needed by adapters to present
/// choices to users without loading goal or log data.
pub struct GetTimeSpansMetadataQuery;

impl Query for GetTimeSpansMetadataQuery {
    type Result = Vec<StatisticMetadata>;
}
