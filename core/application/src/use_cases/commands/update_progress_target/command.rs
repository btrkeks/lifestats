use chrono::{DateTime, Utc};
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Command;

pub struct UpdateProgressTargetCommand {
    pub item_id: ProgressItemId,
    pub target_value: i64,
    pub updated_at: DateTime<Utc>,
}

impl Command for UpdateProgressTargetCommand {
    type Result = ();
}
