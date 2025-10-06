use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitGoal {
    target: bool,
}

impl HabitGoal {
    pub fn new(target: bool) -> Self {
        Self { target }
    }

    pub fn occurred(&self) -> bool {
        self.target
    }
}

impl Goal for HabitGoal {
    type LogValue = bool;

    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage {
        let value = *value.unwrap_or(&false);
        (self.target == value).into()
    }
}
