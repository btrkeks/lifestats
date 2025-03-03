use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::my_time::MyTime;
use crate::statistic::objects::update_policy::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::statistic::Statistic;
use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use transfer_objects::raw_goal::RawGoal;

pub type PointInTimeStatistic =
    StatisticTemplate<MyTime, MyTime, FixedValueUpdatePolicy, FinishedInTimeGoal>;
pub trait PointInTimeLogsRepository = LoggingDataRepositoryTrait<StoredType = MyTime>;
pub trait PointInTimeGoalsRepository =
    GoalRepositoryTrait<StoredValue = MyTime, GoalType = FinishedInTimeGoal>;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FinishedInTimeGoal {
    target: MyTime,
}

impl FinishedInTimeGoal {
    pub fn new(target: MyTime) -> Self {
        Self { target }
    }
}

impl Goal<MyTime> for FinishedInTimeGoal {
    // fn set_target(mut self, target: MyTime) -> Self {
    //     self.target = target;
    //     self
    // }

    fn new_with_target(target: MyTime) -> Self {
        Self { target }
    }

    fn set_extra_data_1(self, _extra_data_1: &str) -> anyhow::Result<Self> {
        Ok(self)
    }

    fn get_extra_data_1(&self) -> Option<String> {
        None
    }

    fn get_value(&self) -> &MyTime {
        &self.target
    }

    fn get_percentage_achieved(&self, value: Option<&MyTime>) -> f64 {
        if let Some(value) = value {
            let diff = self.target.num_seconds_from_midnight() as f64
                - value.num_seconds_from_midnight() as f64;
            (1.0 + diff / 3600.0).max(0.0)
        } else {
            // There is no canonical way to have a default value here
            0.0
        }
    }

    fn into_raw_goal(self) -> RawGoal {
        RawGoal {
            target: self.target.to_string(),
            extra_data_1: None,
        }
    }
}

impl IntegerStatistic for PointInTimeStatistic {
    fn get_integer_data_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<Vec<DateValue<Option<i64>>>> {
        self._get_values_for_closed_range(start_date, end_date)?
            .into_iter()
            .map(|date_value| {
                let value = date_value
                    .value
                    .map(|time| time.num_seconds_from_midnight() as i64);
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
    ) -> anyhow::Result<Vec<DateValue<Option<i64>>>> {
        self._get_goals_for_closed_range(start_date, end_date)?
            .into_iter()
            .map(|date_value| {
                let value = date_value
                    .value
                    .map(|goal| goal.target.num_seconds_from_midnight() as i64);
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
    use chrono::NaiveTime;

    #[test]
    fn test_exactly_on_target() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());

        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 1.0);
    }

    #[test]
    fn test_one_hour_early() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(13, 0, 0).unwrap());

        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 2.0);
    }

    #[test]
    fn test_half_hour_early() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(13, 30, 0).unwrap());

        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 1.5);
    }

    #[test]
    fn test_fifteen_minutes_late() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(14, 15, 0).unwrap());

        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 0.75);
    }

    #[test]
    fn test_one_hour_late() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(15, 0, 0).unwrap());

        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 0.0);
    }

    #[test]
    fn test_multiple_hours_late() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(17, 0, 0).unwrap());

        // Even though the calculation would yield -2.0, it should be capped at 0.0
        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 0.0);
    }

    #[test]
    fn test_near_midnight_boundary() {
        let target_time = MyTime(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(23, 59, 58).unwrap());

        // One second early should be a very small increase over 100%
        assert!(goal.get_percentage_achieved(Some(&actual_time)) > 1.0);
        assert!(goal.get_percentage_achieved(Some(&actual_time)) < 1.001);
    }

    #[test]
    fn test_extreme_earliness() {
        let target_time = MyTime(NaiveTime::from_hms_opt(23, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        // 23 hours early should result in a large percentage
        assert_eq!(goal.get_percentage_achieved(Some(&actual_time)), 24.0);
    }

    #[test]
    fn test_almost_zero_percent() {
        let target_time = MyTime(NaiveTime::from_hms_opt(14, 0, 0).unwrap());
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = MyTime(NaiveTime::from_hms_opt(14, 59, 59).unwrap());

        // Almost an hour late should be very close to 0% but not quite
        let result = goal.get_percentage_achieved(Some(&actual_time));
        assert!(result > 0.0);
        assert!(result < 0.01);
    }
}
