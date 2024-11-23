use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::objects::{TimeFrame, TimeFrameList};
use crate::statistics::type_definitions::{Statistic, StatisticMetadata, StatisticTypeSpecific};
use crate::statistics::update_policy::TimeFrameListUpdatePolicy;

const DEFAULT_GOAL_EFFICIENCY: f64 = 1.0;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSpansStatistic {
    metadata: StatisticMetadata,
    goal: Option<TimeSpansGoal>,
}

impl Statistic for TimeSpansStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }
}

impl StatisticTypeSpecific for TimeSpansStatistic {
    type LogValue = TimeFrame;
    type RetrieveValue = TimeFrameList;
    type UpdatePolicy = TimeFrameListUpdatePolicy;
    type GoalType = TimeSpansGoal;

    fn get_goal(&self) -> &Option<Self::GoalType> {
        &self.goal
    }

    // fn parse_string_to_value(&self, string: &str) -> anyhow::Result<Self::LogValue> {
    //     parse_time_frame(string).with_context(|| {
    //         anyhow!(
    //             "Parsing {} into a TimeFrame for id {:?}",
    //             string,
    //             self.get_id()
    //         )
    //     })
    // }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSpansGoal {
    target_frames: TimeFrameList,
    #[serde(default = "default_goal_efficiency")]
    efficiency: f64,  // Target percentage between 0 and 1
}

fn default_goal_efficiency() -> f64 {
    DEFAULT_GOAL_EFFICIENCY
}

impl Goal<TimeFrameList> for TimeSpansGoal {
    fn get_percentage_achieved(&self, value: &TimeFrameList) -> f64 {
        let target_duration = self.target_frames.get_total_duration().num_seconds() as f64 * self.efficiency;
        let actual_duration = value.get_total_duration().num_seconds() as f64;
        actual_duration / target_duration
    }
}