use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for the consistency score endpoint
#[derive(Debug, Deserialize, Validate)]
pub struct ConsistencyParams {
    /// Number of days to evaluate (including today).
    /// Defaults to 14 if not specified.
    /// Valid range: [1, 366]
    #[serde(default = "default_consistency_days")]
    #[validate(range(min = 1, max = 366))]
    pub days: u32,
}

fn default_consistency_days() -> u32 {
    14
}

/// Response DTO for the consistency score endpoint
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsistencyScoreResponse {
    /// ID of the statistic
    pub statistic_id: u32,
    /// Number of days in the range that had a goal set
    pub days_evaluated: u32,
    /// Number of days (with goals) where the goal was achieved
    pub days_achieved: u32,
    /// Consistency score as a percentage (0-100), using round-half-up
    pub consistency_score_percent: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistency_params_default() {
        let json = r#"{}"#;
        let params: ConsistencyParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.days, 14);
    }

    #[test]
    fn test_consistency_params_custom() {
        let json = r#"{"days": 30}"#;
        let params: ConsistencyParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.days, 30);
    }

    #[test]
    fn test_consistency_params_validation_min() {
        let params = ConsistencyParams { days: 1 };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_consistency_params_validation_max() {
        let params = ConsistencyParams { days: 366 };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_consistency_params_validation_below_min() {
        let params = ConsistencyParams { days: 0 };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_consistency_params_validation_above_max() {
        let params = ConsistencyParams { days: 367 };
        assert!(params.validate().is_err());
    }
}
