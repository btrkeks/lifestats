use crate::cqrs::Command;
use chrono::{NaiveDate, NaiveTime};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::value_objects::update_policies::{
    amount::AmountUpdatePolicy, fixed_value::FixedValueUpdatePolicy,
    time_frame_list::TimeFrameListUpdatePolicy,
};

/// Strongly-typed payload for log values
///
/// Each variant corresponds to a specific statistic type and carries
/// the correctly-typed value along with its update policy.
/// This eliminates string parsing within the application layer.
#[derive(Debug, Clone)]
pub enum LogValuePayload {
    Integer {
        value: i64,
        update_policy: AmountUpdatePolicy,
    },
    Habit {
        value: bool,
        update_policy: FixedValueUpdatePolicy,
    },
    PointInTime {
        value: NaiveTime,
        update_policy: FixedValueUpdatePolicy,
    },
    TimeFrames {
        value: TimeFrameList,
        update_policy: TimeFrameListUpdatePolicy,
    },
}

/// Command to log a value for a statistic on a specific date
///
/// This command now uses strongly-typed payloads instead of strings,
/// moving parsing responsibility to the adapter layer (web_api, CLI, etc.)
pub struct LogValueCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub payload: LogValuePayload,
    pub statistic: AnyStatistic,
}

impl Command for LogValueCommand {
    type Result = ();
}
