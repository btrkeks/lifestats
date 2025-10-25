use crate::cqrs::Command;
use lifestats_core::shared::ids::StatisticId;

// Re-export the shared StatisticType and related types from dto module
pub use crate::dto::{StatisticType, StatisticTypeParseError};

// Type alias for backward compatibility
pub type StatisticTypeDto = StatisticType;

/// Command to create a new statistic metadata entry.
pub struct CreateStatisticCommand {
    pub name: String,
    pub category: String,
    pub statistic_type: StatisticTypeDto,
    pub source_statistic_ids: Option<Vec<StatisticId>>,
}

impl Command for CreateStatisticCommand {
    type Result = lifestats_core::statistic::any_statistic::AnyStatistic;
}
