use chrono::{DateTime, NaiveDate, Utc};
use lifestats_core::progress::ports::progress_repository::SaveCheckpointResult;
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Command;

pub struct LogProgressCheckpointCommand {
    pub item_id: ProgressItemId,
    pub recorded_at: DateTime<Utc>,
    pub effective_date: NaiveDate,
    pub position: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
}

impl Command for LogProgressCheckpointCommand {
    type Result = SaveCheckpointResult;
}
