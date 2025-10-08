use std::collections::HashMap;
use std::sync::Arc;

use super::bundle_factory::BundleFactory;
use crate::error::{ApplicationError, ApplicationResult};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

#[cfg(test)]
use mockall::automock;

/// Trait defining the contract for integration service implementations.
///
/// This trait allows for dependency injection and testability.
#[cfg_attr(test, automock)]
pub trait IntegrationServiceTrait: Send + Sync {
    /// Provisions a bundle of statistics for the given source and context.
    ///
    /// This method is idempotent: calling it multiple times with the same
    /// source and context will return the same statistic IDs.
    ///
    /// Returns a map from logical key (e.g., "time_spent") to StatisticId.
    fn provision_bundle(
        &self,
        source: &str,
        context: HashMap<String, String>,
    ) -> ApplicationResult<HashMap<String, StatisticId>>;
}

/// Service for provisioning integration bundles in an idempotent manner.
///
/// This service ensures that multiple calls with the same source and context
/// return the same statistic IDs without creating duplicates.
pub struct IntegrationService {
    repository: Arc<dyn StatisticRepository + Send + Sync>,
    factories: HashMap<String, Arc<dyn BundleFactory>>,
}

impl IntegrationService {
    /// Creates a new IntegrationService with the given repository and factories.
    pub fn new(
        repository: Arc<dyn StatisticRepository + Send + Sync>,
        factories: Vec<Arc<dyn BundleFactory>>,
    ) -> Self {
        // Build a map from source_name to factory for O(1) lookup
        let factories_map = factories
            .into_iter()
            .map(|factory| (factory.source_name().to_string(), factory))
            .collect();

        Self {
            repository,
            factories: factories_map,
        }
    }
}

impl IntegrationServiceTrait for IntegrationService {
    /// Provisions a bundle of statistics for the given source and context.
    ///
    /// This method is idempotent: calling it multiple times with the same
    /// source and context will return the same statistic IDs.
    ///
    /// Returns a map from logical key (e.g., "time_spent") to StatisticId.
    fn provision_bundle(
        &self,
        source: &str,
        context: HashMap<String, String>,
    ) -> ApplicationResult<HashMap<String, StatisticId>> {
        let factory = self
            .factories
            .get(source)
            .ok_or_else(|| ApplicationError::InvalidInput(format!("Unknown source: {}", source)))?;

        let provisioned_stats = factory.provision(&context)?;

        let mut result = HashMap::new();
        let mut statistics_to_create = Vec::new();
        let mut keys_to_create = Vec::new();

        // Each provisioned statistic defines canonical tags. Reuse them to look up
        // existing metadata so factories control idempotency via their tag design.
        for provisioned in provisioned_stats {
            let tags = provisioned.statistic.get_metadata().tags().clone();

            let existing = self
                .repository
                .find_by_tags(&tags)
                .map_err(ApplicationError::Repository)?;

            if let Some(statistic) = existing.into_iter().next() {
                result.insert(provisioned.key, statistic.get_metadata().id());
            } else {
                keys_to_create.push(provisioned.key);
                statistics_to_create.push(provisioned.statistic);
            }
        }

        if statistics_to_create.is_empty() {
            return Ok(result);
        }

        let persisted = self
            .repository
            .create_many_metadata(statistics_to_create)
            .map_err(ApplicationError::Repository)?;

        for (key, statistic) in keys_to_create.into_iter().zip(persisted.into_iter()) {
            result.insert(key, statistic.get_metadata().id());
        }

        Ok(result)
    }
}
