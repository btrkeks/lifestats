use std::sync::Arc;

use crate::cqrs::CommandHandler;
use crate::error::{ApplicationError, ApplicationResult};
use crate::use_cases::commands::create_statistic::command::{
    CreateStatisticCommand, StatisticTypeDto,
};
use chrono::Local;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
use lifestats_core::statistic::types::derived_statistics::time_difference::TimeDifferenceStatistic;
use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

use events::event_port::EventPublisher;

/// Handler responsible for creating new statistics from CreateStatisticCommand.
pub struct CreateStatisticCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> CreateStatisticCommandHandler<R, E>
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

    fn ensure_sources(
        source_statistic_ids: &Option<Vec<StatisticId>>,
    ) -> ApplicationResult<Vec<StatisticId>> {
        source_statistic_ids.clone().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "source_statistic_ids required for this statistic type".to_string(),
            )
        })
    }
}

impl<R, E> CommandHandler<CreateStatisticCommand> for CreateStatisticCommandHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn handle(&self, command: CreateStatisticCommand) -> ApplicationResult<AnyStatistic> {
        let CreateStatisticCommand {
            name,
            category,
            statistic_type,
            source_statistic_ids,
        } = command;

        // Create metadata with placeholder ID (repository will assign real ID)
        let metadata = StatisticMetadata::new(StatisticId(0), category, name).with_tag(
            "statistic_type".to_string(),
            statistic_type.as_str().to_string(),
        );

        // Get current date for default_goal_start
        let today = Local::now().date_naive();

        let statistic = match statistic_type {
            StatisticTypeDto::IntegerAmount => {
                AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, today))
            }
            StatisticTypeDto::Habit => {
                AnyStatistic::Habit(HabitStatistic::new(metadata, None, today))
            }
            StatisticTypeDto::PointInTime => {
                AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, today))
            }
            StatisticTypeDto::TimeSpans => {
                AnyStatistic::TimeSpans(TimeSpansStatistic::new(metadata, None, today))
            }
            StatisticTypeDto::Summing => {
                let source_ids = Self::ensure_sources(&source_statistic_ids)?;
                AnyStatistic::Summing(SummingStatistic::new(metadata, source_ids, None, today))
            }
            StatisticTypeDto::TimeDifference => {
                let source_ids = Self::ensure_sources(&source_statistic_ids)?;
                if source_ids.len() != 2 {
                    return Err(ApplicationError::InvalidInput(
                        "TimeDifference statistics require exactly 2 source_statistic_ids"
                            .to_string(),
                    ));
                }
                AnyStatistic::TimeDifference(TimeDifferenceStatistic::new(
                    metadata,
                    source_ids[0],
                    source_ids[1],
                    None,
                    today,
                ))
            }
        };

        let created_statistic = self.statistic_repository.create_metadata(statistic)?;

        // No events currently published for metadata changes, but keep publisher for parity
        let _ = &self.event_publisher;

        Ok(created_statistic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::event_port::MockEventPublisher;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use std::str::FromStr;

    fn clone_metadata_with_new_id(
        metadata: &StatisticMetadata,
        new_id: StatisticId,
    ) -> StatisticMetadata {
        let mut new_metadata = StatisticMetadata::new(
            new_id,
            metadata.category().to_string(),
            metadata.name().to_string(),
        );

        for (key, value) in metadata.tags() {
            new_metadata = new_metadata.with_tag(key.clone(), value.clone());
        }

        new_metadata
    }

    #[test]
    fn test_from_str_success() {
        assert!(matches!(
            StatisticTypeDto::from_str("Habit"),
            Ok(StatisticTypeDto::Habit)
        ));
        assert!(StatisticTypeDto::from_str("unknown").is_err());
    }

    #[test]
    fn test_create_integer_amount_statistic() {
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        repo.expect_create_metadata().times(1).returning(|stat| {
            let metadata = stat.get_metadata();
            let new_metadata = clone_metadata_with_new_id(metadata, StatisticId(1));
            let today = Local::now().date_naive();
            let created =
                AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(new_metadata, None, today));
            Ok(created)
        });

        let handler = CreateStatisticCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));
        let command = CreateStatisticCommand {
            name: "Push Ups".to_string(),
            category: "Fitness".to_string(),
            statistic_type: StatisticTypeDto::IntegerAmount,
            source_statistic_ids: None,
        };

        let result = handler.handle(command);
        assert!(result.is_ok());
        let stat = result.unwrap();
        assert_eq!(stat.get_metadata().id(), StatisticId(1));
        assert_eq!(stat.get_metadata().name(), "Push Ups");
        assert_eq!(
            stat.get_metadata()
                .get_tag("statistic_type")
                .map(String::as_str),
            Some("IntegerAmount")
        );
    }

    #[test]
    fn test_create_summing_requires_sources() {
        let repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();
        let handler = CreateStatisticCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = CreateStatisticCommand {
            name: "Total".into(),
            category: "Derived".into(),
            statistic_type: StatisticTypeDto::Summing,
            source_statistic_ids: None,
        };

        let result = handler.handle(command);
        assert!(matches!(result, Err(ApplicationError::InvalidInput(_))));
    }

    #[test]
    fn test_create_time_difference_requires_two_sources() {
        let repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();
        let handler = CreateStatisticCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));

        let command = CreateStatisticCommand {
            name: "Diff".into(),
            category: "Derived".into(),
            statistic_type: StatisticTypeDto::TimeDifference,
            source_statistic_ids: Some(vec![StatisticId(1)]),
        };

        let result = handler.handle(command);
        assert!(matches!(result, Err(ApplicationError::InvalidInput(_))));
    }

    #[test]
    fn test_create_time_difference_success() {
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        repo.expect_create_metadata().times(1).returning(|stat| {
            let metadata = stat.get_metadata();
            let new_metadata = clone_metadata_with_new_id(metadata, StatisticId(42));
            let today = Local::now().date_naive();
            let created = AnyStatistic::TimeDifference(TimeDifferenceStatistic::new(
                new_metadata,
                StatisticId(1),
                StatisticId(2),
                None,
                today,
            ));
            Ok(created)
        });

        let handler = CreateStatisticCommandHandler::new(Arc::new(repo), Arc::new(event_publisher));
        let command = CreateStatisticCommand {
            name: "Diff".into(),
            category: "Derived".into(),
            statistic_type: StatisticTypeDto::TimeDifference,
            source_statistic_ids: Some(vec![StatisticId(1), StatisticId(2)]),
        };

        let result = handler.handle(command).unwrap();
        assert_eq!(result.get_metadata().id(), StatisticId(42));
        assert_eq!(
            result
                .get_metadata()
                .get_tag("statistic_type")
                .map(String::as_str),
            Some("TimeDifference")
        );
    }
}
