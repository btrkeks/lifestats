use serde::{Deserialize, Serialize};

/// Represents the result of querying a goal for a specific date.
///
/// This enum captures the three possible states:
/// - `NotSet`: No daily override exists (default goal should apply if it exists)
/// - `Suppressed`: Goal is explicitly suppressed for this date (no goal applies)
/// - `Goal`: A concrete goal is set for this date
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GoalQueryResult {
    /// No override - use default goal if available
    #[serde(rename = "not_set")]
    NotSet,
    /// Explicitly suppressed - no goal applies
    Suppressed,
    /// Concrete goal data
    Goal(serde_json::Value),
}

impl GoalQueryResult {
    /// Returns `true` if the goal is suppressed.
    pub fn is_suppressed(&self) -> bool {
        matches!(self, GoalQueryResult::Suppressed)
    }

    /// Returns `true` if a goal is set (not suppressed and not missing).
    pub fn is_set(&self) -> bool {
        matches!(self, GoalQueryResult::Goal(_))
    }

    /// Extracts the goal data if one is set.
    pub fn goal_data(&self) -> Option<&serde_json::Value> {
        match self {
            GoalQueryResult::Goal(data) => Some(data),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_suppressed() {
        assert!(GoalQueryResult::Suppressed.is_suppressed());
        assert!(!GoalQueryResult::NotSet.is_suppressed());
        assert!(!GoalQueryResult::Goal(json!({})).is_suppressed());
    }

    #[test]
    fn test_is_set() {
        assert!(GoalQueryResult::Goal(json!({ "target": 100 })).is_set());
        assert!(!GoalQueryResult::NotSet.is_set());
        assert!(!GoalQueryResult::Suppressed.is_set());
    }

    #[test]
    fn test_goal_data_extraction() {
        let goal = GoalQueryResult::Goal(json!({ "target": 100 }));
        assert_eq!(goal.goal_data(), Some(&json!({ "target": 100 })));
        assert_eq!(GoalQueryResult::NotSet.goal_data(), None);
        assert_eq!(GoalQueryResult::Suppressed.goal_data(), None);
    }

    #[test]
    fn test_serialization() {
        let not_set = GoalQueryResult::NotSet;
        let json = serde_json::to_value(&not_set).unwrap();
        assert_eq!(json, json!({ "kind": "not_set" }));

        let suppressed = GoalQueryResult::Suppressed;
        let json = serde_json::to_value(&suppressed).unwrap();
        assert_eq!(json, json!({ "kind": "suppressed" }));

        // Goal variant flattens the JSON object with the kind tag
        let goal = GoalQueryResult::Goal(json!({ "target": 100 }));
        let json = serde_json::to_value(&goal).unwrap();
        assert_eq!(json, json!({ "kind": "goal", "target": 100 }));
    }
}
