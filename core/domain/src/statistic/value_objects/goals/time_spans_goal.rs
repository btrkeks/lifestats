use crate::shared::percentage::Percentage;
use crate::shared::time_frame_list::TimeFrameList;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSpansGoal {
    target: TimeFrameList,
    efficiency: Percentage, // Target percentage between 0 and 100
}

impl TimeSpansGoal {
    pub fn new(target_frames: TimeFrameList, efficiency: Percentage) -> Self {
        Self {
            target: target_frames,
            efficiency,
        }
    }

    pub fn target(&self) -> &TimeFrameList {
        &self.target
    }

    pub fn efficiency(&self) -> Percentage {
        self.efficiency
    }
}

impl Goal for TimeSpansGoal {
    type LogValue = TimeFrameList;

    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage {
        let actual_duration = value
            .map(|time_frame_list| time_frame_list.get_total_duration().num_seconds() as f64)
            .unwrap_or(0.0);

        let target_duration =
            self.target.get_total_duration().num_seconds() as f64 * self.efficiency.as_fraction();

        if target_duration > 0.0 {
            Percentage::try_from(actual_duration / target_duration).unwrap()
        } else {
            Percentage::try_from(1.0).unwrap()
        }
    }
}
