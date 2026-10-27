// New bounded schema, using the same request shape as amount goals:
// const timedAmountsGoalSchema=z.object({
//   target:z.coerce.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
//   is_at_least:booleanish,
// });
// goalFormSchemaFor switch: case 'TimedAmounts': return timedAmountsGoalSchema;
// goalFormDefaultsFor: case 'TimedAmounts': return {target:0,is_at_least:false};
// Zero at-most is only the unsaved initial form, never a provisioned goal.
// formGoal: case 'TimedAmounts': safeParse with this bounded schema, then
//   {type:'TimedAmounts',data:parsed.data} or null.
// parseGoalData: case 'TimedAmounts': validate stored target_amount as bounded
//   nonnegative safe integer, transform to {target,is_at_least}, return typed variant.
// Do not reuse an unbounded integer schema here or cast external rawData.
