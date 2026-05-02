use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use lifestats_core::progress::ports::progress_repository::{
    ProgressCheckpointRange, ProgressRepository,
};
use lifestats_core::shared::ports::error::RepositoryError;

use super::command::UpdateProgressTargetCommand;

pub struct UpdateProgressTargetCommandHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl UpdateProgressTargetCommandHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl CommandHandler<UpdateProgressTargetCommand> for UpdateProgressTargetCommandHandler {
    fn handle(&self, command: UpdateProgressTargetCommand) -> ApplicationResult<()> {
        let item = self
            .repository
            .find_item_by_id(command.item_id)?
            .ok_or(RepositoryError::NotFound)?;

        let checkpoints = self
            .repository
            .find_checkpoints(command.item_id, ProgressCheckpointRange::default())?;
        let furthest = checkpoints
            .iter()
            .map(|checkpoint| checkpoint.position())
            .max()
            .unwrap_or(0);
        if command.target_value < furthest {
            return Err(ApplicationError::InvalidInput(format!(
                "progress target {} cannot be below furthest checkpoint {}",
                command.target_value, furthest
            )));
        }

        let updated = item.with_target_value(command.target_value, command.updated_at)?;
        self.repository.update_item(updated)?;
        Ok(())
    }
}
