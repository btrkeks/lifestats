// web_api/src/types/goal.rs
use serde::{Deserialize, Serialize};
use lifestats_domain::shared::ids::StatisticId;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetGoalRequest {
    pub statistic_id: StatisticId,
    pub target: String,
    pub extra_data: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteGoalRequest {
    pub statistic_id: StatisticId,
    pub date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoalResponse {
    pub success: bool,
    pub message: String,
}

impl GoalResponse {
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
