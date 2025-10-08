use crate::cqrs::Command;
use lifestats_core::shared::ids::StatisticId;

/// Command to delete the default goal associated with a statistic.
pub struct DeleteDefaultGoalCommand {
    pub statistic_id: StatisticId,
}

impl Command for DeleteDefaultGoalCommand {
    type Result = ();
}
