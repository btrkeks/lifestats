use crate::cqrs::Command;
use lifestats_core::shared::ids::StatisticId;

use super::super::set_daily_goal::GoalPayload;

/// Command to set a new default goal for a statistic
///
/// This uses the same GoalPayload as SetDailyGoalCommand since the
/// goal structure is identical - only the scope (default vs daily) differs.
pub struct SetDefaultGoalCommand {
    pub statistic_id: StatisticId,
    pub goal: GoalPayload,
}

impl Command for SetDefaultGoalCommand {
    type Result = ();
}
