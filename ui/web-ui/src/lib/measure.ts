import type { DayValue, StatisticMetadata } from '@/types';
import { formatClock } from './goalFormat';

/**
 * How a statistic's daily values read. The backend stores no units, so counts
 * get theirs from the statistic name ("[Clef] Piano practice (minutes)" → min)
 * and amounts named like a duration are taken to be seconds.
 */
export type Measure =
  | { kind: 'count'; unit: string | null }
  | { kind: 'duration' } // seconds
  | { kind: 'clock' } // "HH:MM:SS"
  | { kind: 'habit' }
  | { kind: 'spans' }; // total seconds inside the logged windows

const COUNT_UNITS: [RegExp, string][] = [
  [/\(mg\)/i, 'mg'],
  [/\((minutes|mins?)\)/i, 'min'],
  [/\breps\b/i, 'reps'],
  [/\bcards\b/i, 'cards'],
  [/\bcharacters\b/i, 'chars'],
  [/\bsteps\b/i, 'steps'],
  [/\bsets\b/i, 'sets'],
];

const SECONDS_NAME = /duration|screen time|時間/i;

export function measureOf(statistic: Pick<StatisticMetadata, 'name' | 'statistic_type'>): Measure {
  switch (statistic.statistic_type) {
    case 'Habit':
      return { kind: 'habit' };
    case 'PointInTime':
      return { kind: 'clock' };
    case 'TimeSpans':
      return { kind: 'spans' };
    case 'TimeDifference':
      return { kind: 'duration' };
    case 'TimedAmounts':
    case 'IntegerAmount':
    case 'Summing': {
      const unit = COUNT_UNITS.find(([pattern]) => pattern.test(statistic.name))?.[1] ?? null;
      if (
        !unit &&
        statistic.statistic_type === 'IntegerAmount' &&
        SECONDS_NAME.test(statistic.name)
      ) {
        return { kind: 'duration' };
      }
      return { kind: 'count', unit };
    }
  }
}

/** 25200 → "7 h"; 5940 → "1 h 39 min"; 2700 → "45 min"; 20 → "20 s". */
export function formatSeconds(totalSeconds: number): string {
  const seconds = Math.max(0, Math.round(totalSeconds));
  if (seconds < 60) return `${seconds} s`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  if (hours === 0) return `${minutes} min`;
  return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
}

/** 4000 → "4,000" */
export function formatCount(amount: number): string {
  return amount.toLocaleString('en-US');
}

/** A day's value without its unit, e.g. "327", "1 h 39 min", "22:37", "Done". */
export function formatValue(measure: Measure, value: DayValue): string {
  switch (measure.kind) {
    case 'count':
      return typeof value === 'number' ? formatCount(value) : String(value);
    case 'duration':
    case 'spans':
      return typeof value === 'number' ? formatSeconds(value) : String(value);
    case 'clock':
      return typeof value === 'string' ? formatClock(value) : String(value);
    case 'habit':
      return value === true ? 'Done' : 'Not done';
  }
}

/** A count with its unit, e.g. "4,000 chars"; other measures read as `formatValue`. */
export function formatValueWithUnit(measure: Measure, value: DayValue): string {
  const text = formatValue(measure, value);
  return measure.kind === 'count' && measure.unit ? `${text} ${measure.unit}` : text;
}
