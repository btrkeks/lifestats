use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use chrono::{NaiveDateTime, Timelike};

#[derive(Debug, Default, Clone)]
pub struct FinishedInTimeGoal {
    target: NaiveDateTime,
}

impl FinishedInTimeGoal {
    pub fn new(target: NaiveDateTime) -> Self {
        Self { target }
    }
}

impl Goal for FinishedInTimeGoal {
    type LogValue = NaiveDateTime;

    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage {
        if let Some(value) = value {
            let diff = self.target.num_seconds_from_midnight() as f64
                - value.num_seconds_from_midnight() as f64;
            Percentage::try_from((1.0 + diff / 3600.0).max(0.0)).unwrap()
        } else {
            // There is no canonical way to have a default value here
            Percentage::default()
        }
    }
}
