use crate::shared::timed_amount_list::TimedAmountList;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::timed_amounts_goal::TimedAmountsGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::timed_amounts::TimedAmountsUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use chrono::NaiveDate;

/// A statistic whose day is a list of (time, amount) entries.
///
/// Examples:
/// - Caffeine (mg): [08:12 120, 13:40 60]
/// - Nicotine gum (mg)
///
/// Source-owned for now: only its integration writes it, by replacing whole days
/// (see `crate::opennutritracker::OpenNutriTrackerDayStore`). The integer value is
/// the daily total, so plots, dashboards and amount goals work unchanged.
#[derive(Clone)]
pub struct TimedAmountsStatistic {
    // Metadata
    metadata: StatisticMetadata,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl TimedAmountsStatistic {
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
}

impl Statistic for TimedAmountsStatistic {
    type LogValueType = TimedAmountList;
    type GoalType = TimedAmountsGoal;

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

impl LogableStatistic for TimedAmountsStatistic {
    type UpdatePolicyType = TimedAmountsUpdatePolicy;

    fn calculate_new_log_value(
        &self,
        current_value: Option<Self::LogValueType>,
        new_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> (Self::LogValueType, bool) {
        update_policy.apply(current_value, new_value)
    }
}

impl IntegerStatistic for TimedAmountsStatistic {
    fn value_as_i64(&self, value: &Self::LogValueType) -> i64 {
        value.total()
    }
}
