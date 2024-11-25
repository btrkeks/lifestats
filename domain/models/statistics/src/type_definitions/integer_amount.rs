use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::statistic_metadata::StatisticMetadata;
use crate::objects::update_policies::amount::AmountUpdatePolicy;
use crate::statistic::{Statistic, StatisticDefinition};

pub type IntegerAmountStatistic = StatisticDefinition<i64, i64, AmountUpdatePolicy, IntegerAmountGoal>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct IntegerAmountGoal {
    target: i64,
    is_good: bool,
}

impl IntegerAmountGoal {
    pub fn new(target_amount: i64, is_good: bool) -> Self {
        Self {
            target: target_amount,
            is_good,
        }
    }
}

impl Goal<i64> for IntegerAmountGoal {
    fn get_percentage_achieved(&self, value: &i64) -> f64 {
        let perc = if self.is_good {
            *value as f64 / self.target as f64
        } else {
            self.target as f64 / value.max(&1).to_owned() as f64
        };
        perc.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::objects::statistic_metadata::StatisticId;
    use super::*;

    fn create_test_metadata() -> StatisticMetadata {
        StatisticMetadata {
            id: StatisticId(1),
            name: "test_stat".to_string(),
            category: "test".to_string(),
        }
    }

    fn create_test_statistic(goal: Option<IntegerAmountGoal>) -> IntegerAmountStatistic {
        IntegerAmountStatistic::new(create_test_metadata(), goal)
    }

    #[test]
    fn test_statistic_metadata() {
        let stat = create_test_statistic(None);
        assert_eq!(stat.get_name(), "test_stat");
        assert_eq!(stat.get_category(), "test");
    }

    #[test]
    fn test_goal_getter() {
        let stat_no_goal = create_test_statistic(None);
        assert!(stat_no_goal.get_goal().is_none());

        let goal = IntegerAmountGoal::new(100, true);
        let stat_with_goal = create_test_statistic(Some(goal));
        assert!(stat_with_goal.get_goal().is_some());
    }

    mod goal_tests {
        use super::*;

        #[test]
        fn test_goal_creation() {
            let goal = IntegerAmountGoal::new(100, true);
            let percentage = goal.get_percentage_achieved(&50);
            assert_eq!(percentage, 0.5);
        }

        #[test]
        fn test_positive_goal_percentage() {
            let goal = IntegerAmountGoal::new(100, true);

            // Test various scenarios
            assert_eq!(goal.get_percentage_achieved(&0), 0.0);
            assert_eq!(goal.get_percentage_achieved(&50), 0.5);
            assert_eq!(goal.get_percentage_achieved(&100), 1.0);
            assert_eq!(goal.get_percentage_achieved(&200), 2.0);
        }

        #[test]
        fn test_negative_goal_percentage() {
            let goal = IntegerAmountGoal::new(100, false);

            // For negative goals (is_good = false), higher values should result in lower percentages
            assert_eq!(goal.get_percentage_achieved(&200), 0.5); // 100/200
            assert_eq!(goal.get_percentage_achieved(&100), 1.0); // 100/100
            assert_eq!(goal.get_percentage_achieved(&50), 2.0);  // 100/50
        }

        #[test]
        fn test_zero_and_negative_values() {
            let goal = IntegerAmountGoal::new(100, true);

            // Test with zero
            assert_eq!(goal.get_percentage_achieved(&0), 0.0);

            // Test with negative values
            assert_eq!(goal.get_percentage_achieved(&-50), 0.0);
        }

        #[test]
        fn test_negative_goal_zero_handling() {
            let goal = IntegerAmountGoal::new(100, false);

            // When is_good is false and value is 0, it should use max(value, 1) to avoid division by zero
            assert_eq!(goal.get_percentage_achieved(&0), 100.0); // 100/1
        }

        #[test]
        fn test_edge_cases() {
            // Test with very large numbers
            let large_goal = IntegerAmountGoal::new(i64::MAX, true);
            assert!(large_goal.get_percentage_achieved(&(i64::MAX / 2)) < 1.0);

            // Test with very small numbers
            let small_goal = IntegerAmountGoal::new(1, true);
            assert_eq!(small_goal.get_percentage_achieved(&0), 0.0);

            // Test with negative goals
            let negative_goal = IntegerAmountGoal::new(-100, true);
            assert_eq!(negative_goal.get_percentage_achieved(&-50), 0.5);
        }
    }
}