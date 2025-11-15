use lifestats_core::shared::percentage::Percentage;
use serde::{Deserialize, Serialize};

/// Result of goal performance analysis over a date range
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalPerformanceDto {
    /// Overall achievement score averaged across the date range
    pub overall_score: Percentage,
    /// Trend indicator based on exponentially weighted moving averages
    pub trend: TrendDto,
}

/// Trend direction and strength based on EWMA analysis
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "direction", rename_all = "snake_case")]
pub enum TrendDto {
    /// Performance is improving (fast EWMA > slow EWMA)
    Improving {
        /// Strength of the trend (0.0 to 1.0)
        strength: f64,
    },
    /// Performance is declining (fast EWMA < slow EWMA)
    Declining {
        /// Strength of the trend (0.0 to 1.0)
        strength: f64,
    },
    /// Performance is stable (difference within epsilon threshold)
    Stable,
    /// Not enough data points to calculate a meaningful trend
    InsufficientData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_dto_serialization_improving() {
        let trend = TrendDto::Improving { strength: 0.75 };
        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("\"direction\":\"improving\""));
        assert!(json.contains("\"strength\":0.75"));
    }

    #[test]
    fn test_trend_dto_serialization_declining() {
        let trend = TrendDto::Declining { strength: 0.5 };
        let json = serde_json::to_string(&trend).unwrap();
        assert!(json.contains("\"direction\":\"declining\""));
        assert!(json.contains("\"strength\":0.5"));
    }

    #[test]
    fn test_trend_dto_serialization_stable() {
        let trend = TrendDto::Stable;
        let json = serde_json::to_string(&trend).unwrap();
        assert_eq!(json, r#"{"direction":"stable"}"#);
    }

    #[test]
    fn test_trend_dto_serialization_insufficient_data() {
        let trend = TrendDto::InsufficientData;
        let json = serde_json::to_string(&trend).unwrap();
        assert_eq!(json, r#"{"direction":"insufficient_data"}"#);
    }

    #[test]
    fn test_goal_performance_dto_serialization() {
        let dto = GoalPerformanceDto {
            overall_score: Percentage::new(85),
            trend: TrendDto::Improving { strength: 0.6 },
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"overall_score\":85"));
        assert!(json.contains("\"direction\":\"improving\""));
        assert!(json.contains("\"strength\":0.6"));
    }
}
