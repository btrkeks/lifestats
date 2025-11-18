use chrono::NaiveDate;
use serde::Serialize;

/// Response for checking if a statistic has been logged on a specific date.
#[derive(Debug, Serialize)]
pub struct LoggedValueResponse {
    pub statistic_id: u32,
    pub date: NaiveDate,
    pub has_value: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}
