// CHANGED EXCERPTS only; existing variants/arms remain.
pub enum AnyGoal {
    // ...
    TimedAmounts(TimedAmountsGoal),
}
pub enum AnyStatistic {
    // ...
    TimedAmounts(TimedAmountsStatistic),
}
// In dispatch_statistic!: AnyStatistic::TimedAmounts(s) => s.$method($($args)*),
// In value_as_i64_from_json's match:
// AnyStatistic::TimedAmounts(s) => deserialize TimedAmountList then s.value_as_i64(&value).
// In get_default_goal's match:
// AnyStatistic::TimedAmounts(s) => s.get_default_goal().cloned().map(AnyGoal::TimedAmounts).
// In evaluate_streak_for_day's match:
// AnyStatistic::TimedAmounts(_) => evaluate_streak_for_day_inner::<TimedAmountList,TimedAmountsGoal>(metadata,log_value,goal).
// No new from/into/as/setter helpers without a caller; direct enum construction and
// the existing Statistic trait suffice for this delivery.
