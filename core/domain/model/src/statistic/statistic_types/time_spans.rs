use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::time_frame_list::{TimeFrame, TimeFrameList};
use crate::statistic::objects::update_policy::time_frame_list::TimeFrameListUpdatePolicy;
use crate::statistic::statistic::Statistic;
use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use transfer_objects::raw_goal::RawGoal;

pub type TimeSpansStatistic =
    StatisticTemplate<TimeFrame, TimeFrameList, TimeFrameListUpdatePolicy, TimeSpansGoal>;
pub trait TimeSpansLogsRepository = LoggingDataRepositoryTrait<StoredType = TimeFrameList>;
pub trait TimeSpansGoalsRepository =
    GoalRepositoryTrait<StoredValue = TimeFrameList, GoalType = TimeSpansGoal>;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct TimeSpansGoal {
    target: TimeFrameList,
    efficiency: u8, // Target percentage between 0 and 100
}

impl TimeSpansGoal {
    pub fn new(target_frames: TimeFrameList, efficiency: u8) -> Result<Self> {
        if efficiency > 100 {
            return Err(anyhow!("Efficiency is out of range: {}", efficiency));
        }

        Ok(Self {
            target: target_frames,
            efficiency,
        })
    }
}

impl Goal<TimeFrameList> for TimeSpansGoal {
    fn new_with_target(target: TimeFrameList) -> Self {
        Self {
            target,
            efficiency: 100,
        }
    }

    fn set_extra_data_1(mut self, extra_data_1: &str) -> Result<Self> {
        self.efficiency = extra_data_1.parse()?;
        Ok(self)
    }

    fn get_extra_data_1(&self) -> Option<String> {
        Some(self.efficiency.to_string())
    }

    fn get_value(&self) -> &TimeFrameList {
        &self.target
    }

    fn get_percentage_achieved(&self, value: Option<&TimeFrameList>) -> f64 {
        let actual_duration = value
            .map(|time_frame_list| time_frame_list.get_total_duration().num_seconds() as f64)
            .unwrap_or(0.0);

        let target_duration = self.target.get_total_duration().num_seconds() as f64
            * (self.efficiency as f64 / 100.0);

        if target_duration > 0.0 {
            actual_duration / target_duration
        } else {
            1.0
        }
    }

    fn into_raw_goal(self) -> RawGoal {
        RawGoal {
            target: self.target.to_string(),
            extra_data_1: Some(self.efficiency.to_string()),
        }
    }
}

impl IntegerStatistic for TimeSpansStatistic {
    fn get_integer_data_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<i64>>>> {
        self._get_values_for_closed_range(start_date, end_date)?
            .into_iter()
            .map(|date_value| {
                let value = date_value
                    .value
                    .map(|time_frame_list| time_frame_list.get_total_duration().num_seconds());
                Ok(DateValue {
                    date: date_value.date,
                    value,
                })
            })
            .collect()
    }

    fn get_integer_goals_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<i64>>>> {
        self._get_goals_for_closed_range(start_date, end_date)?
            .into_iter()
            .map(|date_value| {
                let value = date_value.value.map(|time_frame_goal| {
                    time_frame_goal.target.get_total_duration().num_seconds()
                        * time_frame_goal.efficiency as i64
                        / 100
                });

                Ok(DateValue {
                    date: date_value.date,
                    value,
                })
            })
            .collect()
    }

    // fn get_integer_scores_closed_range(
    //     &self,
    //     start_date: NaiveDate,
    //     end_date: NaiveDate,
    // ) -> anyhow::Result<Vec<DateValue<Option<f64>>>> {
    //     self.get_scores_for_closed_range(start_date, end_date)
    // }

    fn get_y_axis_conversion(&self) -> YAxisConversion {
        YAxisConversion::SecondsToTime
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
        let goal = TimeSpansGoal::new(target_frames, 100).unwrap();
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 1.0);
    }

    #[test]
    fn test_half_duration() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 100,
        };
        let actual_frames = create_time_frame_list(vec![(0, 1800)]); // 30 minutes

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 0.5);
    }

    #[test]
    fn test_double_duration() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 100,
        };
        let actual_frames = create_time_frame_list(vec![(0, 7200)]); // 2 hours

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 2.0);
    }

    #[test]
    fn test_with_efficiency() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 50,
        };
        let actual_frames = create_time_frame_list(vec![(0, 1800)]); // 30 minutes

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 1.0);
    }

    #[test]
    fn test_multiple_frames() {
        let target_frames = create_time_frame_list(vec![(0, 1800), (2000, 1800)]); // Two 30-minute frames
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 100,
        };
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // One 1-hour frame

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 1.0);
    }

    #[test]
    fn test_empty_actual_frames() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 100,
        };
        let actual_frames = create_time_frame_list(vec![]); // Empty list

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 0.0);
    }

    #[test]
    fn test_zero_efficiency() {
        let target_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour
        let goal = TimeSpansGoal {
            target: target_frames,
            efficiency: 0,
        };
        let actual_frames = create_time_frame_list(vec![(0, 3600)]); // 1 hour

        assert_eq!(goal.get_percentage_achieved(Some(&actual_frames)), 1.0);
    }
}
