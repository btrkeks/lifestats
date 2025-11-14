use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// Request DTO for setting a goal (both default and daily goals)
///
/// The goal_data HashMap contains type-specific fields:
/// - IntegerAmount: {"target": "100", "is_at_least": "true"}
/// - Habit: {"occurred": "true"}
/// - PointInTime: {"deadline": "23:00"}
/// - TimeSpans: {"target": "[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]", "efficiency": "80"}
/// - Summing/TimeDifference: {"target": "3600", "is_at_least": "true"}
#[derive(Debug, Deserialize, Validate)]
pub struct SetDefaultGoalRequest {
    pub goal_data: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SetDailyGoalRequest {
    Goal { goal_data: HashMap<String, String> },
    Suppressed,
}

/// Response DTO for goal data
#[derive(Debug, Serialize, Deserialize)]
pub struct GoalResponse {
    pub goal_data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_goal_request_deserialization() {
        let json = r#"{"goal_data": {"target": "100", "is_at_least": "true"}}"#;
        let req: SetDefaultGoalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.goal_data.get("target").unwrap(), "100");
    }
}
