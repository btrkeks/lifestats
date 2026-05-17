use serde::{Deserialize, Serialize};

/// Response DTO for the current streak endpoint.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreakResponse {
    /// ID of the statistic.
    pub statistic_id: u32,
    /// Current streak length in days.
    pub current_streak_days: u32,
}
