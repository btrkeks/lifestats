use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::habit_goal::HabitGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use chrono::NaiveDate;

/// A statistic that represents a habit
///
/// Examples:
/// - Whether you brushed your teeth that day
/// - Whether you went to the gym that day
/// - Whether you avoided touching your phone in the morning
#[derive(Clone)]
pub struct HabitStatistic {
    // Metadata
    metadata: StatisticMetadata,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for HabitStatistic {
    type LogValueType = bool;
    type GoalType = HabitGoal;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_default_goal(&self) -> Option<&Self::GoalType> {
        self.default_goal.as_ref()
    }

    fn _set_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate) {
        self.default_goal.replace(goal);
        self.default_goal_start = start_date;
    }

    fn get_default_goal_valid_since(&self) -> NaiveDate {
        self.default_goal_start
    }
}

impl LogableStatistic for HabitStatistic {
    type UpdatePolicyType = FixedValueUpdatePolicy;

    fn calculate_new_log_value(
        &self,
        current_value: Option<Self::LogValueType>,
        new_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> (Self::LogValueType, bool) {
        update_policy.apply(current_value, new_value)
    }
}

impl HabitStatistic {
    pub fn new(
        metadata: StatisticMetadata,
        default_goal: Option<<Self as Statistic>::GoalType>,
        default_goal_start: NaiveDate,
    ) -> Self {
        Self {
            metadata,
            default_goal,
            default_goal_start,
        }
    }

    pub fn calculate_new_log_value(
        &self,
        current_value: Option<bool>,
        new_value: bool,
        update_policy: FixedValueUpdatePolicy,
    ) -> (bool, bool) {
        <Self as LogableStatistic>::calculate_new_log_value(
            self,
            current_value,
            new_value,
            update_policy,
        )
    }
}

impl IntegerStatistic for HabitStatistic {
    fn value_as_i64(&self, value: &Self::LogValueType) -> i64 {
        if *value {
            1
        } else {
            0
        }
    }
}
