use std::collections::HashMap;

use crate::cqrs::Command;
use crate::dto::summary::TimelineEvent;

/// Command to calculate allocation percentages for a set of timeline events.
/// Example:
///     "Academics": 35.4,
///     "Japanese": 12.5,
///     "Health": 6.3,
///     "Unlogged": 45.8
pub struct CalculatePercentagesCommand {
    pub timeline_events: Vec<TimelineEvent>,
}

impl Command for CalculatePercentagesCommand {
    type Result = HashMap<String, f64>;
}
