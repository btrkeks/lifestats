import type { GoalData } from '@/types';
import { formatCount, formatSeconds, type Measure } from './measure';

/** "07:45:00" → "07:45"; seconds are kept only when non-zero. */
export function formatClock(time: string): string {
  const [hours, minutes, seconds] = time.split(':');
  if (hours === undefined || minutes === undefined) {
    return time;
  }
  return seconds && seconds !== '00' ? `${hours}:${minutes}:${seconds}` : `${hours}:${minutes}`;
}

function formatAmount(target: number, measure: Measure): string {
  return measure.kind === 'duration' ? formatSeconds(target) : formatCount(target);
}

function unitSuffix(measure: Measure): string {
  return measure.kind === 'count' && measure.unit ? ` ${measure.unit}` : '';
}

function formatWindows(goal: Extract<GoalData, { type: 'TimeSpans' }>): string {
  return goal.data.target
    .map((frame) => `${formatClock(frame.start)}–${formatClock(frame.end)}`)
    .join(', ');
}

/** The target alone, as in "327 of 400" or "due 22:05": "400", "7 h", "22:05". */
export function formatGoalTarget(goal: GoalData, measure: Measure): string {
  switch (goal.type) {
    case 'IntegerAmount':
    case 'TimedAmounts':
    case 'Summing':
    case 'TimeDifference':
      return formatAmount(goal.data.target, measure);
    case 'Habit':
      return goal.data.occurred ? 'Done' : 'Not done';
    case 'PointInTime':
      return formatClock(goal.data.deadline);
    case 'TimeSpans':
      return `${goal.data.efficiency}%`;
  }
}

/** One-line goal, e.g. "≥ 400 cards", "≤ 1 h", "by 22:05", "21:15–21:45 · 80%". */
export function formatGoal(goal: GoalData, measure: Measure): string {
  switch (goal.type) {
    case 'IntegerAmount':
    case 'TimedAmounts':
    case 'Summing':
    case 'TimeDifference':
      return `${goal.data.is_at_least ? '≥' : '≤'} ${formatAmount(goal.data.target, measure)}${unitSuffix(measure)}`;
    case 'Habit':
      return goal.data.occurred ? 'Done' : 'Not done';
    case 'PointInTime':
      return `by ${formatClock(goal.data.deadline)}`;
    case 'TimeSpans':
      return `${formatWindows(goal)} · ${goal.data.efficiency}%`;
  }
}

/** A sentence for the default goal card, e.g. "At least 400 a day." */
export function describeGoal(goal: GoalData, measure: Measure): string {
  switch (goal.type) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
      return `${goal.data.is_at_least ? 'At least' : 'At most'} ${formatAmount(goal.data.target, measure)} a day.`;
    case 'TimedAmounts':
      return `${goal.data.is_at_least ? 'At least' : 'At most'} ${formatAmount(goal.data.target, measure)}${unitSuffix(measure)} a day.`;
    case 'Habit':
      return goal.data.occurred ? 'Done every day.' : 'Not done on any day.';
    case 'PointInTime':
      return `Logged by ${formatClock(goal.data.deadline)} each day.`;
    case 'TimeSpans':
      return `${goal.data.efficiency}% of ${formatWindows(goal)} spent on it each day.`;
  }
}

function normalizeClock(time: string): string {
  return time.length === 5 ? `${time}:00` : time;
}

/** Whether two goals ask for the same thing ("22:05" equals "22:05:00"). */
export function goalsEqual(a: GoalData, b: GoalData): boolean {
  switch (a.type) {
    case 'IntegerAmount':
    case 'TimedAmounts':
    case 'Summing':
    case 'TimeDifference':
      return (
        b.type === a.type &&
        b.data.target === a.data.target &&
        b.data.is_at_least === a.data.is_at_least
      );
    case 'Habit':
      return b.type === 'Habit' && b.data.occurred === a.data.occurred;
    case 'PointInTime':
      return (
        b.type === 'PointInTime' &&
        normalizeClock(b.data.deadline) === normalizeClock(a.data.deadline)
      );
    case 'TimeSpans':
      return (
        b.type === 'TimeSpans' &&
        b.data.efficiency === a.data.efficiency &&
        b.data.target.length === a.data.target.length &&
        b.data.target.every(
          (frame, i) =>
            normalizeClock(frame.start) === normalizeClock(a.data.target[i].start) &&
            normalizeClock(frame.end) === normalizeClock(a.data.target[i].end)
        )
      );
  }
}
