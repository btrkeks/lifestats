use crate::cqrs::Command;
use lifestats_core::shared::ids::StatisticId;

/// Command to delete a statistic's metadata and associated records.
pub struct DeleteStatisticCommand {
    pub statistic_id: StatisticId,
}

impl Command for DeleteStatisticCommand {
    type Result = ();
}
