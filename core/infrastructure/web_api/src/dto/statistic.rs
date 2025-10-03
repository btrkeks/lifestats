use lifestats_domain::statistic::any_statistic::AnyStatistic;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request DTO for logging a value to a statistic
#[derive(Debug, Deserialize, Validate)]
pub struct LogValueRequest {
    #[validate(length(min = 1))]
    pub value: String,
    pub update_policy: Option<String>,
}

/// Response DTO representing a statistic's metadata
#[derive(Debug, Serialize)]
pub struct StatisticMetadataResponse {
    pub id: u32,
    pub name: String,
    pub category: String,
}

impl From<&AnyStatistic> for StatisticMetadataResponse {
    fn from(statistic: &AnyStatistic) -> Self {
        let metadata = statistic.get_metadata();
        Self {
            id: metadata.id().0,
            name: metadata.name().to_string(),
            category: metadata.category().to_string(),
        }
    }
}
