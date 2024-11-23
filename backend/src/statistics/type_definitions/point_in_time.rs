use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::type_definitions::{Statistic, StatisticMetadata, StatisticTypeSpecific};
use crate::statistics::update_policy::FixedValueUpdatePolicy;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PointInTimeStatistic {
    metadata: StatisticMetadata,
    goal: Option<FinishedInTimeGoal>,
}

impl Statistic for PointInTimeStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }
}

impl StatisticTypeSpecific for PointInTimeStatistic {
    type LogValue = NaiveTime;
    type RetrieveValue = NaiveTime;
    type UpdatePolicy = FixedValueUpdatePolicy;
    type GoalType = FinishedInTimeGoal;

    fn get_goal(&self) -> &Option<Self::GoalType> {
        &self.goal
    }

    // fn parse_string_to_value(&self, string: &str) -> anyhow::Result<Self::LogValue> {
    //     parse_unknown_time_format(string).with_context(|| {
    //         anyhow!(
    //             "Parsing {} into a NaiveTime for id {:?}",
    //             string,
    //             self.get_id()
    //         )
    //     })
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedInTimeGoal {
    target_time: NaiveTime,
}

impl Goal<NaiveTime> for FinishedInTimeGoal {
    fn get_percentage_achieved(&self, value: &NaiveTime) -> f64 {
        let diff = self.target_time.num_seconds_from_midnight() as f64
            - value.num_seconds_from_midnight() as f64;
        // Calculate percentage based on hourly difference, capped at 0.0
        (1.0 + diff / 3600.0).max(0.0)
    }
}