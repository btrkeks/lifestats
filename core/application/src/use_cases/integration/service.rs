use std::collections::HashMap;
use std::sync::Arc;

use super::bundle_factory::BundleFactory;
use super::ebook_event_store::{EbookEventStore, EbookSyncEvent};
use crate::error::{ApplicationError, ApplicationResult};
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

#[cfg(test)]
use mockall::automock;

const EBOOK_SOURCE: &str = "ebook";
const EBOOK_METRIC_CHARACTERS_READ: &str = "characters_read";

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

    /// Accepts an idempotent ebook sync event and applies it at most once.
    fn ingest_ebook_event(
        &self,
        request: EbookEventIngestionRequest,
    ) -> ApplicationResult<EbookEventIngestionResult>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookEventIngestionRequest {
    pub event_id: String,
    pub date: NaiveDate,
    pub metric: String,
    pub delta: i64,
    pub book_id: Option<String>,
    pub book_title: String,
    pub confirmed_position_after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookEventIngestionResult {
    pub accepted: bool,
    pub statistic_id: StatisticId,
}

/// Service for provisioning integration bundles in an idempotent manner.
///
/// This service ensures that multiple calls with the same source and context
/// return the same statistic IDs without creating duplicates.
pub struct IntegrationService {
    repository: Arc<dyn StatisticRepository + Send + Sync>,
    factories: HashMap<String, Arc<dyn BundleFactory>>,
    ebook_event_store: Option<Arc<dyn EbookEventStore>>,
}

impl IntegrationService {
    /// Creates a new IntegrationService with the given repository and factories.
    pub fn new(
        repository: Arc<dyn StatisticRepository + Send + Sync>,
        factories: Vec<Arc<dyn BundleFactory>>,
        ebook_event_store: Option<Arc<dyn EbookEventStore>>,
    ) -> Self {
        // Build a map from source_name to factory for O(1) lookup
        let factories_map = factories
            .into_iter()
            .map(|factory| (factory.source_name().to_string(), factory))
            .collect();

        Self {
            repository,
            factories: factories_map,
            ebook_event_store,
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

    fn ingest_ebook_event(
        &self,
        request: EbookEventIngestionRequest,
    ) -> ApplicationResult<EbookEventIngestionResult> {
        if request.event_id.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "ebook event_id must not be empty".to_string(),
            ));
        }

        if request.metric != EBOOK_METRIC_CHARACTERS_READ {
            return Err(ApplicationError::InvalidInput(format!(
                "unsupported ebook metric: {}",
                request.metric
            )));
        }

        if request.delta <= 0 {
            return Err(ApplicationError::InvalidInput(
                "ebook delta must be positive".to_string(),
            ));
        }

        let ebook_event_store = self.ebook_event_store.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "ebook event ingestion is not configured for this deployment".to_string(),
            )
        })?;

        let statistic_ids = self.provision_bundle(EBOOK_SOURCE, HashMap::new())?;
        let statistic_id = statistic_ids
            .get(EBOOK_METRIC_CHARACTERS_READ)
            .copied()
            .ok_or_else(|| {
                ApplicationError::Infrastructure(anyhow::anyhow!(
                    "ebook bundle did not provision the characters_read statistic"
                ))
            })?;

        let accepted = ebook_event_store.record_event(&EbookSyncEvent {
            event_id: request.event_id,
            source_name: EBOOK_SOURCE.to_string(),
            metric: request.metric,
            statistic_id,
            date: request.date,
            delta: request.delta,
            book_id: request.book_id,
            book_title: request.book_title,
            confirmed_position_after: request.confirmed_position_after,
        })?;

        Ok(EbookEventIngestionResult {
            accepted,
            statistic_id,
        })
    }
}
