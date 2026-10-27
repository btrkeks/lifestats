// CHANGED ARM in any_goal_to_json(goal):
// AnyGoal::TimedAmounts(goal) => serde_json::to_value(goal).
// Keep the payload untagged; do not serialize AnyGoal itself. Resolver unchanged.
