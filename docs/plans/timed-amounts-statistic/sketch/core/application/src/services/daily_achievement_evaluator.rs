// CHANGED ARM in parse_day_value(stat,value):
// AnyStatistic::TimedAmounts(_) => {
//   let list: TimedAmountList=serde_json::from_value(value)?;
//   DayValue::Amount(list.total())
// }.
// CHANGED ARM in eval_range:
// AnyStatistic::TimedAmounts(_) => {
//   let goal = default_goal.and_then(|g| match g {
//       AnyGoal::TimedAmounts(g)=>Some(g), _=>None
//   });
//   self.eval_range_typed::<TimedAmountList,TimedAmountsGoal>(
//       stat,range,goal,default_goal_valid_since)
// }.
// Keep TimedAmounts OUT of the interval-merging special case in values_in_range.
