use crate::cqrs::Command;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;

use super::super::set_daily_goal::DailyGoalOverride;

/// Command to give every day of a date range the same daily override
/// (a concrete goal or a suppression), all or nothing.
pub struct SetDailyGoalRangeCommand {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
    pub override_kind: DailyGoalOverride,
}

impl Command for SetDailyGoalRangeCommand {
    type Result = ();
}
