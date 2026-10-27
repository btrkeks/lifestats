use lifestats_application::services::DayValue;
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

/// The JSON form of a logged day value: a number for counts (IntegerAmount,
/// Summing) and durations in seconds (TimeSpans, TimeDifference), a boolean for
/// habits, and `"HH:MM:SS"` for PointInTime.
pub fn day_value_json(value: DayValue) -> serde_json::Value {
    match value {
        DayValue::Amount(amount) => amount.into(),
        DayValue::Occurred(occurred) => occurred.into(),
        DayValue::Time(time) => time.format("%H:%M:%S").to_string().into(),
        DayValue::Duration(seconds) => i64::from(seconds).into(),
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
    /// What was logged that day (see `day_value_json`), null when nothing was
    pub value: Option<serde_json::Value>,
    /// The goal that applied that day, same shape as `GET /statistics/{id}/goals/default`
    /// `goal_data`; null when no goal applied
    pub goal: Option<serde_json::Value>,
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
            value: Some(day_value_json(DayValue::Amount(244))),
            goal: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"override\":\"suppressed\""));
        assert!(json.contains("\"percentage_achieved\":null"));
        assert!(json.contains("\"value\":244"));
        assert!(json.contains("\"goal\":null"));
    }

    #[test]
    fn test_day_value_json_per_type() {
        use chrono::NaiveTime;
        use lifestats_core::shared::seconds::Seconds;

        assert_eq!(
            day_value_json(DayValue::Occurred(true)),
            serde_json::json!(true)
        );
        assert_eq!(
            day_value_json(DayValue::Time(NaiveTime::from_hms_opt(22, 37, 0).unwrap())),
            serde_json::json!("22:37:00")
        );
        assert_eq!(
            day_value_json(DayValue::Duration(Seconds::from_i64(5940))),
            serde_json::json!(5940)
        );
    }
}
