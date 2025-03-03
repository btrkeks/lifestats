use serde::{Deserialize, Serialize};
use transfer_objects::statistic_id::StatisticId;

#[derive(Debug, Serialize, Deserialize)]
pub struct LogRequest {
    pub statistic_id: StatisticId,
    pub value: String,
    #[serde(default)]
    pub update_policy: Option<String>,
    #[serde(default = "get_date_today")]
    pub date: time::Date,
}

fn get_date_today() -> time::Date {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .date()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogResponse {
    pub success: bool,
    pub message: String,
}

impl LogResponse {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
        }
    }
}
