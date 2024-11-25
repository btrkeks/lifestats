use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::time_frame_list::{TimeFrame, TimeFrameList};
use crate::statistic::{StatisticDefinition};
use anyhow::{anyhow, Result};
use crate::objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;

pub type TimeSpansStatistic = StatisticDefinition<TimeFrame, TimeFrameList, TimeFrameListUpdatePolicy, TimeSpansGoal>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSpansGoal {
    target: TimeFrameList,
    efficiency: f64,  // Target percentage between 0 and 1.0
}

impl TimeSpansGoal {
    pub fn new(target_frames: TimeFrameList, efficiency: f64) -> Result<Self> {
        if efficiency < 0.0 || efficiency > 1.0 {
            return Err(anyhow!("Efficiency is out of range: {}", efficiency));
        }

        Ok(Self {
            target: target_frames,
            efficiency,
        })
    }
}

impl Goal<TimeFrameList> for TimeSpansGoal {
    fn get_percentage_achieved(&self, value: &TimeFrameList) -> f64 {
        let target_duration = self.target.get_total_duration().num_seconds() as f64 * self.efficiency;
        let actual_duration = value.get_total_duration().num_seconds() as f64;
        if target_duration > 0.0 { actual_duration / target_duration } else { 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_time_frame(start_offset: i64, duration_seconds: i64) -> TimeFrame {
        let start = Utc::now() + Duration::seconds(start_offset);
        let end = start + Duration::seconds(duration_seconds);
        TimeFrame::new(start.time(), end.time()).unwrap()
    }

    fn create_time_frame_list(frames: Vec<(i64, i64)>) -> TimeFrameList {
        let mut time_frame_list = TimeFrameList::default();
        frames
            .into_iter()
            .map(|(start, duration)| create_time_frame(start, duration))
            .for_each(|frame| time_frame_list.append_frame(frame).unwrap());
        time_frame_list
    }

    #[test]
    fn test_exact_match() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal::new(target_frames, 1.0).unwrap();
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 1.0);
    }

    #[test]
    fn test_half_duration() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 1.0,
        };
        let actual_frames = create_time_frame_list(vec![(0, 1800)]); // 30 minutes

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 0.5);
    }

    #[test]
    fn test_double_duration() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 1.0,
        };
        let actual_frames = create_time_frame_list(vec![(0, 7200)]); // 2 hours

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 2.0);
    }

    #[test]
    fn test_with_efficiency() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 0.5,
        };
        let actual_frames = create_time_frame_list(vec![(0, 1800)]); // 30 minutes

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 1.0);
    }

    #[test]
    fn test_multiple_frames() {
        let target_frames = create_time_frame_list(vec![(0, 1800), (2000, 1800)]); // Two 30-minute frames
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 1.0,
        };
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // One 1-hour frame

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 1.0);
    }

    #[test]
    fn test_empty_actual_frames() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 1.0,
        };
        let actual_frames = create_time_frame_list(vec![]); // Empty list

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 0.0);
    }

    #[test]
    fn test_zero_efficiency() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 0.0,
        };
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour

        assert_eq!(goal.get_percentage_achieved(&actual_frames), 1.0);
    }
}