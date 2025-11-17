use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FinishedInTimeGoal {
    target: NaiveTime,
}

impl FinishedInTimeGoal {
    pub fn new(target: NaiveTime) -> Self {
        Self { target }
    }
}

impl Goal for FinishedInTimeGoal {
    type LogValue = NaiveTime;

    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage {
        if let Some(value) = value {
            let diff = self.target.num_seconds_from_midnight() as f64
                - value.num_seconds_from_midnight() as f64;

            let score = if diff >= 0.0 {
                // Early or on-time: linear scoring capped at 200%
                (1.0 + diff / 3600.0).min(2.0)
            } else {
                // Late: exponential decay with 1-hour half-life
                let late_hours = -diff / 3600.0;
                (-late_hours).exp()
            };

            Percentage::try_from(score).unwrap()
        } else {
            // There is no canonical way to have a default value here
            Percentage::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_time(hour: u32, min: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, min, 0).unwrap()
    }

    #[test]
    fn test_on_time_exactly_100_percent() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(21, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        assert_eq!(percentage.as_num(), 100);
    }

    #[test]
    fn test_one_hour_early_is_200_percent() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(20, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        assert_eq!(percentage.as_num(), 200);
    }

    #[test]
    fn test_two_hours_early_capped_at_200_percent() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(19, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        assert_eq!(percentage.as_num(), 200);
    }

    #[test]
    fn test_three_hours_early_capped_at_200_percent() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(18, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        assert_eq!(percentage.as_num(), 200);
    }

    #[test]
    fn test_thirty_minutes_late_exponential_decay() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(21, 30);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // exp(-0.5) ≈ 0.606, which is 60.6%
        assert_eq!(percentage.as_num(), 61);
    }

    #[test]
    fn test_one_hour_late_exponential_decay() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(22, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // exp(-1) ≈ 0.368, which is 36.8%
        assert_eq!(percentage.as_num(), 37);
    }

    #[test]
    fn test_two_hours_late_exponential_decay() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(23, 0);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // exp(-2) ≈ 0.135, which is 13.5%
        assert_eq!(percentage.as_num(), 14);
    }

    #[test]
    fn test_four_hours_late_exponential_decay() {
        let goal = FinishedInTimeGoal::new(make_time(14, 0));
        let logged = make_time(18, 0); // 4 hours after target
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // exp(-4) ≈ 0.018, which is 1.8%
        assert_eq!(percentage.as_num(), 2);
    }

    #[test]
    fn test_no_value_logged_returns_zero() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let percentage = goal.get_percentage_achieved(None);
        assert_eq!(percentage.as_num(), 0);
    }

    #[test]
    fn test_thirty_minutes_early() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(20, 30);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // 1.0 + 0.5 = 1.5 = 150%
        assert_eq!(percentage.as_num(), 150);
    }

    #[test]
    fn test_fifteen_minutes_late() {
        let goal = FinishedInTimeGoal::new(make_time(21, 0));
        let logged = make_time(21, 15);
        let percentage = goal.get_percentage_achieved(Some(&logged));
        // exp(-0.25) ≈ 0.779, which is 77.9%
        assert_eq!(percentage.as_num(), 78);
    }
}
