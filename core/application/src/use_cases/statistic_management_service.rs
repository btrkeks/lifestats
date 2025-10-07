use std::sync::Arc;

use crate::error::{ApplicationError, ApplicationResult};
use chrono::Local;
use events::event_port::EventPublisher;
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

/// Service for managing statistic metadata (create, update, delete operations)
///
/// This service encapsulates the business logic for CRUD operations on statistics
/// themselves, separate from logging values or querying data.
#[derive(Clone)]
pub struct StatisticManagementService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> StatisticManagementService<R, E>
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

    /// Create a new statistic
    ///
    /// Constructs the appropriate AnyStatistic variant based on the statistic_type string,
    /// saves it to the repository, and returns the created statistic with its assigned ID.
    pub fn create_statistic(
        &self,
        command: CreateStatisticCommand,
    ) -> ApplicationResult<AnyStatistic> {
        // Create metadata with placeholder ID (0) - repository will assign the real ID
        let metadata = StatisticMetadata::new(StatisticId(0), command.category, command.name);

        // Get current date for default_goal_start
        let today = Local::now().date_naive();

        // Construct the appropriate statistic type based on statistic_type string
        let statistic = match command.statistic_type.as_str() {
            "IntegerAmount" => {
                AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, today))
            }
            "Habit" => AnyStatistic::Habit(HabitStatistic::new(metadata, None, today)),
            "PointInTime" => {
                AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, today))
            }
            "TimeSpans" => AnyStatistic::TimeSpans(TimeSpansStatistic::new(metadata, None, today)),
            "Summing" => {
                // For Summing statistics, we need source_statistic_ids
                let source_ids = command.source_statistic_ids.ok_or_else(|| {
                    ApplicationError::Infrastructure(anyhow::anyhow!(
                        "source_statistic_ids required for Summing statistics"
                    ))
                })?;
                AnyStatistic::Summing(SummingStatistic::new(metadata, source_ids, None, today))
            }
            "TimeDifference" => {
                // For TimeDifference statistics, we need two source statistic IDs
                let source_ids = command.source_statistic_ids.ok_or_else(|| {
                    ApplicationError::Infrastructure(anyhow::anyhow!(
                        "source_statistic_ids required for TimeDifference statistics"
                    ))
                })?;
                if source_ids.len() != 2 {
                    return Err(ApplicationError::Infrastructure(anyhow::anyhow!(
                        "TimeDifference statistics require exactly 2 source_statistic_ids"
                    )));
                }
                AnyStatistic::TimeDifference(TimeDifferenceStatistic::new(
                    metadata,
                    source_ids[0],
                    source_ids[1],
                    None,
                    today,
                ))
            }
            _ => {
                return Err(ApplicationError::Infrastructure(anyhow::anyhow!(
                    "Unknown statistic_type: {}",
                    command.statistic_type
                )));
            }
        };

        // Save to repository (repository assigns the ID)
        let created_statistic = self.statistic_repository.create_metadata(statistic)?;

        // Note: No specific event for metadata changes in current Event enum
        // Could publish ValueUpdated if needed for UI refresh

        Ok(created_statistic)
    }

    /// Update an existing statistic's metadata
    ///
    /// Note: For MVP, update functionality requires reconstructing the entire statistic
    /// with new metadata. This is complex due to the immutable nature of the domain model.
    /// Consider adding helper methods to domain types for easier updates in the future.
    pub fn update_statistic(&self, _command: UpdateStatisticCommand) -> ApplicationResult<()> {
        // TODO: Implement update by reconstructing AnyStatistic with new metadata
        // This requires pattern matching on each variant and calling the appropriate constructor
        Err(ApplicationError::Infrastructure(anyhow::anyhow!(
            "Update operation not yet implemented - requires domain model enhancements"
        )))
    }

    /// Delete a statistic
    ///
    /// Removes the statistic and all its associated data from the repository.
    pub fn delete_statistic(&self, command: DeleteStatisticCommand) -> ApplicationResult<()> {
        // Verify the statistic exists before deleting
        let _ = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // Delete from repository
        self.statistic_repository
            .delete_metadata(command.statistic_id)?;

        // Note: No specific event for metadata changes in current Event enum

        Ok(())
    }
}

/// Command for creating a new statistic
pub struct CreateStatisticCommand {
    pub name: String,
    pub category: String,
    pub statistic_type: String, // "IntegerAmount", "Habit", "PointInTime", "TimeSpans", "Summing", "TimeDifference"
    /// Required for Summing and TimeDifference statistics
    pub source_statistic_ids: Option<Vec<StatisticId>>,
}

/// Command for updating an existing statistic
pub struct UpdateStatisticCommand {
    pub statistic_id: StatisticId,
    pub name: Option<String>,
    pub category: Option<String>,
    // Note: statistic_type cannot be changed after creation
    // Note: source_statistic_ids for derived stats would require special handling
}

/// Command for deleting a statistic
pub struct DeleteStatisticCommand {
    pub statistic_id: StatisticId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::event_port::MockEventPublisher;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

    #[test]
    fn test_create_integer_amount_statistic() {
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        // Expect create_metadata to be called and return a statistic with ID 1
        repo.expect_create_metadata().times(1).returning(|stat| {
            // The mock simulates the DB assigning an ID and returning the new state.
            // It does this by reconstructing the object, respecting its invariants.
            let metadata = stat.get_metadata();
            let new_metadata = StatisticMetadata::new(
                StatisticId(1), // The new ID assigned by the database
                metadata.category().to_string(),
                metadata.name().to_string(),
            );

            // Reconstruct the same AnyStatistic variant with the new metadata
            let created = match stat {
                AnyStatistic::IntegerAmount(_) => AnyStatistic::IntegerAmount(
                    IntegerAmountStatistic::new(new_metadata, None, Local::now().date_naive()),
                ),
                // ... add other arms as needed for tests ...
                _ => panic!("Test mock not configured for this statistic type"),
            };
            Ok(created)
        });

        // Note: No event published for metadata operations (no MetadataCreated event exists)

        let service = StatisticManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = CreateStatisticCommand {
            name: "Push Ups".to_string(),
            category: "Fitness".to_string(),
            statistic_type: "IntegerAmount".to_string(),
            source_statistic_ids: None,
        };

        let result = service.create_statistic(command);
        assert!(result.is_ok());
        let statistic = result.unwrap();
        assert_eq!(statistic.get_metadata().id(), StatisticId(1));
    }

    #[test]
    fn test_create_unknown_statistic_type_fails() {
        let repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();
        let service = StatisticManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = CreateStatisticCommand {
            name: "Test".to_string(),
            category: "Test".to_string(),
            statistic_type: "UnknownType".to_string(),
            source_statistic_ids: None,
        };

        let result = service.create_statistic(command);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_statistic() {
        let mut repo = MockStatisticRepository::new();
        let event_publisher = MockEventPublisher::new();

        let stat_id = StatisticId(42);
        let test_date = Local::now().date_naive();
        let metadata = StatisticMetadata::new(stat_id, "Test".into(), "Test Stat".into());
        let stat =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, test_date));

        // Expect find to verify existence
        repo.expect_find_metadata_by_id()
            .times(1)
            .withf(move |id| *id == stat_id)
            .return_once(move |_| Ok(stat));

        // Expect delete to be called
        repo.expect_delete_metadata()
            .times(1)
            .withf(move |id| *id == stat_id)
            .return_once(|_| Ok(()));

        // Note: No event published for metadata operations (no MetadataDeleted event exists)

        let service = StatisticManagementService::new(Arc::new(repo), Arc::new(event_publisher));

        let command = DeleteStatisticCommand {
            statistic_id: stat_id,
        };

        let result = service.delete_statistic(command);
        assert!(result.is_ok());
    }
}
