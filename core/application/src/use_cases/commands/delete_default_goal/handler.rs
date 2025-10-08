use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use crate::use_cases::commands::delete_default_goal::command::DeleteDefaultGoalCommand;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

/// Handler responsible for removing a statistic's default goal.
pub struct DeleteDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> DeleteDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(statistic_repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            statistic_repository,
            event_publisher,
        }
    }
}

impl<R, E> CommandHandler<DeleteDefaultGoalCommand> for DeleteDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: DeleteDefaultGoalCommand) -> ApplicationResult<()> {
        self.statistic_repository
            .delete_default_goal(command.statistic_id)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

    #[test]
    fn test_delete_default_goal() {
        let mut repo = MockStatisticRepository::new();
        let mut publisher = MockEventPublisher::new();
        let stat_id = StatisticId(9);

        repo.expect_delete_default_goal()
            .times(1)
            .withf(move |id| *id == stat_id)
            .return_once(|_| Ok(()));

        publisher
            .expect_publish_event_async()
            .times(1)
            .withf(move |event| matches!(event, Event::GoalUpdated(id) if *id == stat_id))
            .return_const(());

        let handler = DeleteDefaultGoalCommandHandler::new(Arc::new(repo), Arc::new(publisher));
        let command = DeleteDefaultGoalCommand {
            statistic_id: stat_id,
        };

        assert!(handler.handle(command).is_ok());
    }
}
