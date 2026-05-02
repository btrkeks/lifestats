use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::ProgressRepository;
use lifestats_core::progress::{NewProgressItem, ProgressMetadata, ProgressTarget};

use super::command::CreateProgressItemCommand;

pub struct CreateProgressItemCommandHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl CreateProgressItemCommandHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl CommandHandler<CreateProgressItemCommand> for CreateProgressItemCommandHandler {
    fn handle(
        &self,
        command: CreateProgressItemCommand,
    ) -> ApplicationResult<lifestats_core::progress::ProgressItem> {
        let metadata = ProgressMetadata::new(command.name, command.category, command.tags)?;
        let target = ProgressTarget::new(command.target_value, command.unit)?;
        let item = NewProgressItem::new(metadata, target, command.created_at);

        Ok(self.repository.create_item(item)?)
    }
}
