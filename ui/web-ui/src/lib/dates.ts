import { addDays, format, parseISO } from 'date-fns';

export const ISO_DATE = 'yyyy-MM-dd';

export function toIsoDate(date: Date): string {
  return format(date, ISO_DATE);
}

export function todayIso(): string {
  return toIsoDate(new Date());
}

/** Shifts an ISO date by a number of days (negative for the past). */
export function shiftIsoDate(iso: string, days: number): string {
  return toIsoDate(addDays(parseISO(iso), days));
}

/** "2026-10-25" → "Sun 25 Oct" */
export function formatDay(iso: string): string {
  return format(parseISO(iso), 'EEE d MMM');
}

/** "2026-10-27" → "Tue 27 Oct 2026" */
export function formatDayWithYear(iso: string): string {
  return format(parseISO(iso), 'EEE d MMM yyyy');
}

/** "2026-10-25" → "Sunday 25 October 2026", for screen readers */
export function formatDayLong(iso: string): string {
  return format(parseISO(iso), 'EEEE d MMMM yyyy');
}

/** "2026-10-12" → "Oct" */
export function formatShortMonth(iso: string): string {
  return format(parseISO(iso), 'MMM');
}

/** "2026-10-12" → "12 Oct" */
export function formatDayMonth(iso: string): string {
  return format(parseISO(iso), 'd MMM');
}

/**
 * A run of dates in a few words: "25 Oct", "24–26 Oct", "30 Sep–2 Oct".
 * `from` and `to` are inclusive and `from <= to`.
 */
export function formatDateSpan(from: string, to: string): string {
  if (from === to) return formatDayMonth(from);
  if (from.slice(0, 7) === to.slice(0, 7)) {
    return `${parseISO(from).getDate()}–${formatDayMonth(to)}`;
  }
  return `${formatDayMonth(from)}–${formatDayMonth(to)}`;
}

/** Groups ascending ISO dates into runs of consecutive days. */
export function consecutiveRuns(dates: string[]): { from: string; to: string }[] {
  const runs: { from: string; to: string }[] = [];
  for (const date of dates) {
    const last = runs[runs.length - 1];
    if (last && shiftIsoDate(last.to, 1) === date) {
      last.to = date;
    } else {
      runs.push({ from: date, to: date });
    }
  }
  return runs;
}
