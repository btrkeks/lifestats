use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::progress::{NewProgressCheckpoint, NewProgressItem, ProgressCheckpoint, ProgressItem};
use crate::shared::ids::ProgressItemId;
use crate::shared::ports::error::RepositoryError;

#[derive(Debug, Clone, Copy, Default)]
pub struct ProgressCheckpointRange {
    pub recorded_at_start: Option<DateTime<Utc>>,
    pub recorded_at_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveCheckpointResult {
    Inserted(ProgressCheckpoint),
    Duplicate(ProgressCheckpoint),
}

#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait ProgressRepository {
    fn create_item(&self, item: NewProgressItem) -> Result<ProgressItem, RepositoryError>;

    fn update_item(&self, item: ProgressItem) -> Result<(), RepositoryError>;

    fn find_item_by_id(&self, id: ProgressItemId) -> Result<Option<ProgressItem>, RepositoryError>;

    fn find_all_items(&self) -> Result<Vec<ProgressItem>, RepositoryError>;

    fn find_items_by_tags(
        &self,
        tags: &HashMap<String, String>,
    ) -> Result<Vec<ProgressItem>, RepositoryError>;

    fn save_checkpoint(
        &self,
        checkpoint: NewProgressCheckpoint,
    ) -> Result<SaveCheckpointResult, RepositoryError>;

    fn find_latest_checkpoint(
        &self,
        item_id: ProgressItemId,
    ) -> Result<Option<ProgressCheckpoint>, RepositoryError>;

    fn find_checkpoints(
        &self,
        item_id: ProgressItemId,
        range: ProgressCheckpointRange,
    ) -> Result<Vec<ProgressCheckpoint>, RepositoryError>;
}
