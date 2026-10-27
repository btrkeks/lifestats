// CHANGED ARM in evaluate_goal_achievement:
// AnyStatistic::TimedAmounts(_) => {
//   TODO: optional JSON -> optional validated TimedAmountList, transpose errors;
//   goal JSON -> TimedAmountsGoal; goal.get_percentage_achieved(list.as_ref()).
// }
// CHANGED handle boundary: if TimedAmounts and plot type TimeFrames/CircularProgress,
// return InvalidInput BEFORE reading/iterating date range, even if no points exist.
// Update both HTTP plot error mappings to return 400 for InvalidInput.
