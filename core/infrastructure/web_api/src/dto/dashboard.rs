use chrono::NaiveDate;
use serde::Deserialize;
use validator::Validate;

/// Request body for POST /statistics/dashboard
#[derive(Debug, Deserialize, Validate)]
pub struct DashboardRequest {
    /// Statistics to include in the dashboard.
    #[validate(length(min = 1, message = "statistic_ids cannot be empty"))]
    pub statistic_ids: Vec<u32>,
    /// Reference date (typically "today").
    pub today: NaiveDate,
    /// Number of days for performance window (1-365).
    #[validate(range(min = 1, max = 365, message = "performance_window_days must be 1-365"))]
    pub performance_window_days: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_valid_request() {
        let json = r#"{
            "statistic_ids": [80, 24, 23],
            "today": "2025-12-22",
            "performance_window_days": 14
        }"#;
        let req: DashboardRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.statistic_ids, vec![80, 24, 23]);
        assert_eq!(req.today.to_string(), "2025-12-22");
        assert_eq!(req.performance_window_days, 14);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn rejects_empty_statistic_ids() {
        let req = DashboardRequest {
            statistic_ids: vec![],
            today: NaiveDate::from_ymd_opt(2025, 12, 22).unwrap(),
            performance_window_days: 14,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rejects_zero_performance_window() {
        let req = DashboardRequest {
            statistic_ids: vec![1],
            today: NaiveDate::from_ymd_opt(2025, 12, 22).unwrap(),
            performance_window_days: 0,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn rejects_performance_window_over_365() {
        let req = DashboardRequest {
            statistic_ids: vec![1],
            today: NaiveDate::from_ymd_opt(2025, 12, 22).unwrap(),
            performance_window_days: 400,
        };
        assert!(req.validate().is_err());
    }
}
