import { DayOfWeek, Duration, LocalDate } from '@js-joda/core';
import { match } from 'ts-pattern';

/**
 * `toLocaleString` constructs a whole ICU formatter per call, which is far too expensive to do from a
 * render -- the month grid formats a date for every one of its 42 cells. Formatters are immutable, so one
 * per locale and option set can be reused forever.
 */
const formatters = new Map<string, Intl.DateTimeFormat>();

function formatterFor(locale: string | undefined, opts: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const key = `${locale}|${JSON.stringify(opts)}`;
  const existing = formatters.get(key);
  if (existing) {
    return existing;
  }

  const formatter = new Intl.DateTimeFormat(locale, opts);
  formatters.set(key, formatter);
  return formatter;
}

/** For code outside React. Components use `useFormatDate`, which reads the locale for them. */
export function formatDate(locale: string | undefined, date: LocalDate, opts: Intl.DateTimeFormatOptions): string {
  return formatterFor(locale, opts).format(new Date(date.year(), date.month().ordinal(), date.dayOfMonth()));
}

export function getDateOnDay(dayOfWeek: DayOfWeek) {
  // Super gross and hacky, but then we get i18n formatting for free
  const constantSundayDay = LocalDate.of(2025, 5, 4);
  const dateWithSpecifiedDayOfWeek = constantSundayDay.plusDays(dayOfWeek.value());
  return dateWithSpecifiedDayOfWeek;
}

export function formatDuration(duration: Duration, truncateTo: 'mins' | 'none' | 'hours-mins' = 'none') {
  const str = match({ duration, truncateTo })
    .with({ truncateTo: 'mins' }, () => `${Math.floor(duration.toMinutes())} mins`)
    .when(
      ({ duration, truncateTo }) => duration.toHours() >= 1 && truncateTo === 'hours-mins',
      () => `${duration.toHours()} hrs ${Math.floor(duration.toMinutes() - 60 * duration.toHours())} mins`,
    )
    .when(
      ({ duration }) => duration.toHours() >= 1,
      () => `${duration.toHours()} hrs`,
    )
    .when(
      ({ duration }) => duration.toMinutes() >= 1,
      () => `${duration.toMinutes()} mins`,
    )
    .otherwise(() => `${duration.toMillis() / 1000} secs`);

  return str;
}

/**
 * Takes a string formatted by the csharp format string: @"d\.hh\:mm\:ss\:FFF"
 * and returns a js joda duration
 * Days and milliseconds are optional (e.g., "02:30:45" or "1.02:30:45:123")
 */
export function parseDuration(val: string): Duration {
  // Regex with optional days and milliseconds
  const regex = /^(?:(\d+)\.)?(\d{2}):(\d{2}):(\d{2})(?::(\d{3}))?$/;
  const matchResult = val.match(regex);

  if (!matchResult) {
    throw new Error(`Invalid duration format: ${val}`);
  }

  const [, daysStr, hoursStr, minutesStr, secondsStr, millisStr] = matchResult;

  const days = daysStr ? Number(daysStr) : 0;
  const hours = Number(hoursStr);
  const minutes = Number(minutesStr);
  const seconds = Number(secondsStr);
  const millis = millisStr ? Number(millisStr) : 0;

  return Duration.ofDays(days).plusHours(hours).plusMinutes(minutes).plusSeconds(seconds).plusMillis(millis);
}
