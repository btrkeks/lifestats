use crate::cqrs::Command;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

/// Command to remove a daily goal override for a specific date.
pub struct DeleteDailyGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

impl Command for DeleteDailyGoalCommand {
    type Result = ();
}
