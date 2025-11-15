use serde::Deserialize;
use validator::Validate;

/// Query parameters for the goal performance endpoint
#[derive(Debug, Deserialize, Validate)]
pub struct GoalPerformanceParams {
    /// Start date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub start_date: String,
    /// End date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub end_date: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_performance_params_valid() {
        let json = r#"{"start_date": "2025-01-01", "end_date": "2025-01-31"}"#;
        let params: GoalPerformanceParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.start_date, "2025-01-01");
        assert_eq!(params.end_date, "2025-01-31");
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_goal_performance_params_empty_start() {
        let params = GoalPerformanceParams {
            start_date: String::new(),
            end_date: "2025-01-31".to_string(),
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_goal_performance_params_empty_end() {
        let params = GoalPerformanceParams {
            start_date: "2025-01-01".to_string(),
            end_date: String::new(),
        };
        assert!(params.validate().is_err());
    }
}
