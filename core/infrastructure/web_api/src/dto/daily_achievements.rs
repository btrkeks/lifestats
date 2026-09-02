use lifestats_application::use_cases::queries::get_daily_achievements::DailyGoalOverrideKind;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Query parameters for `GET /statistics/{id}/achievements` (same names as `/score`)
#[derive(Debug, Deserialize, Validate)]
pub struct DailyAchievementsParams {
    /// Start date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub start: String,
    /// End date (YYYY-MM-DD)
    #[validate(length(min = 1))]
    pub end: String,
}

/// Daily override state for one day
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DailyGoalOverrideResponse {
    /// No override row; the default goal applies (if any)
    None,
    /// The goal is explicitly suppressed for this date
    Suppressed,
    /// A concrete daily goal overrides the default
    Goal,
}

impl From<DailyGoalOverrideKind> for DailyGoalOverrideResponse {
    fn from(kind: DailyGoalOverrideKind) -> Self {
        match kind {
            DailyGoalOverrideKind::None => Self::None,
            DailyGoalOverrideKind::Suppressed => Self::Suppressed,
            DailyGoalOverrideKind::Goal => Self::Goal,
        }
    }
}

/// One day of goal achievement
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyAchievementResponse {
    /// YYYY-MM-DD
    pub date: String,
    /// Whether any goal (default or daily) applied on this day
    pub has_goal: bool,
    /// Whether the applicable goal was achieved
    pub achieved: bool,
    /// Achievement percentage 0-255 (can exceed 100), null when no goal applied
    pub percentage_achieved: Option<u8>,
    /// `override` is a reserved keyword in Rust, hence the rename
    #[serde(rename = "override")]
    pub override_kind: DailyGoalOverrideResponse,
}

/// Response DTO for `GET /statistics/{id}/achievements`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyAchievementsResponse {
    pub statistic_id: u32,
    /// Start date of the range (YYYY-MM-DD)
    pub start_date: String,
    /// End date of the range (YYYY-MM-DD)
    pub end_date: String,
    /// One entry per day in the range, ascending by date
    pub days: Vec<DailyAchievementResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_achievements_params_empty_start_invalid() {
        let params = DailyAchievementsParams {
            start: String::new(),
            end: "2025-01-31".to_string(),
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_daily_achievement_response_serializes_override_key() {
        let response = DailyAchievementResponse {
            date: "2025-01-03".to_string(),
            has_goal: false,
            achieved: false,
            percentage_achieved: None,
            override_kind: DailyGoalOverrideResponse::Suppressed,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"override\":\"suppressed\""));
        assert!(json.contains("\"percentage_achieved\":null"));
    }
}
