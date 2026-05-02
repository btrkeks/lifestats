use lifestats_core::progress::{ProgressCheckpoint, ProgressItem};
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Query;

pub struct GetProgressSummaryQuery {
    pub item_id: ProgressItemId,
}

impl Query for GetProgressSummaryQuery {
    type Result = ProgressSummary;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSummary {
    pub item: ProgressItem,
    pub latest_checkpoint: Option<ProgressCheckpoint>,
    pub latest_position: Option<i64>,
    pub furthest_position: Option<i64>,
    pub percent_latest: Option<f64>,
    pub percent_furthest: Option<f64>,
    pub is_complete: bool,
}
