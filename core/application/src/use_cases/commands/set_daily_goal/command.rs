use crate::cqrs::Command;
use chrono::{NaiveDate, NaiveTime};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::shared::time_frame_list::TimeFrameList;

/// Strongly-typed goal payload
///
/// Each variant corresponds to a specific statistic type and carries
/// the correctly-typed goal data. This eliminates HashMap parsing.
#[derive(Debug, Clone)]
pub enum GoalPayload {
    IntegerAmount {
        target: i64,
        is_at_least: bool,
    },
    Habit {
        occurred: bool,
    },
    FinishedInTime {
        deadline: NaiveTime,
    },
    TimeSpans {
        target: TimeFrameList,
        efficiency: Percentage,
    },
    SecondsAmount {
        target: Seconds,
        is_at_least: bool,
    },
}

/// Command to set a goal for a specific date
pub struct SetDailyGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub goal: GoalPayload,
}

impl Command for SetDailyGoalCommand {
    type Result = ();
}
