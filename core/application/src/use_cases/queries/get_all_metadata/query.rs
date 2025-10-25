use crate::cqrs::Query;
use crate::dto::StatisticSummary;

/// Query object to retrieve all statistic metadata entries with type information.
pub struct GetAllMetadataQuery;

impl Query for GetAllMetadataQuery {
    type Result = Vec<StatisticSummary>;
}
