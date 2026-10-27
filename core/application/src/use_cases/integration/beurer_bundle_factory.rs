use chrono::NaiveDate;
use lifestats_core::beurer::{BeurerBundleDefinitions, METRICS};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

pub struct BeurerBundleFactory;

impl BeurerBundleFactory {
    pub fn definitions(today: NaiveDate) -> BeurerBundleDefinitions {
        let names = [
            "[Beurer] Weight (kg)",
            "[Beurer] Body Fat (%)",
            "[Beurer] Muscle (%)",
            "[Beurer] Body Water (%)",
        ];
        BeurerBundleDefinitions(std::array::from_fn(|index| {
            let metadata =
                StatisticMetadata::new(StatisticId(0), "Beurer".into(), names[index].into())
                    .with_tag("source".into(), "beurer".into())
                    .with_tag("metric".into(), METRICS[index].into());
            IntegerAmountStatistic::new(metadata, None, today)
        }))
    }
}
