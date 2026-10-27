import {
  addDays,
  addMonths,
  differenceInCalendarDays,
  endOfMonth,
  endOfWeek,
  format,
  parseISO,
  startOfMonth,
  startOfWeek,
} from 'date-fns';
import { toIsoDate } from '@/lib/dates';

const WEEK = { weekStartsOn: 1 } as const;

/** "2026-10-27" → "2026-10" */
export function monthOf(isoDate: string): string {
  return isoDate.slice(0, 7);
}

/** Moves a "YYYY-MM" month by `delta` months. */
export function shiftMonth(month: string, delta: number): string {
  return format(addMonths(parseISO(`${month}-01`), delta), 'yyyy-MM');
}

/** "2026-10" → "October 2026" */
export function formatMonth(month: string): string {
  return format(parseISO(`${month}-01`), 'MMMM yyyy');
}

/** The dates shown for a month: whole Monday-to-Sunday weeks, 5 or 6 rows. */
export function monthGrid(month: string): { start: string; end: string; dates: string[] } {
  const first = parseISO(`${month}-01`);
  const start = startOfWeek(startOfMonth(first), WEEK);
  const end = endOfWeek(endOfMonth(first), WEEK);
  const dates = Array.from({ length: differenceInCalendarDays(end, start) + 1 }, (_, i) =>
    toIsoDate(addDays(start, i))
  );
  return { start: toIsoDate(start), end: toIsoDate(end), dates };
}

/** Every date from `a` to `b` inclusive, ascending, whichever comes first. */
export function datesBetween(a: string, b: string): string[] {
  const [from, to] = a <= b ? [a, b] : [b, a];
  const start = parseISO(from);
  const count = differenceInCalendarDays(parseISO(to), start) + 1;
  return Array.from({ length: count }, (_, i) => toIsoDate(addDays(start, i)));
}

/** Monday of the week containing `isoDate`. */
export function startOfWeekIso(isoDate: string): string {
  return toIsoDate(startOfWeek(parseISO(isoDate), WEEK));
}
