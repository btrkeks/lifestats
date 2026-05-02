use std::collections::HashMap;

use chrono::{DateTime, Utc};
use lifestats_core::progress::{ProgressItem, ProgressUnit};

use crate::cqrs::Command;

pub struct CreateProgressItemCommand {
    pub name: String,
    pub category: String,
    pub target_value: i64,
    pub unit: ProgressUnit,
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl Command for CreateProgressItemCommand {
    type Result = ProgressItem;
}
