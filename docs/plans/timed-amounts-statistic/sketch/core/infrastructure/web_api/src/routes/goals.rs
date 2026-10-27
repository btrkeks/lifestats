// CHANGED parse_hashmap_into_goal_payload arm:
// AnyStatistic::TimedAmounts(_) => {
//   TODO: parse target:i64 and optional is_at_least:bool (default true),
//   construct TimedAmountsGoal::new(target,is_at_least) or existing 400 BadRequest;
//   return GoalPayload::TimedAmounts(goal).
// }
// Existing default/daily/range DTOs and responses remain unchanged.
