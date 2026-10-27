use serde::{Deserialize, Deserializer, Serialize};
use crate::{shared::{percentage::Percentage, timed_amount_list::TimedAmountList},
    statistic::value_objects::goals::{amount_goal::AmountGoal, goal_trait::Goal}};

/// Stored as {target_amount,is_at_least}; native goal consumes the full list.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct TimedAmountsGoal(AmountGoal<i64>);
impl TimedAmountsGoal {
    /// 0 <= target <= MAX_DAILY_AMOUNT. Prevents AmountGoal's *100 overflowing.
    pub fn new(target: i64, is_at_least: bool) -> Result<Self, TimedAmountsGoalError> { todo!() }
    pub fn target_amount(&self) -> i64 { todo!() }
    pub fn is_at_least(&self) -> bool { todo!() }
}
impl<'de> Deserialize<'de> for TimedAmountsGoal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // TODO: private strict {target_amount:i64,is_at_least:bool}; call new().
        todo!()
    }
}
impl Goal for TimedAmountsGoal {
    type LogValue = TimedAmountList;
    fn get_percentage_achieved(&self, value: Option<&TimedAmountList>) -> Percentage {
        // TODO: None => Percentage::new(0). Some(list) => let total=list.total();
        // self.0.get_percentage_achieved(Some(&total)). [] is confirmed zero.
        todo!()
    }
}
#[derive(Debug, thiserror::Error)]
#[error("Goal target must be within the supported nonnegative range")]
pub struct TimedAmountsGoalError;
// No optional deadline field. Cutoff/composite goals and their UI are deferred.

#[cfg(test)]
mod tests {
    #[test]
    fn missing_is_not_confirmed_zero() { todo!() }
    #[test]
    fn scoring_matches_amount_goal_at_boundaries() { todo!() }
}
