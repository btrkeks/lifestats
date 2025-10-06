use crate::error::ApplicationResult;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use std::collections::HashMap;

/// Represents a statistic that has been provisioned as part of a bundle.
pub struct ProvisionedStatistic {
    /// The logical key for this statistic within the bundle (e.g., "time_spent", "disruptions").
    pub key: String,
    /// The domain entity for the statistic.
    pub statistic: AnyStatistic,
}

/// A trait for factories that know how to create the set of statistics
/// for a specific third-party integration.
///
/// This uses the Strategy pattern to allow different integrations
/// (Anki, Fitbit, etc.) to define their own bundle creation logic.
pub trait BundleFactory: Send + Sync {
    /// The unique name of the source (e.g., "anki", "fitbit").
    fn source_name(&self) -> &'static str;

    /// Creates the domain entities for the bundle based on context.
    ///
    /// The context might contain "deck_name" for Anki, "device_id" for Fitbit, etc.
    /// Returns a vector of provisioned statistics with their logical keys.
    fn provision(
        &self,
        context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>>;
}
