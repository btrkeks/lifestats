use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::{ApplicationError, ApplicationResult};
use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

/// Factory for creating Anki integration bundles.
///
/// Creates three statistics for each Anki deck:
/// - Time Spent: tracks study sessions
/// - Disruptions: counts interruptions during study
/// - Backlog: tracks number of cards due
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
            "Anki".to_string().into_boxed_str(),
            format!("[Anki] Study Time ({})", deck_name).into_boxed_str(),
        )
        .with_tag("source".to_string(), "anki".to_string())
        .with_tag("deck_name".to_string(), deck_name.clone());

        let time_spent = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            time_spent_metadata,
            None,
            current_date,
        ));

        // Create Disruptions statistic
        let disruptions_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string().into_boxed_str(),
            format!("[Anki] Disruptions ({})", deck_name).into_boxed_str(),
        )
        .with_tag("source".to_string(), "anki".to_string())
        .with_tag("deck_name".to_string(), deck_name.clone());

        let disruptions = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            disruptions_metadata,
            None,
            current_date,
        ));

        // Create Backlog statistic
        let backlog_metadata = StatisticMetadata::new(
            temp_id,
            "Anki".to_string().into_boxed_str(),
            format!("[Anki] Backlog ({})", deck_name).into_boxed_str(),
        )
        .with_tag("source".to_string(), "anki".to_string())
        .with_tag("deck_name".to_string(), deck_name.clone());

        let backlog = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            backlog_metadata,
            None,
            current_date,
        ));

        // Return the provisioned statistics
        Ok(vec![
            ProvisionedStatistic {
                key: "time_spent".to_string(),
                statistic: time_spent,
            },
            ProvisionedStatistic {
                key: "disruptions".to_string(),
                statistic: disruptions,
            },
            ProvisionedStatistic {
                key: "backlog".to_string(),
                statistic: backlog,
            },
        ])
    }
}
