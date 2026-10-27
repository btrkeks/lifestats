//! NEW FILE. A per-day list of (time, amount) entries, e.g. caffeine (mg), nicotine (mg).
//! v1 is source-owned: written only by whole-day replacement through its integration.
use chrono::NaiveDate;
use crate::shared::timed_amount_list::TimedAmountList;
use crate::statistic::{
    traits::{base_statistic::Statistic, integer_statistic::IntegerStatistic, logable_statistic::LogableStatistic},
    value_objects::{goals::timed_amounts_goal::TimedAmountsGoal,
        statistic_metadata::StatisticMetadata, update_policies::replace::Replace},
};

#[derive(Clone)]
pub struct TimedAmountsStatistic {
    metadata: StatisticMetadata,
    default_goal: Option<TimedAmountsGoal>,
    default_goal_start: NaiveDate,
}
impl TimedAmountsStatistic {
    pub fn new(metadata: StatisticMetadata, default_goal: Option<TimedAmountsGoal>, default_goal_start: NaiveDate) -> Self { todo!() }
}
impl Statistic for TimedAmountsStatistic {
    type LogValueType = TimedAmountList;
    type GoalType = TimedAmountsGoal;
    fn get_metadata(&self) -> &StatisticMetadata { todo!() }
    fn get_default_goal(&self) -> Option<&TimedAmountsGoal> { todo!() }
    fn _set_default_goal(&mut self, goal: TimedAmountsGoal, start_date: NaiveDate) { todo!() }
    fn get_default_goal_valid_since(&self) -> NaiveDate { todo!() }
}
impl LogableStatistic for TimedAmountsStatistic {
    type UpdatePolicyType = Replace;
    fn calculate_new_log_value(&self, current: Option<TimedAmountList>, incoming: TimedAmountList, policy: Replace) -> (TimedAmountList, bool) {
        // TODO: policy.apply(current,incoming). v1 has exactly one writer (the source
        // day store); there is no LogValuePayload variant, so no generic path reaches this.
        todo!()
    }
}
impl IntegerStatistic for TimedAmountsStatistic {
    fn value_as_i64(&self, value: &TimedAmountList) -> i64 {
        // TODO: value.total(); no parallel sum loop.
        todo!()
    }
}
