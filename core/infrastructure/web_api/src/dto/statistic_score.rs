use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for the statistic score endpoint
#[derive(Debug, Deserialize, Validate)]
pub struct StatisticScoreParams {
    /// Start date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub start: String,
    /// End date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub end: String,
}

/// Response DTO for the statistic score endpoint
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatisticScoreResponse {
    /// ID of the statistic
    pub statistic_id: u32,
    /// Start date of the range (YYYY-MM-DD)
    pub start_date: String,
    /// End date of the range (YYYY-MM-DD)
    pub end_date: String,
    /// Mean percentage achieved as an integer 0-255 (or null when no goals exist in the range)
    pub mean_percentage_achieved: Option<u8>,
    /// Number of days in the range that had a goal and contributed to the score
    pub days_with_goal: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistic_score_params_valid() {
        let json = r#"{"start": "2025-01-01", "end": "2025-01-31"}"#;
        let params: StatisticScoreParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.start, "2025-01-01");
        assert_eq!(params.end, "2025-01-31");
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_statistic_score_params_empty_start() {
        let params = StatisticScoreParams {
            start: String::new(),
            end: "2025-01-31".to_string(),
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_statistic_score_params_empty_end() {
        let params = StatisticScoreParams {
            start: "2025-01-01".to_string(),
            end: String::new(),
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_statistic_score_response_serialization() {
        let response = StatisticScoreResponse {
            statistic_id: 1,
            start_date: "2025-01-01".to_string(),
            end_date: "2025-01-31".to_string(),
            mean_percentage_achieved: Some(85),
            days_with_goal: 25,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"statistic_id\":1"));
        assert!(json.contains("\"start_date\":\"2025-01-01\""));
        assert!(json.contains("\"end_date\":\"2025-01-31\""));
        assert!(json.contains("\"mean_percentage_achieved\":85"));
        assert!(json.contains("\"days_with_goal\":25"));
    }

    #[test]
    fn test_statistic_score_response_with_no_goals() {
        let response = StatisticScoreResponse {
            statistic_id: 1,
            start_date: "2025-01-01".to_string(),
            end_date: "2025-01-31".to_string(),
            mean_percentage_achieved: None,
            days_with_goal: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"mean_percentage_achieved\":null"));
        assert!(json.contains("\"days_with_goal\":0"));
    }
}
