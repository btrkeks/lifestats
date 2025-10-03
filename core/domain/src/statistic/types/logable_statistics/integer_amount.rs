use crate::statistic::error::{Result, StatisticError};
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::amount::AmountUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use chrono::NaiveDate;

/// A statistic that represents an arbitrary integer amount
///
/// Examples:
/// - Amount of pages read
/// - Number of sit-ups done
pub struct IntegerAmountStatistic {
    // Metadata
    metadata: StatisticMetadata,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for IntegerAmountStatistic {
    type LogValueType = i64;
    type GoalType = AmountGoal<i64>;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_log_value(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>> {
        Err(StatisticError::InvalidOperation(format!(
            "Statistic {} cannot fetch log values without an external repository (date: {date})",
            self.id()
        )))
    }

    fn get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        Err(StatisticError::InvalidOperation(format!(
            "Statistic {} cannot fetch goals without an external repository (date: {date})",
            self.id()
        )))
    }

    fn set_goal(&self, date: NaiveDate, goal: Self::GoalType) -> Result<()> {
        Err(StatisticError::InvalidOperation(
            format!(
                "Statistic {} cannot persist goals without an external repository (date: {date}, goal: {goal:?})",
                self.id()
            ),
        ))
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

impl LogableStatistic for IntegerAmountStatistic {
    type UpdatePolicyType = AmountUpdatePolicy;

    fn calculate_new_log_value(
        &self,
        current_value: Option<Self::LogValueType>,
        new_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> (Self::LogValueType, bool) {
        update_policy.apply(current_value, new_value)
    }
}

impl IntegerAmountStatistic {
    pub fn calculate_new_log_value(
        &self,
        current_value: Option<i64>,
        new_value: i64,
        update_policy: AmountUpdatePolicy,
    ) -> (i64, bool) {
        <Self as LogableStatistic>::calculate_new_log_value(
            self,
            current_value,
            new_value,
            update_policy,
        )
    }
}
