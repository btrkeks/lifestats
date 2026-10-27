import type { GoalData } from '@/types';
import type { Measure } from '@/lib/measure';
import type { Day, DayStatus } from './day';

const MINUTES_PER_DAY = 24 * 60;

/** "22:37:00" → 1357 */
export function clockMinutes(time: string): number {
  const [hours = '0', minutes = '0'] = time.split(':');
  return Number(hours) * 60 + Number(minutes);
}

/**
 * Minutes of `time` placed within 12 hours of `anchor`, so a bedtime of 00:30
 * reads as 24:30 next to a 22:05 goal instead of half a day earlier.
 */
export function anchoredMinutes(time: string, anchor: number): number {
  let minutes = clockMinutes(time);
  while (minutes < anchor - MINUTES_PER_DAY / 2) minutes += MINUTES_PER_DAY;
  while (minutes >= anchor + MINUTES_PER_DAY / 2) minutes -= MINUTES_PER_DAY;
  return minutes;
}

/** 1357 → "22:37", 1470 → "24:30" (kept past midnight on purpose). */
export function formatMinutes(minutes: number): string {
  const rounded = Math.round(minutes);
  const sign = rounded < 0 ? '-' : '';
  const abs = Math.abs(rounded);
  return `${sign}${String(Math.floor(abs / 60)).padStart(2, '0')}:${String(abs % 60).padStart(2, '0')}`;
}

/** Met days out of days that had a goal, up to and including today. */
export function metCount(days: Day[]): { met: number; withGoal: number } {
  const judged = days.filter(
    (day) => day.status === 'met' || day.status === 'missed' || day.status === 'open'
  );
  return { met: judged.filter((day) => day.status === 'met').length, withGoal: judged.length };
}

/** Where the chart's goal line sits, in the chart's units; null when there's none. */
export function goalLevel(goal: GoalData | null): number | null {
  if (!goal) return null;
  switch (goal.type) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
      return goal.data.target;
    case 'PointInTime':
      return clockMinutes(goal.data.deadline);
    case 'Habit':
    case 'TimeSpans':
      return 100;
  }
}

export interface TrendPoint {
  date: string;
  /** Value in chart units (amount, anchored minutes or percent); null when nothing to plot */
  y: number | null;
  status: DayStatus;
}

/**
 * Chart points: amounts for counts and durations, anchored clock minutes for
 * times, the achieved percentage for habits and time spans.
 */
export function trendPoints(days: Day[], measure: Measure, level: number | null): TrendPoint[] {
  return days.map((day) => {
    let y: number | null = null;
    switch (measure.kind) {
      case 'count':
      case 'duration':
        y = typeof day.value === 'number' ? day.value : null;
        break;
      case 'clock':
        y = typeof day.value === 'string' ? anchoredMinutes(day.value, level ?? 12 * 60) : null;
        break;
      case 'habit':
      case 'spans':
        y = day.percentage_achieved;
        break;
    }
    return { date: day.date, y, status: day.status };
  });
}

export interface TrendReadout {
  label: string;
  value: string;
  /** Change against the window before; `better` decides the colour */
  delta: { text: string; direction: 'up' | 'down'; better: boolean } | null;
}

function average(values: number[]): number | null {
  return values.length === 0 ? null : values.reduce((sum, v) => sum + v, 0) / values.length;
}

function metRate(days: Day[]): number | null {
  const judged = days.filter((day) => day.status === 'met' || day.status === 'missed');
  if (judged.length === 0) return null;
  return Math.round((judged.filter((day) => day.status === 'met').length / judged.length) * 100);
}

/**
 * "Last 14 days vs the 14 before", from days ending today (today itself is
 * left out: it isn't finished). Times read as an average, everything else as
 * the share of met days.
 */
export function trendReadout(
  days: Day[],
  measure: Measure,
  goal: GoalData | null,
  window = 14
): TrendReadout {
  const finished = days.slice(0, -1);
  const recent = finished.slice(-window);
  const before = finished.slice(-2 * window, -window);
  const label = `Last ${window} days`;

  if (measure.kind === 'clock') {
    const anchor = goalLevel(goal) ?? 12 * 60;
    const minutesOf = (list: Day[]) =>
      list.flatMap((day) =>
        typeof day.value === 'string' ? [anchoredMinutes(day.value, anchor)] : []
      );
    const now = average(minutesOf(recent));
    const then = average(minutesOf(before));
    if (now === null) return { label, value: 'No times', delta: null };
    const diff = then === null ? 0 : Math.round(now - then);
    return {
      label,
      value: `${formatMinutes(now)} avg`,
      delta:
        diff === 0
          ? null
          : {
              text: `${Math.abs(diff)} min ${diff < 0 ? 'earlier' : 'later'} than the ${window} before`,
              direction: diff < 0 ? 'down' : 'up',
              better: diff < 0,
            },
    };
  }

  const now = metRate(recent);
  const then = metRate(before);
  if (now === null) return { label, value: 'No goal days', delta: null };
  return {
    label,
    value: `${now}% met`,
    delta:
      then === null || then === now
        ? null
        : {
            text: `from ${then}% before`,
            direction: now > then ? 'up' : 'down',
            better: now > then,
          },
  };
}
