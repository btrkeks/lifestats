use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::update_policies::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::{StatisticDefinition};

pub type PointInTimeStatistic = StatisticDefinition<NaiveTime, NaiveTime, FixedValueUpdatePolicy, FinishedInTimeGoal>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishedInTimeGoal {
    target: NaiveTime,
}

impl FinishedInTimeGoal {
    pub fn new(target: NaiveTime) -> Self {
        Self {
            target,
        }
    }
}

impl Goal<NaiveTime> for FinishedInTimeGoal {
    fn get_percentage_achieved(&self, value: &NaiveTime) -> f64 {
        let diff = self.target.num_seconds_from_midnight() as f64
            - value.num_seconds_from_midnight() as f64;
        (1.0 + diff / 3600.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_exactly_on_target() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();

        assert_eq!(goal.get_percentage_achieved(&actual_time), 1.0);
    }

    #[test]
    fn test_one_hour_early() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(13, 0, 0).unwrap();

        assert_eq!(goal.get_percentage_achieved(&actual_time), 2.0);
    }

    #[test]
    fn test_half_hour_early() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(13, 30, 0).unwrap();

        assert_eq!(goal.get_percentage_achieved(&actual_time), 1.5);
    }

    #[test]
    fn test_fifteen_minutes_late() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(14, 15, 0).unwrap();

        assert_eq!(goal.get_percentage_achieved(&actual_time), 0.75);
    }

    #[test]
    fn test_one_hour_late() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(15, 0, 0).unwrap();

        assert_eq!(goal.get_percentage_achieved(&actual_time), 0.0);
    }

    #[test]
    fn test_multiple_hours_late() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        // Even though the calculation would yield -2.0, it should be capped at 0.0
        assert_eq!(goal.get_percentage_achieved(&actual_time), 0.0);
    }

    #[test]
    fn test_near_midnight_boundary() {
        let target_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(23, 59, 58).unwrap();

        // One second early should be a very small increase over 100%
        assert!(goal.get_percentage_achieved(&actual_time) > 1.0);
        assert!(goal.get_percentage_achieved(&actual_time) < 1.001);
    }

    #[test]
    fn test_extreme_earliness() {
        let target_time = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        // 23 hours early should result in a large percentage
        assert_eq!(goal.get_percentage_achieved(&actual_time), 24.0);
    }

    #[test]
    fn test_almost_zero_percent() {
        let target_time = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let goal = FinishedInTimeGoal::new(target_time);
        let actual_time = NaiveTime::from_hms_opt(14, 59, 59).unwrap();

        // Almost an hour late should be very close to 0% but not quite
        let result = goal.get_percentage_achieved(&actual_time);
        assert!(result > 0.0);
        assert!(result < 0.01);
    }
}