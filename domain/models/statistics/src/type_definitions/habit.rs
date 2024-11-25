use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::statistic_metadata::StatisticMetadata;
use crate::objects::update_policies::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::{Statistic, StatisticDefinition};

pub type HabitStatistic = StatisticDefinition<bool, bool, FixedValueUpdatePolicy, HabitGoal>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitGoal {
    target: bool,
}

impl HabitGoal {
    pub fn new(target: bool) -> HabitGoal {
        HabitGoal { target }
    }
}

impl Goal<bool> for HabitGoal {
    fn get_percentage_achieved(&self, value: &bool) -> f64 {
        if *value == self.target { 1.0 } else { 0.0 }
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

    fn create_test_statistic(goal: Option<HabitGoal>) -> HabitStatistic {
        HabitStatistic::new(create_test_metadata(), goal)
    }

    #[test]
    fn test_statistic_metadata() {
        let stat = create_test_statistic(None);
        assert_eq!(stat.get_id(), StatisticId(1));
        assert_eq!(stat.get_name(), "test_stat");
        assert_eq!(stat.get_category(), "test");
    }

    #[test]
    fn test_goal_getter() {
        let stat_no_goal = create_test_statistic(None);
        assert!(stat_no_goal.get_goal().is_none());

        let goal = HabitGoal::new(true);
        let stat_with_goal = create_test_statistic(Some(goal));
        assert!(stat_with_goal.get_goal().is_some());
    }

    mod goal_tests {
        use super::*;

        #[test]
        fn test_goal_1() {
            let goal = HabitGoal::new(true);
            assert_eq!(goal.get_percentage_achieved(&true), 1.0);
        }

        #[test]
        fn test_goal_2() {
            let goal = HabitGoal::new(true);
            assert_eq!(goal.get_percentage_achieved(&false), 0.0);
        }

        #[test]
        fn test_goal_3() {
            let goal = HabitGoal::new(false);
            assert_eq!(goal.get_percentage_achieved(&true), 0.0);
        }

        #[test]
        fn test_goal_4() {
            let goal = HabitGoal::new(false);
            assert_eq!(goal.get_percentage_achieved(&false), 1.0);
        }
    }
}