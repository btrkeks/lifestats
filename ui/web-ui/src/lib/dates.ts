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

/** "2026-09-01" → "Tue, Sep 1" */
export function formatDayLabel(iso: string): string {
  return format(parseISO(iso), 'EEE, MMM d');
}

/** "2026-09-01" → "T" */
export function weekdayInitial(iso: string): string {
  return format(parseISO(iso), 'EEEEE');
}
