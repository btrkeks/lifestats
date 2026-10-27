// In formatGoalTarget, formatGoal, describeGoal, goalsEqual switches:
// add case 'TimedAmounts' to the existing IntegerAmount/Summing/TimeDifference group.
// Uses amount target/direction; no clock goal or second target is inferred.
// For describeGoal's TimedAmounts arm, use unitSuffix(measure) in the sentence,
// e.g. "At most 400 mg a day." Keep other types' existing formatting.
