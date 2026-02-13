//! DTOs for the dashboard endpoint.

use serde::{Deserialize, Serialize};

use super::TrendDto;

/// Dashboard data for a single statistic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatisticDto {
    pub statistic_id: u32,
    pub statistic_name: String,
    pub statistic_type: String,
    /// Today's logged value as i64 (None if no value logged).
    pub today_value: Option<i64>,
    /// 7-day rolling average (None if insufficient data).
    pub avg_7d_value: Option<f64>,
    /// Performance metrics (None if no goals or insufficient data).
    pub performance: Option<DashboardPerformanceDto>,
}

/// Performance subset for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPerformanceDto {
    /// Overall achievement score (0-100).
    pub overall_score: u8,
    /// Trend direction and strength.
    pub trend: TrendDto,
}

/// Full dashboard response containing multiple statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponseDto {
    pub statistics: Vec<DashboardStatisticDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_dashboard_statistic_with_all_fields() {
        let dto = DashboardStatisticDto {
            statistic_id: 80,
            statistic_name: "Steps".to_string(),
            statistic_type: "IntegerAmount".to_string(),
            today_value: Some(573),
            avg_7d_value: Some(412.5),
            performance: Some(DashboardPerformanceDto {
                overall_score: 61,
                trend: TrendDto::Improving { strength: 0.24 },
            }),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"statistic_id\":80"));
        assert!(json.contains("\"statistic_name\":\"Steps\""));
        assert!(json.contains("\"today_value\":573"));
        assert!(json.contains("\"avg_7d_value\":412.5"));
        assert!(json.contains("\"overall_score\":61"));
    }

    #[test]
    fn serializes_dashboard_statistic_with_null_fields() {
        let dto = DashboardStatisticDto {
            statistic_id: 24,
            statistic_name: "Reading".to_string(),
            statistic_type: "TimeSpans".to_string(),
            today_value: None,
            avg_7d_value: None,
            performance: None,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"today_value\":null"));
        assert!(json.contains("\"avg_7d_value\":null"));
        assert!(json.contains("\"performance\":null"));
    }
}
