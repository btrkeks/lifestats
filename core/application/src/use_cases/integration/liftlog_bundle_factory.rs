use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::ApplicationResult;
use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

pub const LIFTLOG_SOURCE_TAG: &str = "liftlog";
const LIFTLOG_BUNDLE_VERSION: &str = "1";
pub const METRIC_WORKOUT_TIME: &str = "workout_time";
pub const METRIC_TOTAL_WEIGHT: &str = "total_weight";

pub struct LiftLogBundleFactory;

impl BundleFactory for LiftLogBundleFactory {
    fn source_name(&self) -> &'static str {
        LIFTLOG_SOURCE_TAG
    }

    fn provision(
        &self,
        _context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>> {
        let temp_id = StatisticId(0);
        let current_date = Utc::now().naive_utc().date();

        let workout_time_metadata = StatisticMetadata::new(
            temp_id,
            "LiftLog".to_string(),
            "[LiftLog] Workout Time".to_string(),
        )
        .with_tag("source".to_string(), LIFTLOG_SOURCE_TAG.to_string())
        .with_tag("metric".to_string(), METRIC_WORKOUT_TIME.to_string())
        .with_tag(
            "bundle_version".to_string(),
            LIFTLOG_BUNDLE_VERSION.to_string(),
        );

        let workout_time = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            workout_time_metadata,
            None,
            current_date,
        ));

        let total_weight_metadata = StatisticMetadata::new(
            temp_id,
            "LiftLog".to_string(),
            "[LiftLog] Total Weight Lifted (kg)".to_string(),
        )
        .with_tag("source".to_string(), LIFTLOG_SOURCE_TAG.to_string())
        .with_tag("metric".to_string(), METRIC_TOTAL_WEIGHT.to_string())
        .with_tag(
            "bundle_version".to_string(),
            LIFTLOG_BUNDLE_VERSION.to_string(),
        );

        let total_weight = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            total_weight_metadata,
            None,
            current_date,
        ));

        Ok(vec![
            ProvisionedStatistic {
                key: METRIC_WORKOUT_TIME.to_string(),
                statistic: workout_time,
            },
            ProvisionedStatistic {
                key: METRIC_TOTAL_WEIGHT.to_string(),
                statistic: total_weight,
            },
        ])
    }
}
