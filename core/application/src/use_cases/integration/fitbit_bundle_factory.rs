use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::ApplicationResult;
use crate::integrations::fitbit::ports::FitbitMetric;
use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

/// Factory for creating Fitbit integration bundles.
///
/// Creates four core statistics for Fitbit data:
/// - Steps: tracks daily step count
/// - Sleep Duration: tracks total sleep time
/// - Fell Asleep Time: tracks when user went to bed
/// - Wake Up Time: tracks when user woke up
pub struct FitbitBundleFactory;

impl BundleFactory for FitbitBundleFactory {
    fn source_name(&self) -> &'static str {
        "fitbit"
    }

    fn provision(
        &self,
        _context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>> {
        // Use temporary ID 0 - it will be replaced when saved to the repository
        let temp_id = StatisticId(0);
        let current_date = Utc::now().naive_utc().date();

        // Create Steps statistic
        let steps_metadata =
            StatisticMetadata::new(temp_id, "Fitbit".to_string(), "[Fitbit] Steps".to_string())
                .with_tag("source".to_string(), "fitbit".to_string())
                .with_tag(
                    "metric".to_string(),
                    FitbitMetric::Steps.canonical_tag_value().to_string(),
                );

        let steps = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            steps_metadata,
            None,
            current_date,
        ));

        // Create Sleep Duration statistic
        let sleep_duration_metadata = StatisticMetadata::new(
            temp_id,
            "Fitbit".to_string(),
            "[Fitbit] Sleep Duration".to_string(),
        )
        .with_tag("source".to_string(), "fitbit".to_string())
        .with_tag(
            "metric".to_string(),
            FitbitMetric::SleepDuration
                .canonical_tag_value()
                .to_string(),
        );

        let sleep_duration = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            sleep_duration_metadata,
            None,
            current_date,
        ));

        // Create Fell Asleep Time statistic
        let fell_asleep_time_metadata = StatisticMetadata::new(
            temp_id,
            "Fitbit".to_string(),
            "[Fitbit] Fell Asleep Time".to_string(),
        )
        .with_tag("source".to_string(), "fitbit".to_string())
        .with_tag(
            "metric".to_string(),
            FitbitMetric::FellAsleepTime
                .canonical_tag_value()
                .to_string(),
        );

        let fell_asleep_time = AnyStatistic::PointInTime(PointInTimeStatistic::new(
            fell_asleep_time_metadata,
            None,
            current_date,
        ));

        // Create Wake Up Time statistic
        let wake_up_time_metadata = StatisticMetadata::new(
            temp_id,
            "Fitbit".to_string(),
            "[Fitbit] Wake Up Time".to_string(),
        )
        .with_tag("source".to_string(), "fitbit".to_string())
        .with_tag(
            "metric".to_string(),
            FitbitMetric::WakeUpTime.canonical_tag_value().to_string(),
        );

        let wake_up_time = AnyStatistic::PointInTime(PointInTimeStatistic::new(
            wake_up_time_metadata,
            None,
            current_date,
        ));

        // Return the provisioned statistics
        Ok(vec![
            ProvisionedStatistic {
                key: FitbitMetric::Steps.canonical_tag_value().to_string(),
                statistic: steps,
            },
            ProvisionedStatistic {
                key: FitbitMetric::SleepDuration
                    .canonical_tag_value()
                    .to_string(),
                statistic: sleep_duration,
            },
            ProvisionedStatistic {
                key: FitbitMetric::FellAsleepTime
                    .canonical_tag_value()
                    .to_string(),
                statistic: fell_asleep_time,
            },
            ProvisionedStatistic {
                key: FitbitMetric::WakeUpTime.canonical_tag_value().to_string(),
                statistic: wake_up_time,
            },
        ])
    }
}
