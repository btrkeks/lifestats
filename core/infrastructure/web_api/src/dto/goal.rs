use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;
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

/// Response DTO for `GET /statistics/{id}/goals/{date}`.
///
/// Mirrors the domain `GoalQueryResult` tri-state but nests the goal payload under
/// `goal_data` so readers of `response.goal_data` keep working.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DailyGoalResponse {
    /// No daily override exists; the default goal applies (if any)
    NotSet,
    /// The goal is explicitly suppressed for this date
    Suppressed,
    /// A concrete goal overrides the default for this date
    Goal { goal_data: serde_json::Value },
}

impl From<GoalQueryResult> for DailyGoalResponse {
    fn from(result: GoalQueryResult) -> Self {
        match result {
            GoalQueryResult::NotSet => Self::NotSet,
            GoalQueryResult::Suppressed => Self::Suppressed,
            GoalQueryResult::Goal(goal_data) => Self::Goal { goal_data },
        }
    }
}

#[cfg(test)]
mod daily_goal_response_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_daily_goal_response_not_set_serialization() {
        let json = serde_json::to_value(DailyGoalResponse::from(GoalQueryResult::NotSet)).unwrap();
        assert_eq!(json, json!({"kind": "not_set"}));
    }

    #[test]
    fn test_daily_goal_response_suppressed_serialization() {
        let json =
            serde_json::to_value(DailyGoalResponse::from(GoalQueryResult::Suppressed)).unwrap();
        assert_eq!(json, json!({"kind": "suppressed"}));
    }

    #[test]
    fn test_daily_goal_response_goal_serialization() {
        let goal = json!({"target_amount": 10, "is_at_least": true});
        let json =
            serde_json::to_value(DailyGoalResponse::from(GoalQueryResult::Goal(goal.clone())))
                .unwrap();
        assert_eq!(json, json!({"kind": "goal", "goal_data": goal}));
    }
}
