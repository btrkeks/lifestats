use chrono::NaiveDate;
use lifestats_application::use_cases::queries::get_time_spans_total::TimeSpansTotalResult;
use lifestats_core::shared::ids::StatisticId;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimeSpansTotalResponse {
    pub statistic_id: StatisticId,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_seconds: i64,
    pub total_duration_formatted: String,
    pub days_with_data: usize,
}

impl From<TimeSpansTotalResult> for TimeSpansTotalResponse {
    fn from(result: TimeSpansTotalResult) -> Self {
        Self {
            statistic_id: result.statistic_id,
            start_date: result.start_date,
            end_date: result.end_date,
            total_seconds: result.total_seconds,
            total_duration_formatted: format_duration_hms(result.total_seconds),
            days_with_data: result.days_with_data,
        }
    }
}

fn format_duration_hms(total_seconds: i64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_hms() {
        assert_eq!(format_duration_hms(0), "0h 0m 0s");
        assert_eq!(format_duration_hms(3661), "1h 1m 1s");
        assert_eq!(format_duration_hms(7200), "2h 0m 0s");
        assert_eq!(format_duration_hms(5400), "1h 30m 0s");
        assert_eq!(format_duration_hms(12600), "3h 30m 0s");
    }
}
