// CHANGED union excerpts; all existing members remain.
export type StatisticType = /* existing members | */ 'TimedAmounts';
export type GoalData = /* existing members | */
  { type: 'TimedAmounts'; data: IntegerAmountGoalData };
// DayValue remains number|boolean|string. A TimedAmounts value is the daily sum,
// including 0 for a stored empty list; null still means missing.
