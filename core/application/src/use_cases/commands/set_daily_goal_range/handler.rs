use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use events::event_port::{Event, EventPublisher};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::super::set_daily_goal::{parse_goal_payload_to_json, DailyGoalOverride};
use super::command::SetDailyGoalRangeCommand;

/// Handler for SetDailyGoalRangeCommand
pub struct SetDailyGoalRangeCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> SetDailyGoalRangeCommandHandler<R, E>
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

impl<R, E> CommandHandler<SetDailyGoalRangeCommand> for SetDailyGoalRangeCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: SetDailyGoalRangeCommand) -> ApplicationResult<()> {
        let statistic = self.repository.find_metadata_by_id(command.statistic_id)?;

        let goal_json = match command.override_kind {
            DailyGoalOverride::Goal(goal_payload) => {
                Some(parse_goal_payload_to_json(goal_payload, statistic)?)
            }
            DailyGoalOverride::Suppressed => None,
        };

        self.repository
            .save_goals_in_range(command.statistic_id, command.date_range, goal_json)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::commands::set_daily_goal::GoalPayload;
    use chrono::{NaiveDate, NaiveTime};
    use events::event_port::MockEventPublisher;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn bedtime(id: StatisticId) -> AnyStatistic {
        let metadata = StatisticMetadata::new(id, "health".into(), "bedtime".into());
        AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, date(1)))
    }

    fn command(override_kind: DailyGoalOverride) -> SetDailyGoalRangeCommand {
        SetDailyGoalRangeCommand {
            statistic_id: StatisticId(20),
            date_range: DateRange::new(date(12), date(21)).unwrap(),
            override_kind,
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
    fn test_sets_the_goal_on_every_day_of_the_range() {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_metadata_by_id()
            .return_once(|id| Ok(bedtime(id)));
        repo.expect_save_goals_in_range()
            .times(1)
            .withf(|id, range, goal| {
                *id == StatisticId(20)
                    && range.start() == date(12)
                    && range.end() == date(21)
                    && goal.as_ref().and_then(|g| g.get("target"))
                        == Some(&serde_json::json!("22:05:00"))
            })
            .return_once(|_, _, _| Ok(()));

        let handler = SetDailyGoalRangeCommandHandler::new(
            Arc::new(repo),
            Arc::new(publisher_expecting_one_event()),
        );
        let deadline = NaiveTime::from_hms_opt(22, 5, 0).unwrap();

        let result = handler.handle(command(DailyGoalOverride::Goal(
            GoalPayload::FinishedInTime { deadline },
        )));

        assert!(result.is_ok());
    }

    #[test]
    fn test_suppresses_every_day_of_the_range() {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_metadata_by_id()
            .return_once(|id| Ok(bedtime(id)));
        repo.expect_save_goals_in_range()
            .times(1)
            .withf(|_, _, goal| goal.is_none())
            .return_once(|_, _, _| Ok(()));

        let handler = SetDailyGoalRangeCommandHandler::new(
            Arc::new(repo),
            Arc::new(publisher_expecting_one_event()),
        );

        assert!(handler
            .handle(command(DailyGoalOverride::Suppressed))
            .is_ok());
    }

    #[test]
    fn test_rejects_a_goal_of_the_wrong_type_without_writing() {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_metadata_by_id()
            .return_once(|id| Ok(bedtime(id)));
        repo.expect_save_goals_in_range().never();

        let handler = SetDailyGoalRangeCommandHandler::new(
            Arc::new(repo),
            Arc::new(MockEventPublisher::new()),
        );

        let result = handler.handle(command(DailyGoalOverride::Goal(
            GoalPayload::IntegerAmount {
                target: 5,
                is_at_least: true,
            },
        )));

        assert!(result.is_err());
    }
}
