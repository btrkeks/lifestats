use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use chrono::{NaiveDate, NaiveDateTime};

/// A statistic that represents a point in time
///
/// Examples:
/// - Wake-up time
/// - Time went to bed
/// - Time workout finished
pub struct PointInTimeStatistic {
    // Metadata
    metadata: StatisticMetadata,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for PointInTimeStatistic {
    type LogValueType = NaiveDateTime;
    type GoalType = FinishedInTimeGoal;

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

impl LogableStatistic for PointInTimeStatistic {
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

impl PointInTimeStatistic {
    pub fn calculate_new_log_value(
        &self,
        current_value: Option<NaiveDateTime>,
        new_value: NaiveDateTime,
        update_policy: FixedValueUpdatePolicy,
    ) -> (NaiveDateTime, bool) {
        <Self as LogableStatistic>::calculate_new_log_value(
            self,
            current_value,
            new_value,
            update_policy,
        )
    }
}
