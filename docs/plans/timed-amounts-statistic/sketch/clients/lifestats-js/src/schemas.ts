// EXCERPT — clients/lifestats-js/src/schemas.ts
// SILENT and severe if missed: getAllStatistics() Zod-fails the WHOLE list once
// one TimedAmounts statistic exists (client.ts:45, :89).
export const statisticTypeSchema = z.enum([
  'IntegerAmount',
  'Habit',
  'PointInTime',
  'TimeSpans',
  'TimedAmounts', // + NEW
  'Summing',
  'TimeDifference',
]);
