import { formatClock, formatGoalTarget } from '@/lib/goalFormat';
import { formatValue, formatValueWithUnit, type Measure } from '@/lib/measure';
import type { Day } from './day';

export type Tone = 'met' | 'open' | 'missed' | 'muted';

export interface TodayText {
  text: string;
  tone: Tone;
}

/** The goal list's line for today, e.g. "327 of 400", "Due 22:05", "20 s used". */
export function todayProgress(day: Day, measure: Measure): TodayText {
  if (day.status === 'suppressed') return { text: 'Suppressed', tone: 'muted' };
  const goal = day.goal;
  if (!goal || day.status === 'none' || day.status === 'future') {
    return { text: 'No goal today', tone: 'muted' };
  }
  const met = day.status === 'met';
  const value = day.value;

  switch (goal.type) {
    case 'TimedAmounts': {
      if (value === null) return { text: 'No data', tone: 'muted' };
      if (!goal.data.is_at_least) {
        return { text: `${formatValueWithUnit(measure, value)} used`, tone: met ? 'met' : 'missed' };
      }
      return {
        text: `${formatValue(measure, value)} of ${formatValueWithUnit(measure, goal.data.target)}`,
        tone: met ? 'met' : 'open',
      };
    }
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference': {
      const done = formatValue(measure, typeof value === 'number' ? value : 0);
      if (!goal.data.is_at_least) {
        return { text: `${done} used`, tone: met ? 'met' : 'missed' };
      }
      return { text: `${done} of ${formatGoalTarget(goal, measure)}`, tone: met ? 'met' : 'open' };
    }
    case 'PointInTime':
      if (typeof value === 'string') {
        return { text: `At ${formatClock(value)}`, tone: met ? 'met' : 'missed' };
      }
      return { text: `Due ${formatClock(goal.data.deadline)}`, tone: 'open' };
    case 'Habit':
      return met ? { text: 'Done', tone: 'met' } : { text: 'Not done yet', tone: 'open' };
    case 'TimeSpans': {
      const spent = formatValue(measure, typeof value === 'number' ? value : 0);
      return met ? { text: spent, tone: 'met' } : { text: `${spent} so far`, tone: 'open' };
    }
  }
}

/** The header's "Today" figure, e.g. "327 / 400", "due 22:05", "Suppressed". */
export function todayFigure(day: Day, measure: Measure): string {
  if (day.status === 'suppressed') return 'Suppressed';
  const goal = day.goal;
  if (!goal || day.status === 'none') return 'No goal';
  const value = day.value;
  switch (goal.type) {
    case 'TimedAmounts':
      if (value === null) return 'No data';
      return `${formatValue(measure, value)} / ${formatValueWithUnit(measure, goal.data.target)}`;
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
      return `${formatValue(measure, typeof value === 'number' ? value : 0)} / ${formatGoalTarget(goal, measure)}`;
    case 'PointInTime':
      return typeof value === 'string'
        ? formatClock(value)
        : `due ${formatClock(goal.data.deadline)}`;
    case 'Habit':
      return day.status === 'met' ? 'Done' : 'Not yet';
    case 'TimeSpans':
      return `${day.percentage_achieved ?? 0}%`;
  }
}

/** Counts for "Today 1 met · 5 open · 2 suppressed". */
export function todayCounts(days: Day[]): {
  met: number;
  open: number;
  missed: number;
  suppressed: number;
} {
  const counts = { met: 0, open: 0, missed: 0, suppressed: 0 };
  for (const day of days) {
    if (day.status === 'met') counts.met += 1;
    else if (day.status === 'open') counts.open += 1;
    else if (day.status === 'missed') counts.missed += 1;
    else if (day.status === 'suppressed') counts.suppressed += 1;
  }
  return counts;
}
