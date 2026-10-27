/// Application-layer DTOs for statistic metadata and type information
///
/// These types serve as the boundary between the application layer and infrastructure.
/// They bundle domain concepts (StatisticMetadata) with type information that would
/// otherwise be lost when crossing architectural boundaries.
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::ApplicationError;

/// Strongly typed representation of statistic types
///
/// This enum represents all possible statistic types in the system.
/// It's used at the application boundary to communicate type information
/// without exposing the full AnyStatistic enum to API consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum_macros::EnumIter)]
pub enum StatisticType {
    IntegerAmount,
    Habit,
    PointInTime,
    TimeSpans,
    TimedAmounts,
    Summing,
    TimeDifference,
}

impl StatisticType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatisticType::IntegerAmount => "IntegerAmount",
            StatisticType::Habit => "Habit",
            StatisticType::PointInTime => "PointInTime",
            StatisticType::TimeSpans => "TimeSpans",
            StatisticType::TimedAmounts => "TimedAmounts",
            StatisticType::Summing => "Summing",
            StatisticType::TimeDifference => "TimeDifference",
        }
    }

    /// Whether users log values of this type by hand (`/statistics/loggable`).
    /// Derived types compute their values; TimedAmounts days are written only by
    /// their source integration.
    pub fn is_manually_loggable(self) -> bool {
        match self {
            Self::IntegerAmount | Self::Habit | Self::PointInTime | Self::TimeSpans => true,
            Self::TimedAmounts | Self::Summing | Self::TimeDifference => false,
        }
    }
}

impl fmt::Display for StatisticType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct StatisticTypeParseError {
    invalid_value: String,
}

impl StatisticTypeParseError {
    pub fn new(invalid_value: impl Into<String>) -> Self {
        Self {
            invalid_value: invalid_value.into(),
        }
    }

    pub fn to_application_error(self) -> ApplicationError {
        ApplicationError::InvalidInput(format!("Unknown statistic_type: {}", self.invalid_value))
    }
}

impl fmt::Display for StatisticTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown statistic_type: {}", self.invalid_value)
    }
}

impl std::error::Error for StatisticTypeParseError {}

impl FromStr for StatisticType {
    type Err = StatisticTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Derived from `as_str`, so a new variant cannot be printable but unparseable.
        use strum::IntoEnumIterator;
        Self::iter()
            .find(|statistic_type| statistic_type.as_str() == s)
            .ok_or_else(|| StatisticTypeParseError::new(s))
    }
}

/// Application-layer DTO bundling metadata with type information
///
/// This type solves a key architectural challenge: `StatisticMetadata` (domain)
/// doesn't carry type information, but API consumers need it. Rather than
/// polluting the domain model, we create this application-layer DTO that
/// combines metadata with the type extracted from `AnyStatistic`.
///
/// # Design Rationale
/// - Follows CQRS pattern: query responses can be shaped for use-case needs
/// - Respects hexagonal architecture: domain stays pure, application adapts
/// - Type-safe: no string-based type lookups or fallbacks to "Unknown"
/// - Self-documenting: name clearly indicates it's a summary view
#[derive(Debug, Clone)]
pub struct StatisticSummary {
    pub metadata: StatisticMetadata,
    pub statistic_type: StatisticType,
    /// The active default goal, serialized like `StatisticRepository::find_default_goal`
    /// returns it (`None` when the statistic has no default goal).
    pub default_goal: Option<serde_json::Value>,
}

impl From<&AnyStatistic> for StatisticSummary {
    fn from(statistic: &AnyStatistic) -> Self {
        let metadata = statistic.get_metadata().clone();
        let statistic_type = match statistic {
            AnyStatistic::Habit(_) => StatisticType::Habit,
            AnyStatistic::IntegerAmount(_) => StatisticType::IntegerAmount,
            AnyStatistic::PointInTime(_) => StatisticType::PointInTime,
            AnyStatistic::TimeSpans(_) => StatisticType::TimeSpans,
            AnyStatistic::TimedAmounts(_) => StatisticType::TimedAmounts,
            AnyStatistic::Summing(_) => StatisticType::Summing,
            AnyStatistic::TimeDifference(_) => StatisticType::TimeDifference,
        };

        let default_goal = statistic
            .get_default_goal()
            .and_then(|goal| crate::services::any_goal_to_json(&goal).ok());

        Self {
            metadata,
            statistic_type,
            default_goal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
    use lifestats_core::statistic::types::derived_statistics::time_difference::TimeDifferenceStatistic;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;

    #[test]
    fn test_statistic_type_as_str() {
        assert_eq!(StatisticType::IntegerAmount.as_str(), "IntegerAmount");
        assert_eq!(StatisticType::Habit.as_str(), "Habit");
        assert_eq!(StatisticType::PointInTime.as_str(), "PointInTime");
        assert_eq!(StatisticType::TimeSpans.as_str(), "TimeSpans");
        assert_eq!(StatisticType::Summing.as_str(), "Summing");
        assert_eq!(StatisticType::TimeDifference.as_str(), "TimeDifference");
    }

    #[test]
    fn test_statistic_type_from_str() {
        assert_eq!(
            "IntegerAmount".parse::<StatisticType>().unwrap(),
            StatisticType::IntegerAmount
        );
        assert_eq!(
            "Habit".parse::<StatisticType>().unwrap(),
            StatisticType::Habit
        );
        assert_eq!(
            "PointInTime".parse::<StatisticType>().unwrap(),
            StatisticType::PointInTime
        );
        assert_eq!(
            "TimeSpans".parse::<StatisticType>().unwrap(),
            StatisticType::TimeSpans
        );
        assert_eq!(
            "Summing".parse::<StatisticType>().unwrap(),
            StatisticType::Summing
        );
        assert_eq!(
            "TimeDifference".parse::<StatisticType>().unwrap(),
            StatisticType::TimeDifference
        );
    }

    #[test]
    fn test_statistic_type_from_str_invalid() {
        let result = "InvalidType".parse::<StatisticType>();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unknown statistic_type: InvalidType"
        );
    }

    #[test]
    fn test_statistic_type_round_trip() {
        use strum::IntoEnumIterator;
        for stat_type in StatisticType::iter() {
            let str_repr = stat_type.as_str();
            let parsed = str_repr.parse::<StatisticType>().unwrap();
            assert_eq!(parsed, stat_type);
        }
        assert_eq!(
            "TimedAmounts".parse::<StatisticType>().unwrap(),
            StatisticType::TimedAmounts
        );
    }

    #[test]
    fn test_statistic_summary_from_integer_amount() {
        let metadata = StatisticMetadata::new(
            StatisticId(1),
            "Fitness".to_string(),
            "Push Ups".to_string(),
        );
        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata.clone(),
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.metadata.id(), metadata.id());
        assert_eq!(summary.metadata.name(), "Push Ups");
        assert_eq!(summary.metadata.category(), "Fitness");
        assert_eq!(summary.statistic_type, StatisticType::IntegerAmount);
        assert_eq!(summary.default_goal, None);
    }

    #[test]
    fn test_statistic_summary_default_goal_serializes_inner_goal() {
        use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;

        let metadata = StatisticMetadata::new(
            StatisticId(1),
            "Fitness".to_string(),
            "Push Ups".to_string(),
        );
        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata,
            Some(AmountGoal::new(10, true)),
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(
            summary.default_goal,
            Some(serde_json::json!({"target_amount": 10, "is_at_least": true}))
        );
    }

    #[test]
    fn test_statistic_summary_from_habit() {
        let metadata = StatisticMetadata::new(
            StatisticId(2),
            "Health".to_string(),
            "Meditation".to_string(),
        );
        let statistic = AnyStatistic::Habit(HabitStatistic::new(
            metadata,
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.statistic_type, StatisticType::Habit);
    }

    #[test]
    fn test_statistic_summary_from_point_in_time() {
        let metadata =
            StatisticMetadata::new(StatisticId(3), "Sleep".to_string(), "Wake Time".to_string());
        let statistic = AnyStatistic::PointInTime(PointInTimeStatistic::new(
            metadata,
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.statistic_type, StatisticType::PointInTime);
    }

    #[test]
    fn test_statistic_summary_from_time_spans() {
        let metadata =
            StatisticMetadata::new(StatisticId(4), "Work".to_string(), "Focus Time".to_string());
        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            metadata,
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.statistic_type, StatisticType::TimeSpans);
    }

    #[test]
    fn test_statistic_summary_from_summing() {
        let metadata =
            StatisticMetadata::new(StatisticId(5), "Work".to_string(), "Total Work".to_string());
        let statistic = AnyStatistic::Summing(SummingStatistic::new(
            metadata,
            Vec::new(),
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.statistic_type, StatisticType::Summing);
    }

    #[test]
    fn test_statistic_summary_from_time_difference() {
        let metadata = StatisticMetadata::new(
            StatisticId(6),
            "Sleep".to_string(),
            "Sleep Duration".to_string(),
        );
        let statistic = AnyStatistic::TimeDifference(TimeDifferenceStatistic::new(
            metadata,
            StatisticId(1),
            StatisticId(2),
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(summary.statistic_type, StatisticType::TimeDifference);
    }

    #[test]
    fn test_statistic_summary_preserves_tags() {
        // Test that tags from metadata are preserved in the summary
        let metadata = StatisticMetadata::new(
            StatisticId(7),
            "日本語::4 - Sentences".to_string(),
            "[Anki] Backlog (日本語::4 - Sentences)".to_string(),
        )
        .with_tag("deck_name".to_string(), "日本語::4 - Sentences".to_string())
        .with_tag("stat_key".to_string(), "backlog".to_string())
        .with_tag("source".to_string(), "anki".to_string());

        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata.clone(),
            None,
            Local::now().date_naive(),
        ));

        let summary = StatisticSummary::from(&statistic);

        assert_eq!(
            summary.metadata.get_tag("deck_name"),
            Some(&"日本語::4 - Sentences".to_string())
        );
        assert_eq!(
            summary.metadata.get_tag("stat_key"),
            Some(&"backlog".to_string())
        );
        assert_eq!(
            summary.metadata.get_tag("source"),
            Some(&"anki".to_string())
        );
        assert_eq!(summary.statistic_type, StatisticType::IntegerAmount);
    }
}
