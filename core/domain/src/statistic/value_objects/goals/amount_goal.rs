use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountGoal<T: Default + Clone + Into<i64> + Copy> {
    target_amount: T,
    is_good: bool,
}

impl<T: Default + Into<i64> + Copy> AmountGoal<T> {
    pub fn new(target_amount: T, is_good: bool) -> Self {
        Self {
            target_amount,
            is_good,
        }
    }

    pub fn target_amount(&self) -> T {
        self.target_amount
    }

    pub fn is_at_least(&self) -> bool {
        self.is_good
    }
}

impl<T> Goal for AmountGoal<T>
where
    T: Default + Into<i64> + Copy,
{
    type LogValue = T;

    fn get_percentage_achieved(&self, value: Option<&T>) -> Percentage {
        let default_value = T::default();
        let value = value.unwrap_or(&default_value);

        let target_u64: i64 = self.target_amount.into();
        let value_u64: i64 = (*value).into();

        let perc = if self.is_good {
            if target_u64 == 0 {
                100
            } else {
                (value_u64 * 100) / target_u64
            }
        } else {
            // TODO: Dividing doesn't make much sense in this case
            if value_u64 == 0 {
                100
            } else {
                (target_u64 * 100) / value_u64
            }
        };

        Percentage::new(perc as u8)
    }
}
