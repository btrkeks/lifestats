import type { DefaultGoal, GoalData } from '@/types';
import { consecutiveRuns, formatDateSpan, formatDay } from '@/lib/dates';
import { formatGoal, goalsEqual } from '@/lib/goalFormat';
import type { Measure } from '@/lib/measure';
import type { Day } from './day';

/** The backend's start for default goals that were never dated. */
const UNDATED = '1970-01-01';

/**
 * What saving a default goal from `appliesFrom` does to one day:
 * - `before`: earlier than the start, keeps its goal
 * - `gains`: now judged against the new goal
 * - `same`: already had exactly this goal
 * - `override`: a suppression or custom goal that stays
 * - `future`: not judged yet (the new goal will apply)
 */
export type PreviewMark = 'before' | 'gains' | 'same' | 'override' | 'future';

export function previewMark(day: Day, appliesFrom: string, newGoal: GoalData | null): PreviewMark {
  if (day.date < appliesFrom) return 'before';
  if (day.status === 'future') return 'future';
  if (day.override !== 'none') return 'override';
  if (day.goal && newGoal && goalsEqual(day.goal, newGoal)) return 'same';
  return 'gains';
}

export type EffectKind = 'start' | 'gains' | 'suppressed' | 'custom' | 'earlier';

export interface Effect {
  kind: EffectKind;
  text: string;
}

function plural(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

function spans(dates: string[]): string {
  return consecutiveRuns(dates)
    .map((run) => formatDateSpan(run.from, run.to))
    .join(', ');
}

/**
 * The "When you save" summary. `days` should run from `appliesFrom` (or the
 * current start, if earlier) to today.
 */
export function describeEffects({
  days,
  appliesFrom,
  newGoal,
  current,
  measure,
  today,
}: {
  days: Day[];
  appliesFrom: string;
  newGoal: GoalData | null;
  current: DefaultGoal | null;
  measure: Measure;
  today: string;
}): Effect[] {
  const effects: Effect[] = [];

  if (current && current.validSince !== UNDATED && current.validSince !== appliesFrom) {
    effects.push({
      kind: 'start',
      text: `Starts ${formatDay(appliesFrom)} instead of ${formatDay(current.validSince)}`,
    });
  } else if (!current) {
    effects.push({ kind: 'start', text: `Starts ${formatDay(appliesFrom)}` });
  }

  if (appliesFrom > today) {
    effects.push({ kind: 'gains', text: 'No past days change; today keeps its goal' });
  } else {
    const marked = days.filter((day) => day.date >= appliesFrom && day.date <= today);
    const gaining = marked.filter((day) => previewMark(day, appliesFrom, newGoal) === 'gains');
    const gained = gaining.filter((day) => !day.goal).length;
    const switched = gaining.length - gained;
    if (gained > 0) {
      effects.push({
        kind: 'gains',
        text: `${plural(gained, 'more day gets', 'more days get')} this goal, no backfilling`,
      });
    }
    if (switched > 0) {
      effects.push({
        kind: 'gains',
        text: `${plural(switched, 'day switches', 'days switch')} to this goal`,
      });
    }
    if (gaining.length === 0) {
      effects.push({ kind: 'gains', text: 'No past day changes' });
    }

    const suppressed = marked.filter((day) => day.override === 'suppressed').map((day) => day.date);
    if (suppressed.length > 0) {
      effects.push({
        kind: 'suppressed',
        text: `${spans(suppressed)} ${suppressed.length === 1 ? 'stays' : 'stay'} suppressed`,
      });
    }
    const custom = marked.filter((day) => day.override === 'goal').map((day) => day.date);
    if (custom.length > 0) {
      effects.push({
        kind: 'custom',
        text: `${spans(custom)} ${custom.length === 1 ? 'keeps its' : 'keep their'} custom goal`,
      });
    }
  }

  if (current && appliesFrom > current.validSince) {
    effects.push({
      kind: 'earlier',
      text: `Days before ${formatDay(appliesFrom)} keep ${formatGoal(current.goal, measure)}`,
    });
  }

  return effects;
}
