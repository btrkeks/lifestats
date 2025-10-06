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
                // Extract the key from the statistic's name or tags
                // For now, we'll use a simple heuristic: the last part of the name
                // This works for "[Anki] Time Spent (deck)" -> "time_spent"
                let key = derive_key_from_stat(&stat);
                result.insert(key, stat.get_metadata().id());
            }
            return Ok(result);
        }

        // Statistics don't exist - create them
        let factory = self
            .factories
            .get(source)
            .ok_or_else(|| ApplicationError::InvalidInput(format!("Unknown source: {}", source)))?;

        let provisioned_stats = factory.provision(&context)?;

        let mut result = HashMap::new();
        for provisioned in provisioned_stats {
            self.repository
                .save_metadata(&provisioned.statistic)
                .map_err(ApplicationError::Repository)?;

            result.insert(
                provisioned.key.clone(),
                provisioned.statistic.get_metadata().id(),
            );
        }

        Ok(result)
    }
}

/// Helper function to derive a logical key from a statistic's name.
/// This is a simple heuristic and could be improved.
fn derive_key_from_stat(stat: &lifestats_core::statistic::any_statistic::AnyStatistic) -> String {
    let name = stat.get_metadata().name();

    // Simple heuristic: convert name to lowercase snake_case
    // "[Anki] Time Spent (deck)" -> "time_spent"
    // "[Anki] Disruptions (deck)" -> "disruptions"
    // "[Anki] Backlog (deck)" -> "backlog"

    if name.contains("Time Spent") || name.contains("Study Time") {
        "time_spent".to_string()
    } else if name.contains("Disruptions") {
        "disruptions".to_string()
    } else if name.contains("Backlog") {
        "backlog".to_string()
    } else {
        // Fallback: use the name as-is (converted to snake_case)
        name.to_lowercase().replace(' ', "_")
    }
}
