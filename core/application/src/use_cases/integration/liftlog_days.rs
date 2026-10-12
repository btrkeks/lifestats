use chrono::NaiveDateTime;
use lifestats_core::shared::ids::StatisticId;

/// A finished LiftLog workout as the phone reports it, in the device's local
/// wall-clock time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftLogWorkout {
    pub started_at: NaiveDateTime,
    pub finished_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftLogDaysSyncResult {
    pub statistic_id: StatisticId,
    pub days_updated: usize,
    pub days_cleared: usize,
    pub days_unchanged: usize,
}

impl LiftLogDaysSyncResult {
    pub fn changed(&self) -> bool {
        self.days_updated + self.days_cleared > 0
    }
}
