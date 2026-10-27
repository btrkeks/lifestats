use crate::cqrs::Command;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

use super::super::set_daily_goal::GoalPayload;

/// Command to set a new default goal for a statistic
///
/// This uses the same GoalPayload as SetDailyGoalCommand since the
/// goal structure is identical - only the scope (default vs daily) differs.
pub struct SetDefaultGoalCommand {
    pub statistic_id: StatisticId,
    pub goal: GoalPayload,
    /// First day the goal applies to; `None` means today. Earlier days keep
    /// the default goal they had.
    pub valid_since: Option<NaiveDate>,
}

impl Command for SetDefaultGoalCommand {
    type Result = ();
}
