use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::update_policy::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
use serde::{Deserialize, Serialize};
use transfer_objects::raw_goal::RawGoal;

pub type HabitStatistic = StatisticTemplate<bool, bool, FixedValueUpdatePolicy, HabitGoal>;
pub trait HabitLogsRepository = LoggingDataRepositoryTrait<StoredType = bool>;
pub trait HabitGoalsRepository = GoalRepositoryTrait<StoredValue = bool, GoalType = HabitGoal>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitGoal {
    target: bool,
}

impl HabitGoal {
    pub fn new(target: bool) -> HabitGoal {
        HabitGoal { target }
    }

    // fn is_good_habit(&self) -> bool {
    //     self.target
    // }
}

impl Goal<bool> for HabitGoal {
    fn new_with_target(target: bool) -> Self {
        Self { target }
    }

    fn set_extra_data_1(self, _extra_data_1: &str) -> anyhow::Result<Self> {
        Ok(self)
    }

    fn get_extra_data_1(&self) -> Option<String> {
        None
    }

    fn get_value(&self) -> &bool {
        &self.target
    }

    fn get_percentage_achieved(&self, value: Option<&bool>) -> f64 {
        let value = value.unwrap_or(&false); // Interpret 'nothing logged' as 'false'

        if *value == self.target {
            1.0
        } else {
            0.0
        }
    }

    fn into_raw_goal(self) -> RawGoal {
        RawGoal {
            target: self.target.to_string(),
            extra_data_1: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod goal_tests {
        use super::*;

        #[test]
        fn test_goal_1() {
            let goal = HabitGoal::new(true);
            assert_eq!(goal.get_percentage_achieved(Some(&true)), 1.0);
        }

        #[test]
        fn test_goal_2() {
            let goal = HabitGoal::new(true);
            assert_eq!(goal.get_percentage_achieved(Some(&false)), 0.0);
        }

        #[test]
        fn test_goal_3() {
            let goal = HabitGoal::new(false);
            assert_eq!(goal.get_percentage_achieved(Some(&true)), 0.0);
        }

        #[test]
        fn test_goal_4() {
            let goal = HabitGoal::new(false);
            assert_eq!(goal.get_percentage_achieved(Some(&false)), 1.0);
        }

        #[test]
        fn test_goal_5() {
            let goal = HabitGoal::new(false);
            assert_eq!(goal.get_percentage_achieved(None), 1.0);
        }

        #[test]
        fn test_goal_6() {
            let goal = HabitGoal::new(true);
            assert_eq!(goal.get_percentage_achieved(None), 0.0);
        }
    }
}
