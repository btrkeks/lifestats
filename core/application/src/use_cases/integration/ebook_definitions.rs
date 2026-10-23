use chrono::Utc;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

const EBOOK_SOURCE_TAG: &str = "ebook";
const EBOOK_BUNDLE_VERSION: &str = "1";
const METRIC_CHARACTERS_READ: &str = "characters_read";

pub fn definitions() -> lifestats_core::ebook::EbookBundleDefinitions {
    let metadata = StatisticMetadata::new(
        StatisticId(0),
        "読書".into(),
        "[Ebook] Characters Read".into(),
    )
    .with_tag("source".into(), EBOOK_SOURCE_TAG.into())
    .with_tag("metric".into(), METRIC_CHARACTERS_READ.into())
    .with_tag("bundle_version".into(), EBOOK_BUNDLE_VERSION.into());
    lifestats_core::ebook::EbookBundleDefinitions {
        characters_read: IntegerAmountStatistic::new(metadata, None, Utc::now().date_naive()),
    }
}
