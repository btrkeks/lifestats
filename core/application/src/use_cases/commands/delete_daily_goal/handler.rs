use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use crate::use_cases::commands::delete_daily_goal::command::DeleteDailyGoalCommand;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

/// Handler responsible for deleting a daily goal override.
pub struct DeleteDailyGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> DeleteDailyGoalCommandHandler<R, E>
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

impl<R, E> CommandHandler<DeleteDailyGoalCommand> for DeleteDailyGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: DeleteDailyGoalCommand) -> ApplicationResult<()> {
        self.statistic_repository
            .delete_goal(command.statistic_id, command.date)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

    #[test]
    fn test_delete_daily_goal() {
        let mut repo = MockStatisticRepository::new();
        let mut publisher = MockEventPublisher::new();

        let stat_id = StatisticId(8);
        let date = Local::now().date_naive();

        repo.expect_delete_goal()
            .times(1)
            .withf(move |id, saved_date| *id == stat_id && *saved_date == date)
            .return_once(|_, _| Ok(()));

        publisher
            .expect_publish_event_async()
            .times(1)
            .withf(move |event| matches!(event, Event::GoalUpdated(id) if *id == stat_id))
            .return_const(());

        let handler = DeleteDailyGoalCommandHandler::new(Arc::new(repo), Arc::new(publisher));
        let command = DeleteDailyGoalCommand {
            statistic_id: stat_id,
            date,
        };

        assert!(handler.handle(command).is_ok());
    }
}
