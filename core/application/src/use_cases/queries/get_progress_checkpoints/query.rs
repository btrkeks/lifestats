use chrono::{DateTime, Utc};
use lifestats_core::progress::ProgressCheckpoint;
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Query;

pub struct GetProgressCheckpointsQuery {
    pub item_id: ProgressItemId,
    pub recorded_at_start: Option<DateTime<Utc>>,
    pub recorded_at_end: Option<DateTime<Utc>>,
}

impl Query for GetProgressCheckpointsQuery {
    type Result = Vec<ProgressCheckpoint>;
}
