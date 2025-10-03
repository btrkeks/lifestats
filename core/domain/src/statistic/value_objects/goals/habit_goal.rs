use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;

#[derive(Clone)]
pub struct HabitGoal {
    target: bool,
}

impl Goal for HabitGoal {
    type LogValue = bool;

    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage {
        let value = *value.unwrap_or(&false);
        (self.target == value).into()
    }
}