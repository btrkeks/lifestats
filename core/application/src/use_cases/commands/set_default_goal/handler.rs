use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use chrono::Local;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::super::set_daily_goal::parse_goal_payload_to_json;
use super::command::SetDefaultGoalCommand;

/// Handler for SetDefaultGoalCommand
///
/// Applies the goal from `valid_since` onwards in the statistic's default goal
/// timeline, so days before it keep the goal they had. Daily overrides are untouched.
pub struct SetDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> SetDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R, E> CommandHandler<SetDefaultGoalCommand> for SetDefaultGoalCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: SetDefaultGoalCommand) -> ApplicationResult<()> {
        let statistic = self.repository.find_metadata_by_id(command.statistic_id)?;
        let goal_json = parse_goal_payload_to_json(command.goal, statistic)?;
        let valid_since = command
            .valid_since
            .unwrap_or_else(|| Local::now().date_naive());

        let mut timeline = self
            .repository
            .find_default_goal_timeline(command.statistic_id)?;
        timeline.set_from(valid_since, Some(goal_json));
        self.repository
            .save_default_goal_timeline(command.statistic_id, &timeline)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::commands::set_daily_goal::GoalPayload;
    use chrono::NaiveDate;
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::goals::default_goal_timeline::{
        DefaultGoalSegment, DefaultGoalTimeline,
    };
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn integer_statistic(id: StatisticId) -> AnyStatistic {
        let metadata = StatisticMetadata::new(id, "test".into(), "test_integer".into());
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, date(1, 1)))
    }

    fn command(id: StatisticId, valid_since: Option<NaiveDate>) -> SetDefaultGoalCommand {
        SetDefaultGoalCommand {
            statistic_id: id,
            goal: GoalPayload::IntegerAmount {
                target: 450,
                is_at_least: true,
            },
            valid_since,
        }
    }

    fn publisher_expecting_one_event() -> MockEventPublisher {
        let mut publisher = MockEventPublisher::new();
        publisher
            .expect_publish_event_async()
            .times(1)
            .return_const(());
        publisher
    }

    #[test]
    fn test_set_default_goal_keeps_the_old_goal_before_valid_since() {
        let stat_id = StatisticId(1);
        let mut repo = MockStatisticRepository::new();
        let stat = integer_statistic(stat_id);
        repo.expect_find_metadata_by_id()
            .return_once(move |_| Ok(stat));
        repo.expect_find_default_goal_timeline().return_once(|_| {
            Ok(DefaultGoalTimeline::new(vec![DefaultGoalSegment::new(
                date(9, 9),
                Some(json!({"target_amount": 400, "is_at_least": true})),
            )]))
        });
        repo.expect_save_default_goal_timeline()
            .times(1)
            .withf(move |id, timeline| {
                *id == stat_id
                    && timeline.goal_on(date(10, 11))
                        == Some(&json!({"target_amount": 400, "is_at_least": true}))
                    && timeline.goal_on(date(10, 12))
                        == Some(&json!({"target_amount": 450, "is_at_least": true}))
            })
            .return_once(|_, _| Ok(()));

        let handler = SetDefaultGoalCommandHandler::new(
            Arc::new(repo),
            Arc::new(publisher_expecting_one_event()),
        );

        assert!(handler.handle(command(stat_id, Some(date(10, 12)))).is_ok());
    }

    #[test]
    fn test_set_default_goal_without_valid_since_starts_today() {
        let stat_id = StatisticId(1);
        let today = Local::now().date_naive();
        let mut repo = MockStatisticRepository::new();
        let stat = integer_statistic(stat_id);
        repo.expect_find_metadata_by_id()
            .return_once(move |_| Ok(stat));
        repo.expect_find_default_goal_timeline()
            .return_once(|_| Ok(DefaultGoalTimeline::default()));
        repo.expect_save_default_goal_timeline()
            .times(1)
            .withf(move |_, timeline| {
                timeline.latest().map(|segment| segment.valid_since) == Some(today)
            })
            .return_once(|_, _| Ok(()));

        let handler = SetDefaultGoalCommandHandler::new(
            Arc::new(repo),
            Arc::new(publisher_expecting_one_event()),
        );

        assert!(handler.handle(command(stat_id, None)).is_ok());
    }

    #[test]
    fn test_set_default_goal_rejects_a_goal_of_the_wrong_type() {
        let stat_id = StatisticId(1);
        let mut repo = MockStatisticRepository::new();
        let metadata = StatisticMetadata::new(stat_id, "test".into(), "habit".into());
        let stat = AnyStatistic::Habit(HabitStatistic::new(metadata, None, date(1, 1)));
        repo.expect_find_metadata_by_id()
            .return_once(move |_| Ok(stat));
        repo.expect_save_default_goal_timeline().never();

        let handler =
            SetDefaultGoalCommandHandler::new(Arc::new(repo), Arc::new(MockEventPublisher::new()));

        assert!(handler.handle(command(stat_id, None)).is_err());
    }
}
