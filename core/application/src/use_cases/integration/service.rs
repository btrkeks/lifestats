use std::collections::HashMap;
use std::sync::Arc;

use super::bundle_factory::BundleFactory;
use crate::error::{ApplicationError, ApplicationResult};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

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

    /// Provisions a bundle of statistics for the given source and context.
    ///
    /// This method is idempotent: calling it multiple times with the same
    /// source and context will return the same statistic IDs.
    ///
    /// Returns a map from logical key (e.g., "time_spent") to StatisticId.
    pub fn provision_bundle(
        &self,
        source: &str,
        context: HashMap<String, String>,
    ) -> ApplicationResult<HashMap<String, StatisticId>> {
        // Build tags map for querying existing statistics
        let mut tags = HashMap::new();
        tags.insert("source".to_string(), source.to_string());
        for (key, value) in &context {
            tags.insert(key.clone(), value.clone());
        }

        // Check if statistics already exist
        let existing_stats = self
            .repository
            .find_by_tags(&tags)
            .map_err(ApplicationError::Repository)?;

        if !existing_stats.is_empty() {
            // Statistics already exist - extract IDs and return
            let mut result = HashMap::new();
            for stat in existing_stats {
                if let Some(key) = stat.get_metadata().tags().get("stat_key") {
                    result.insert(key.clone(), stat.get_metadata().id());
                } else {
                    log::warn!(
                        "Integration statistic {} missing stat_key tag; skipping idempotency mapping",
                        stat.get_metadata().id().0
                    );
                }
            }
            return Ok(result);
        }

        // Statistics don't exist - create them
        let factory = self
            .factories
            .get(source)
            .ok_or_else(|| ApplicationError::InvalidInput(format!("Unknown source: {}", source)))?;

        let provisioned_stats = factory.provision(&context)?;
        let statistics: Vec<_> = provisioned_stats
            .into_iter()
            .map(|provisioned| provisioned.statistic)
            .collect();

        let persisted = self
            .repository
            .create_many_metadata(statistics)
            .map_err(ApplicationError::Repository)?;

        let mut result = HashMap::new();
        for statistic in persisted {
            if let Some(key) = statistic.get_metadata().tags().get("stat_key") {
                result.insert(key.clone(), statistic.get_metadata().id());
            } else {
                log::warn!(
                    "Integration statistic {} missing stat_key tag; skipping idempotency mapping",
                    statistic.get_metadata().id().0
                );
            }
        }

        Ok(result)
    }
}
