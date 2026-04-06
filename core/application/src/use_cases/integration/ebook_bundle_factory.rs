use super::bundle_factory::{BundleFactory, ProvisionedStatistic};
use crate::error::ApplicationResult;
use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use std::collections::HashMap;

const EBOOK_SOURCE_TAG: &str = "ebook";
const EBOOK_BUNDLE_VERSION: &str = "1";
const METRIC_CHARACTERS_READ: &str = "characters_read";

pub struct EbookBundleFactory;

impl BundleFactory for EbookBundleFactory {
    fn source_name(&self) -> &'static str {
        EBOOK_SOURCE_TAG
    }

    fn provision(
        &self,
        _context: &HashMap<String, String>,
    ) -> ApplicationResult<Vec<ProvisionedStatistic>> {
        let temp_id = StatisticId(0);
        let current_date = Utc::now().naive_utc().date();

        let metadata = StatisticMetadata::new(
            temp_id,
            "読書".to_string(),
            "[Ebook] Characters Read".to_string(),
        )
        .with_tag("source".to_string(), EBOOK_SOURCE_TAG.to_string())
        .with_tag("metric".to_string(), METRIC_CHARACTERS_READ.to_string())
        .with_tag(
            "bundle_version".to_string(),
            EBOOK_BUNDLE_VERSION.to_string(),
        );

        let characters_read =
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, current_date));

        Ok(vec![ProvisionedStatistic {
            key: METRIC_CHARACTERS_READ.to_string(),
            statistic: characters_read,
        }])
    }
}
