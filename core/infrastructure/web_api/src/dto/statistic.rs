use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request DTO for creating a new statistic
#[derive(Debug, Deserialize, Validate)]
pub struct CreateStatisticRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub category: String,
    #[validate(length(min = 1))]
    pub statistic_type: String, // "IntegerAmount", "Habit", "PointInTime", "TimeSpans", "Summing", "TimeDifference"
    /// Required for Summing and TimeDifference statistics
    pub source_statistic_ids: Option<Vec<u32>>,
}

/// Request DTO for updating a statistic's metadata
#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct UpdateStatisticRequest {
    pub name: Option<String>,
    pub category: Option<String>,
}

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
    pub statistic_type: String,
}

impl StatisticMetadataResponse {
    fn from_metadata_and_type(
        metadata: &StatisticMetadata,
        statistic_type: impl Into<String>,
    ) -> Self {
        Self {
            id: metadata.id().0,
            name: metadata.name().to_string(),
            category: metadata.category().to_string(),
            statistic_type: statistic_type.into(),
        }
    }
}

impl From<&AnyStatistic> for StatisticMetadataResponse {
    fn from(statistic: &AnyStatistic) -> Self {
        let metadata = statistic.get_metadata();
        let statistic_type = match statistic {
            AnyStatistic::Habit(_) => "Habit",
            AnyStatistic::IntegerAmount(_) => "IntegerAmount",
            AnyStatistic::PointInTime(_) => "PointInTime",
            AnyStatistic::TimeSpans(_) => "TimeSpans",
            AnyStatistic::Summing(_) => "Summing",
            AnyStatistic::TimeDifference(_) => "TimeDifference",
        }
        .to_string();
        Self::from_metadata_and_type(metadata, statistic_type)
    }
}

impl From<&StatisticMetadata> for StatisticMetadataResponse {
    fn from(metadata: &StatisticMetadata) -> Self {
        let statistic_type = metadata
            .get_tag("statistic_type")
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        Self::from_metadata_and_type(metadata, statistic_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_from_any_statistic_to_dto() {
        // Create a test statistic
        let metadata = StatisticMetadata::new(
            StatisticId(42),
            "Fitness".to_string(),
            "Push Ups".to_string(),
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
        assert_eq!(dto.statistic_type, "IntegerAmount");
    }

    #[test]
    fn test_from_metadata_uses_tag() {
        let metadata = StatisticMetadata::new(
            StatisticId(5),
            "Wellness".to_string(),
            "Meditation".to_string(),
        )
        .with_tag("statistic_type".to_string(), "Habit".to_string());

        let dto = StatisticMetadataResponse::from(&metadata);

        assert_eq!(dto.id, 5);
        assert_eq!(dto.name, "Meditation");
        assert_eq!(dto.statistic_type, "Habit");
    }
}
