// CHANGED switch arms in todayProgress and todayFigure:
// case 'TimedAmounts': {
//   TODO: if day.value===null return "No data" (muted in todayProgress).
//   Otherwise use formatValueWithUnit(measure,day.value); amount cap says "180 mg used";
//   at-least says "180 of 400 mg"; figure says "180 / 400 mg".
//   Reuse existing status tone and scalar target. [] appears as 0 mg, not missing.
// }
// This must not fall through the existing number-or-0 amount formatting for missing data.
