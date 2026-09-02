use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::percentage::Percentage;

/// Query for per-day goal achievement plus daily-override state over a date range.
///
/// Powers UIs that show a "last N days" strip per statistic and let the user act on
/// a single day (e.g. suppress the goal for a day that was missed for a good reason).
pub struct GetDailyAchievementsQuery {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
}

/// Whether a daily goal override row exists for a date, and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyGoalOverrideKind {
    /// No override row; the default goal applies (if any)
    None,
    /// The goal is explicitly suppressed for this date
    Suppressed,
    /// A concrete daily goal overrides the default
    Goal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyAchievement {
    pub date: NaiveDate,
    /// Whether any goal (default or daily) applied on this day
    pub has_goal: bool,
    /// Whether the applicable goal was achieved
    pub achieved: bool,
    /// Achievement percentage, `None` when no goal applied
    pub percentage_achieved: Option<Percentage>,
    pub override_kind: DailyGoalOverrideKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyAchievementsResult {
    pub statistic_id: StatisticId,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    /// One entry per day in the range, ascending by date
    pub days: Vec<DailyAchievement>,
}

impl Query for GetDailyAchievementsQuery {
    type Result = DailyAchievementsResult;
}
