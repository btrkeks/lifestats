use crate::cqrs::Command;
use lifestats_core::shared::ids::StatisticId;

/// Command to remove a statistic's default goal from today on. Earlier days keep
/// the default goal they had.
pub struct DeleteDefaultGoalCommand {
    pub statistic_id: StatisticId,
}

impl Command for DeleteDefaultGoalCommand {
    type Result = ();
}
