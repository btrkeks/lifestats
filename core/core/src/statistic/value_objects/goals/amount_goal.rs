use crate::shared::percentage::Percentage;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountGoal<T: Default + Clone + Into<i64> + Copy> {
    target_amount: T,
    is_at_least: bool,
}

impl<T: Default + Into<i64> + Copy> AmountGoal<T> {
    pub fn new(target_amount: T, is_good: bool) -> Self {
        Self {
            target_amount,
            is_at_least: is_good,
        }
    }

    pub fn target_amount(&self) -> T {
        self.target_amount
    }

    pub fn is_at_least(&self) -> bool {
        self.is_at_least
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

        let perc = if self.is_at_least {
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

        // Percentage holds 0-255; `as u8` alone would wrap 514% to 2%
        Percentage::new(perc.clamp(0, u8::MAX.into()) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn percent(goal: AmountGoal<i64>, value: i64) -> u8 {
        goal.get_percentage_achieved(Some(&value)).as_num()
    }

    #[test]
    fn test_at_least_goal_reports_the_share_of_the_target() {
        let goal = AmountGoal::new(400, true);
        assert_eq!(percent(goal.clone(), 327), 81);
        assert_eq!(percent(goal.clone(), 461), 115);
        assert_eq!(goal.get_percentage_achieved(None).as_num(), 0);
    }

    #[test]
    fn test_far_above_an_at_least_goal_saturates_instead_of_wrapping() {
        assert_eq!(percent(AmountGoal::new(200, true), 1500), 255);
    }

    #[test]
    fn test_well_under_an_at_most_goal_counts_as_met() {
        // 12 minutes of browsing against at most 1 hour: 514%, not 2%
        let goal = AmountGoal::new(3600, false);
        assert_eq!(percent(goal.clone(), 700), 255);
        assert_eq!(percent(goal.clone(), 3600), 100);
        assert_eq!(percent(goal, 7200), 50);
    }

    #[test]
    fn test_nothing_logged_meets_an_at_most_goal() {
        assert_eq!(
            AmountGoal::new(3600_i64, false)
                .get_percentage_achieved(None)
                .as_num(),
            100
        );
    }

    #[test]
    fn test_negative_values_clamp_to_zero() {
        assert_eq!(percent(AmountGoal::new(400, true), -50), 0);
    }
}
