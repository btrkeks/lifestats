use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

/// Builds canonical Fitbit statistic metadata used across integration tests to
/// mirror the production tagging contract.
pub fn mock_fitbit_statistic_metadata(id: u32, metric: &str) -> StatisticMetadata {
    StatisticMetadata::new(StatisticId(id), "Fitbit".to_string(), metric.to_string())
        .with_tag("source".to_string(), "fitbit".to_string())
        .with_tag("metric".to_string(), metric.to_string())
}
