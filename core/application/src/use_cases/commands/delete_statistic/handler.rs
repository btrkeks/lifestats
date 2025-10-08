use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use crate::use_cases::commands::delete_statistic::command::DeleteStatisticCommand;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

/// Handler responsible for deleting statistics.
pub struct DeleteStatisticCommandHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<R>,
}

impl<R> DeleteStatisticCommandHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<R>) -> Self {
        Self {
            statistic_repository,
        }
    }
}

impl<R> CommandHandler<DeleteStatisticCommand> for DeleteStatisticCommandHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, command: DeleteStatisticCommand) -> ApplicationResult<()> {
        // Ensure the statistic exists before attempting deletion
        let _ = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        self.statistic_repository
            .delete_metadata(command.statistic_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_delete_statistic_happy_path() {
        let mut repo = MockStatisticRepository::new();

        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(StatisticId(1), "cat".into(), "name".into()),
            None,
            Local::now().date_naive(),
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .returning(move |_| Ok(statistic.clone()));

        repo.expect_delete_metadata().times(1).returning(|_| Ok(()));

        let handler = DeleteStatisticCommandHandler::new(Arc::new(repo));
        let command = DeleteStatisticCommand {
            statistic_id: StatisticId(1),
        };

        let result = handler.handle(command);
        assert!(result.is_ok());
    }
}
