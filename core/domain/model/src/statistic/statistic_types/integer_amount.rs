use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::update_policy::amount::AmountUpdatePolicy;
use crate::statistic::statistic::Statistic;
use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
use chrono::NaiveDate;
use transfer_objects::raw_goal::RawGoal;

pub type IntegerAmountStatistic =
    StatisticTemplate<i64, i64, AmountUpdatePolicy, IntegerAmountGoal>;
pub trait IntegerAmountLogsRepository = LoggingDataRepositoryTrait<StoredType = i64>;
pub trait IntegerAmountGoalsRepository =
    GoalRepositoryTrait<GoalType = IntegerAmountGoal, StoredValue = i64>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
    // fn set_target(mut self, target: i64) -> Self {
    //     self.target = target;
    //     self
    // }

    fn new_with_target(target: i64) -> Self {
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

    fn get_value(&self) -> &i64 {
        &self.target
    }

    fn get_percentage_achieved(&self, value: Option<&i64>) -> f64 {
        let value = value.unwrap_or(&0); // Interpret 'nothing logged' as 0

        let perc = if self.is_good {
            *value as f64 / self.target as f64
        } else {
            self.target as f64 / value.max(&1).to_owned() as f64
        };

        perc.max(0.0)
    }

    fn into_raw_goal(self) -> RawGoal {
        RawGoal {
            target: self.target.to_string(),
            extra_data_1: Some(self.is_good.to_string()),
        }
    }
}

impl IntegerStatistic for IntegerAmountStatistic {
    fn get_integer_data_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<Vec<DateValue<Option<i64>>>> {
        self._get_values_for_closed_range(start_date, end_date)
    }

    fn get_integer_goals_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<Vec<DateValue<Option<i64>>>> {
        Ok(self
            ._get_goals_for_closed_range(start_date, end_date)?
            .into_iter()
            .map(|date_value| {
                DateValue::new(date_value.date, date_value.value.map(|goal| goal.target))
            })
            .collect())
    }

    fn get_y_axis_conversion(&self) -> YAxisConversion {
        YAxisConversion::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // fn create_test_metadata() -> StatisticMetadata {
    //     StatisticMetadata {
    //         id: StatisticId(1),
    //         name: "test_stat".to_string(),
    //         category: "test".to_string(),
    //     }
    // }

    mod goal_tests {
        use super::*;

        #[test]
        fn test_goal_creation() {
            let goal = IntegerAmountGoal::new(100, true);
            let percentage = goal.get_percentage_achieved(Some(&50));
            assert_eq!(percentage, 0.5);
        }

        #[test]
        fn test_positive_goal_percentage() {
            let goal = IntegerAmountGoal::new(100, true);

            // Test various scenarios
            assert_eq!(goal.get_percentage_achieved(Some(&0)), 0.0);
            assert_eq!(goal.get_percentage_achieved(Some(&50)), 0.5);
            assert_eq!(goal.get_percentage_achieved(Some(&100)), 1.0);
            assert_eq!(goal.get_percentage_achieved(Some(&200)), 2.0);
        }

        #[test]
        fn test_negative_goal_percentage() {
            let goal = IntegerAmountGoal::new(100, false);

            // For negative goals (is_good = false), higher values should result in lower percentages
            assert_eq!(goal.get_percentage_achieved(Some(&200)), 0.5); // 100/200
            assert_eq!(goal.get_percentage_achieved(Some(&100)), 1.0); // 100/100
            assert_eq!(goal.get_percentage_achieved(Some(&50)), 2.0); // 100/50
        }

        #[test]
        fn test_zero_and_negative_values() {
            let goal = IntegerAmountGoal::new(100, true);

            // Test with zero
            assert_eq!(goal.get_percentage_achieved(Some(&0)), 0.0);

            // Test with negative values
            assert_eq!(goal.get_percentage_achieved(Some(&-50)), 0.0);
        }

        #[test]
        fn test_negative_goal_zero_handling() {
            let goal = IntegerAmountGoal::new(100, false);

            // When is_good is false and value is 0, it should use max(value, 1) to avoid division by zero
            assert_eq!(goal.get_percentage_achieved(Some(&0)), 100.0); // 100/1
        }

        #[test]
        fn test_edge_cases() {
            // Test with very large numbers
            let large_goal = IntegerAmountGoal::new(i64::MAX, true);
            assert!(large_goal.get_percentage_achieved(Some(&(i64::MAX / 2))) < 1.0);

            // Test with very small numbers
            let small_goal = IntegerAmountGoal::new(1, true);
            assert_eq!(small_goal.get_percentage_achieved(Some(&0)), 0.0);

            // Test with negative goals
            let negative_goal = IntegerAmountGoal::new(-100, true);
            assert_eq!(negative_goal.get_percentage_achieved(Some(&-50)), 0.5);
        }
    }
}
