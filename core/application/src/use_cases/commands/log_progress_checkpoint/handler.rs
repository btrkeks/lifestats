use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use lifestats_core::progress::ports::progress_repository::{
    ProgressRepository, SaveCheckpointResult,
};
use lifestats_core::progress::NewProgressCheckpoint;
use lifestats_core::shared::ports::error::RepositoryError;

use super::command::LogProgressCheckpointCommand;

pub struct LogProgressCheckpointCommandHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl LogProgressCheckpointCommandHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl CommandHandler<LogProgressCheckpointCommand> for LogProgressCheckpointCommandHandler {
    fn handle(
        &self,
        command: LogProgressCheckpointCommand,
    ) -> ApplicationResult<SaveCheckpointResult> {
        let item = self
            .repository
            .find_item_by_id(command.item_id)?
            .ok_or(RepositoryError::NotFound)?;

        if command.position > item.target().value() {
            return Err(ApplicationError::InvalidInput(format!(
                "progress checkpoint position {} cannot exceed target {}",
                command.position,
                item.target().value()
            )));
        }

        let checkpoint = NewProgressCheckpoint::new(
            command.item_id,
            command.recorded_at,
            command.effective_date,
            command.position,
            command.source,
            command.source_event_id,
            command.note,
            command.recorded_at,
        )?;

        Ok(self.repository.save_checkpoint(checkpoint)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use lifestats_core::progress::ports::progress_repository::MockProgressRepository;
    use lifestats_core::progress::{ProgressItem, ProgressMetadata, ProgressTarget, ProgressUnit};
    use lifestats_core::shared::ids::ProgressItemId;
    use std::collections::HashMap;

    fn item() -> ProgressItem {
        let now = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        ProgressItem::new(
            ProgressItemId(1),
            ProgressMetadata::new("Book".to_string(), "Reading".to_string(), HashMap::new())
                .unwrap(),
            ProgressTarget::new(100, ProgressUnit::Pages).unwrap(),
            now,
            now,
        )
        .unwrap()
    }

    #[test]
    fn rejects_checkpoint_above_target() {
        let mut repo = MockProgressRepository::new();
        repo.expect_find_item_by_id()
            .times(1)
            .returning(|_| Ok(Some(item())));
        repo.expect_save_checkpoint().times(0);

        let now = Utc.with_ymd_and_hms(2026, 5, 2, 20, 0, 0).unwrap();
        let handler = LogProgressCheckpointCommandHandler::new(Arc::new(repo));
        let result = handler.handle(LogProgressCheckpointCommand {
            item_id: ProgressItemId(1),
            recorded_at: now,
            effective_date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            position: 101,
            source: None,
            source_event_id: None,
            note: None,
        });

        assert!(matches!(result, Err(ApplicationError::InvalidInput(_))));
    }
}
