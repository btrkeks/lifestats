// CHANGED ARM in json_value_to_drawable_data:
// AnyStatistic::TimedAmounts(_) => {
//   TODO: reject TimeFrames/CircularProgress; deserialize TimedAmountList;
//   DrawableData::Scalar(ScalarValue::number(list.total())).
// }
// CHANGED ARM in json_value_to_drawable_goal:
// AnyStatistic::TimedAmounts(_) => {
//   TODO: same plot-type rejection; deserialize TimedAmountsGoal;
//   DrawableGoal::Scalar(ScalarValue::number(goal.target_amount())).
// }
// Keep scalar Number and full statistic name (mg). No new plot-domain variants.
