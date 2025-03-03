/// Logs the time spent on a task
/// Examples for possible bad tasks:
/// - Amount of internet usage
/// /// - Amount of phone usage
use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::my_duration::MyDuration;
use crate::statistic::objects::update_policy::amount::AmountUpdatePolicy;
use crate::statistic::statistic::Statistic;
use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use transfer_objects::raw_goal::RawGoal;

pub type TimeAmountStatistic =
    StatisticTemplate<MyDuration, MyDuration, AmountUpdatePolicy, TimeAmountGoal>;
pub trait TimeAmountLogsRepository = LoggingDataRepositoryTrait<StoredType = MyDuration>;
pub trait TimeAmountGoalsRepository =
    GoalRepositoryTrait<StoredValue = MyDuration, GoalType = TimeAmountGoal>;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct TimeAmountGoal {
    target: MyDuration,
    is_good: bool,
}

impl TimeAmountGoal {
    pub fn new(target: MyDuration, is_good: bool) -> Self {
        Self { target, is_good }
    }
}

impl Goal<MyDuration> for TimeAmountGoal {
    // fn set_target(mut self, target: MyDuration) -> Self {
    //     self.target = target;
    //     self
    // }

    fn new_with_target(target: MyDuration) -> Self {
        Self {
            target,
            is_good: true,
        }
    }

    fn set_extra_data_1(mut self, extra_data_1: &str) -> anyhow::Result<Self> {
        self.is_good = extra_data_1.parse()?;
        Ok(self)
    }

    fn get_extra_data_1(&self) -> Option<String> {
        Some(self.is_good.to_string())
    }

    fn get_value(&self) -> &MyDuration {
        &self.target
    }

    /// Interprets 'None' as 0 seconds
    fn get_percentage_achieved(&self, value: Option<&MyDuration>) -> f64 {
        let value_secs = value.map(|duration| duration.secs()).unwrap_or(0);
        let target_secs = self.target.secs();

        if self.is_good {
            value_secs as f64 / target_secs as f64
        } else {
            // Linear with cutoff: 100% at target, 0% at 2x target or more
            1.0 - (value_secs as f64 - target_secs as f64) / target_secs as f64
        }
        .max(0.0)
    }

    fn into_raw_goal(self) -> RawGoal {
        RawGoal {
            target: self.target.to_string(),
            extra_data_1: Some(self.is_good.to_string()),
        }
    }
}

impl IntegerStatistic for TimeAmountStatistic {
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
                    .as_ref()
                    .map(|duration| duration.secs() as i64);
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
                    .as_ref()
                    .map(|goal| goal.target.secs() as i64);
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

    #[test]
    fn test_percentage_achieved_good_goal() {
        let goal = TimeAmountGoal::new(MyDuration::from_mins(60), true); // 1 hour goal

        // Test exact values
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(60))),
            1.0
        ); // 100% at target
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(30))),
            0.5
        ); // 50% at half target
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(120))),
            2.0
        ); // 200% at double target

        // Test zero
        assert_eq!(goal.get_percentage_achieved(Some(&MyDuration::from_mins(0))), 0.0);
    }

    #[test]
    fn test_percentage_achieved_bad_goal() {
        let goal = TimeAmountGoal::new(MyDuration::from_mins(60), false); // 1 hour limit

        // At target
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(60))),
            1.0
        );

        // At 1.5x target (90 mins) - should be 50%
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(90))),
            0.5
        );

        // At 2x target (120 mins) - should be 0%
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(120))),
            0.0
        );

        // Beyond 2x target - should stay at 0%
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(180))),
            0.0
        );
    }

    #[test]
    fn test_percentage_achieved_bad_goal_under_target() {
        let goal = TimeAmountGoal::new(MyDuration::from_mins(60), false);

        // Below target should give > 100%
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(30))),
            1.5
        ); // 30 mins = 150%
        assert_eq!(goal.get_percentage_achieved(Some(&MyDuration::from_mins(0))), 2.0);
        // 0 mins = 200%
    }

    #[test]
    fn test_percentage_achieved_with_seconds_precision() {
        let goal = TimeAmountGoal::new(MyDuration::from_secs(3600), false); // 1 hour

        // Test with second-level precision
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_secs(3600 + 1800))), // 1.5 hours
            0.5
        );

        // Test with very small differences
        assert!(goal.get_percentage_achieved(Some(&MyDuration::from_secs(3601))) < 1.0);
    }

    #[test]
    fn test_different_time_units() {
        let goal = TimeAmountGoal::new(MyDuration::from_hours(1), false);

        // Test that minutes and hours are converted correctly
        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_mins(90))), // 1.5 hours
            0.5
        );

        assert_eq!(
            goal.get_percentage_achieved(Some(&MyDuration::from_secs(7200))), // 2 hours
            0.0
        );
    }
}
