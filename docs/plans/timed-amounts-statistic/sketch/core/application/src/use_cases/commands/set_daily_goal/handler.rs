// CHANGED ARM in shared parse_goal_payload_to_json before mismatch wildcard:
// (AnyStatistic::TimedAmounts(_),GoalPayload::TimedAmounts(goal)) =>
//     serde_json::to_value(goal).map_err(|e| ApplicationError::Infrastructure(e.into()))?.
// Existing daily/default/range handlers reuse this helper, preserving timeline history.
