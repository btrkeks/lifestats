// Add [/\(mg\)/i,'mg'] to existing COUNT_UNITS name-unit recognition.
// CHANGED switch arm in measureOf:
// case 'TimedAmounts': return {kind:'count',unit:COUNT_UNITS.find(([p])=>p.test(statistic.name))?.[1]??null};
// Never treat it as a duration based on name; one daily unit is the statistic's unit.
// Backend Number scalar and UI count keep current charts/annual totals working.
