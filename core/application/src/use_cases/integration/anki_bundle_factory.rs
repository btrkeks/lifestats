use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::{ApplicationError, ApplicationResult};
use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

/// Factory for creating Anki integration bundles.
///
/// Creates statistics for Anki:
/// - Time Spent: tracks study sessions (per deck)
/// - Disruptions: counts interruptions during study (per deck)
/// - Backlog: tracks number of cards due (per deck)
/// - Finished Time: tracks when all cards in the deck were completed for the day (per deck)
const ANKI_SOURCE_TAG: &str = "anki";
const METRIC_TIME_SPENT: &str = "time_spent";
const METRIC_DISRUPTIONS: &str = "disruptions";
const METRIC_BACKLOG: &str = "backlog";
const METRIC_FINISHED_TIME: &str = "finished_time";

pub struct AnkiBundleFactory;

impl BundleFactory for AnkiBundleFactory {
    fn source_name(&self) -> &'static str {
        "anki"
    }

    fn provision(
        &self,
        context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>> {
        // Extract deck_name from context
        let deck_name = context.get("deck_name").ok_or_else(|| {
            ApplicationError::InvalidInput("Missing required context: deck_name".to_string())
        })?;

        // Use temporary ID 0 - it will be replaced when saved to the repository
        let temp_id = StatisticId(0);
        let current_date = Utc::now().naive_utc().date();

        // Create Time Spent statistic
        let time_spent_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string(),
            format!("[Anki] Study Time ({})", deck_name),
        )
        .with_tag("source".to_string(), ANKI_SOURCE_TAG.to_string())
        .with_tag("deck_name".to_string(), deck_name.clone())
        .with_tag("metric".to_string(), METRIC_TIME_SPENT.to_string());

        let time_spent = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            time_spent_metadata,
            None,
            current_date,
        ));

        // Create Disruptions statistic
        let disruptions_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string(),
            format!("[Anki] Disruptions ({})", deck_name),
        )
        .with_tag("source".to_string(), ANKI_SOURCE_TAG.to_string())
        .with_tag("deck_name".to_string(), deck_name.clone())
        .with_tag("metric".to_string(), METRIC_DISRUPTIONS.to_string());

        let disruptions = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            disruptions_metadata,
            None,
            current_date,
        ));

        // Create Backlog statistic
        let backlog_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string(),
            format!("[Anki] Backlog ({})", deck_name),
        )
        .with_tag("source".to_string(), ANKI_SOURCE_TAG.to_string())
        .with_tag("deck_name".to_string(), deck_name.clone())
        .with_tag("metric".to_string(), METRIC_BACKLOG.to_string());

        let backlog = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            backlog_metadata,
            None,
            current_date,
        ));

        // Create Finished Time statistic (per-deck)
        // This tracks when the user finished all cards in this deck for the day
        let finished_time_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string(),
            format!("[Anki] Finished Time ({})", deck_name),
        )
        .with_tag("source".to_string(), ANKI_SOURCE_TAG.to_string())
        .with_tag("deck_name".to_string(), deck_name.clone())
        .with_tag("metric".to_string(), METRIC_FINISHED_TIME.to_string());

        let finished_time = AnyStatistic::PointInTime(PointInTimeStatistic::new(
            finished_time_metadata,
            None,
            current_date,
        ));

        // Return the provisioned statistics
        Ok(vec![
            ProvisionedStatistic {
                key: METRIC_TIME_SPENT.to_string(),
                statistic: time_spent,
            },
            ProvisionedStatistic {
                key: METRIC_DISRUPTIONS.to_string(),
                statistic: disruptions,
            },
            ProvisionedStatistic {
                key: METRIC_BACKLOG.to_string(),
                statistic: backlog,
            },
            ProvisionedStatistic {
                key: METRIC_FINISHED_TIME.to_string(),
                statistic: finished_time,
            },
        ])
    }
}
