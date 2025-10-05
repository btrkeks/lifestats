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

/// Request DTO for logging a time frame to a TimeSpans statistic
#[derive(Debug, Deserialize, Validate)]
pub struct LogTimeFrameRequest {
    #[validate(custom(function = "validate_time_format"))]
    pub start_time: String,
    #[validate(custom(function = "validate_time_format"))]
    pub end_time: String,
    pub update_policy: Option<String>,
}

fn validate_time_format(time_str: &str) -> Result<(), validator::ValidationError> {
    let time_regex = regex::Regex::new(r"^\d{2}:\d{2}(:\d{2})?$").unwrap();
    if time_regex.is_match(time_str) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("time_format"))
    }
}

/// Response DTO representing a statistic's metadata
#[derive(Debug, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_domain::shared::ids::StatisticId;
    use lifestats_domain::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_from_any_statistic_to_dto() {
        // Create a test statistic
        let metadata = StatisticMetadata::new(
            StatisticId(42),
            "Fitness".to_string().into_boxed_str(),
            "Push Ups".to_string().into_boxed_str(),
        );
        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ));

        // Convert to DTO
        let dto = StatisticMetadataResponse::from(&statistic);

        // Assertions
        assert_eq!(dto.id, 42);
        assert_eq!(dto.name, "Push Ups");
        assert_eq!(dto.category, "Fitness");
    }
}
