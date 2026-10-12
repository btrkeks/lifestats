use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::ApplicationResult;
use chrono::Local;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

pub const YOMIKATA_SOURCE_TAG: &str = "yomikata";
const YOMIKATA_BUNDLE_VERSION: &str = "1";
pub const METRIC_CARDS_REVIEWED: &str = "cards_reviewed";
pub const METRIC_STUDY_MINUTES: &str = "study_minutes";
const CARDS_REVIEWED_DAILY_TARGET: i64 = 400;

pub struct YomikataBundleFactory;

impl BundleFactory for YomikataBundleFactory {
    fn source_name(&self) -> &'static str {
        YOMIKATA_SOURCE_TAG
    }

    fn provision(
        &self,
        _context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>> {
        let temp_id = StatisticId(0);
        let current_date = Local::now().date_naive();

        let cards_reviewed_metadata = StatisticMetadata::new(
            temp_id,
            "Yomikata".to_string(),
            "[Yomikata] Cards Reviewed".to_string(),
        )
        .with_tag("source".to_string(), YOMIKATA_SOURCE_TAG.to_string())
        .with_tag("metric".to_string(), METRIC_CARDS_REVIEWED.to_string())
        .with_tag(
            "bundle_version".to_string(),
            YOMIKATA_BUNDLE_VERSION.to_string(),
        );

        let cards_reviewed = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            cards_reviewed_metadata,
            Some(AmountGoal::new(CARDS_REVIEWED_DAILY_TARGET, true)),
            current_date,
        ));

        let study_minutes_metadata = StatisticMetadata::new(
            temp_id,
            "Yomikata".to_string(),
            "[Yomikata] Study Time (minutes)".to_string(),
        )
        .with_tag("source".to_string(), YOMIKATA_SOURCE_TAG.to_string())
        .with_tag("metric".to_string(), METRIC_STUDY_MINUTES.to_string())
        .with_tag(
            "bundle_version".to_string(),
            YOMIKATA_BUNDLE_VERSION.to_string(),
        );

        let study_minutes = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            study_minutes_metadata,
            None,
            current_date,
        ));

        Ok(vec![
            ProvisionedStatistic {
                key: METRIC_CARDS_REVIEWED.to_string(),
                statistic: cards_reviewed,
            },
            ProvisionedStatistic {
                key: METRIC_STUDY_MINUTES.to_string(),
                statistic: study_minutes,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifestats_core::statistic::traits::base_statistic::Statistic;

    fn tags(metric: &str) -> HashMap<String, String> {
        HashMap::from([
            ("source".to_string(), "yomikata".to_string()),
            ("metric".to_string(), metric.to_string()),
            ("bundle_version".to_string(), "1".to_string()),
        ])
    }

    #[test]
    fn provisions_cards_reviewed_with_goal_and_study_minutes_without() {
        let provisioned = YomikataBundleFactory.provision(&HashMap::new()).unwrap();

        let summary: Vec<_> = provisioned
            .iter()
            .map(|p| {
                let AnyStatistic::IntegerAmount(statistic) = &p.statistic else {
                    panic!("{} is not an IntegerAmount statistic", p.key);
                };
                let metadata = statistic.get_metadata();
                let goal = statistic
                    .get_default_goal()
                    .map(|g| (g.target_amount(), g.is_at_least()));
                (
                    p.key.as_str(),
                    metadata.name(),
                    metadata.category(),
                    metadata.tags().clone(),
                    goal,
                )
            })
            .collect();

        assert_eq!(
            summary,
            vec![
                (
                    "cards_reviewed",
                    "[Yomikata] Cards Reviewed",
                    "Yomikata",
                    tags("cards_reviewed"),
                    Some((400, true)),
                ),
                (
                    "study_minutes",
                    "[Yomikata] Study Time (minutes)",
                    "Yomikata",
                    tags("study_minutes"),
                    None,
                ),
            ]
        );
    }
}
