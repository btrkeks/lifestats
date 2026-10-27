use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use crate::use_cases::commands::delete_default_goal::command::DeleteDefaultGoalCommand;
use chrono::Local;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

/// Handler responsible for removing a statistic's default goal from today on.
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
        let mut timeline = self
            .statistic_repository
            .find_default_goal_timeline(command.statistic_id)?;
        timeline.set_from(Local::now().date_naive(), None);
        self.statistic_repository
            .save_default_goal_timeline(command.statistic_id, &timeline)?;

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
    use lifestats_core::statistic::value_objects::goals::default_goal_timeline::{
        DefaultGoalSegment, DefaultGoalTimeline,
    };
    use serde_json::json;

    #[test]
    fn test_delete_default_goal_ends_it_today_and_keeps_the_past() {
        let mut repo = MockStatisticRepository::new();
        let mut publisher = MockEventPublisher::new();
        let stat_id = StatisticId(9);

        let yesterday = Local::now().date_naive().pred_opt().unwrap();
        repo.expect_find_default_goal_timeline()
            .times(1)
            .return_once(move |_| {
                Ok(DefaultGoalTimeline::new(vec![DefaultGoalSegment::new(
                    yesterday,
                    Some(json!({"target_amount": 5, "is_at_least": true})),
                )]))
            });
        repo.expect_save_default_goal_timeline()
            .times(1)
            .withf(move |id, timeline| {
                *id == stat_id
                    && timeline.goal_on(yesterday).is_some()
                    && timeline.goal_on(Local::now().date_naive()).is_none()
            })
            .return_once(|_, _| Ok(()));

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
