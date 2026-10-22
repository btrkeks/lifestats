import { LocalDate, OffsetDateTime, ZoneId } from '@js-joda/core';

/** UTC epoch milliseconds, the unit Android's calendar provider uses for event and instance times. */
export type Times = { startMs: number; endMs: number };

export const sameTimes = (a: Times, b: Times) => a.startMs === b.startMs && a.endMs === b.endMs;

/** Where a finished workout looks for its event, as the settings were when it was finished. */
export type CalendarTarget = { calendarId: number; accountName: string; title: string };

/**
 * One occurrence of an event, stable across LiftLog's own writes: moving an occurrence of a series turns it
 * into an exception row, but its master and original start stay the same.
 */
export type OccurrenceIdentity = {
  calendarId: number;
  accountName: string;
  /** The series master, or the event itself when it does not repeat. */
  eventId: number;
  /** The occurrence's original start (`ORIGINAL_INSTANCE_TIME`); undefined for an event that does not repeat. */
  instanceTime: number | undefined;
  /** The provider's sync id for `eventId`. A different one later means the row now holds another event. */
  syncId: string | undefined;
  /** The master's recurrence rule. A different one later means the series was changed or split. */
  rrule: string | undefined;
};

/** One event occurrence on a day in the target calendar. */
export type Occurrence = {
  identity: OccurrenceIdentity;
  title: string;
  times: Times;
  /**
   * Why LiftLog cannot move it. `unsupported`: all day, read-only, organised by someone else, or repeating
   * by date list only. `unsynced`: a series that has not reached the account yet, whose occurrences the
   * provider loses when one of them is moved.
   */
  problem: 'unsupported' | 'unsynced' | undefined;
};

/**
 * What the last look at a workout found. A settled outcome waits for the next edit of the workout or for
 * Try again; an unsettled one keeps the workout pending and is retried. Attention outcomes are listed in
 * settings.
 */
export const calendarOutcomes = {
  /** The event shows the workout. */
  adjusted: { settled: true, attention: false },
  /** Resumed, deleted or nothing completed: nothing to move, and an event it moved stays where it is. */
  idle: { settled: true, attention: false },
  'no-match': { settled: true, attention: true },
  ambiguous: { settled: true, attention: true },
  /** The day's only matching event belongs to another workout. */
  claimed: { settled: true, attention: true },
  unsupported: { settled: true, attention: true },
  /** The workout moved to another day; its event stays on the day it was matched on. */
  'moved-date': { settled: true, attention: true },
  /** The event's time was changed outside LiftLog since LiftLog last saw it. */
  changed: { settled: true, attention: true },
  /** The event was deleted, cancelled or replaced. */
  gone: { settled: true, attention: true },
  /** The provider holds two moved copies of the occurrence. */
  duplicate: { settled: true, attention: true },
  /** The calendar was removed, changed accounts or became read-only. */
  'calendar-missing': { settled: true, attention: true },
  unsynced: { settled: false, attention: true },
  error: { settled: false, attention: true },
} as const satisfies Record<string, { settled: boolean; attention: boolean }>;

export type CalendarOutcome = keyof typeof calendarOutcomes;

export function isCalendarOutcome(value: string): value is CalendarOutcome {
  return Object.hasOwn(calendarOutcomes, value);
}

/**
 * A workout's span as calendar times, widened to whole seconds because the provider stores a moved
 * occurrence's length in seconds. Undefined when it has no length, which leaves its event alone.
 */
export function workoutTimes(span: { start: OffsetDateTime; end: OffsetDateTime }): Times | undefined {
  if (!span.end.isAfter(span.start)) {
    return undefined;
  }
  return {
    startMs: Math.floor(span.start.toInstant().toEpochMilli() / 1000) * 1000,
    endMs: Math.ceil(span.end.toInstant().toEpochMilli() / 1000) * 1000,
  };
}

/** The phone's local day, from midnight to midnight: 23 or 25 hours across a DST change. */
export function dayWindow(date: LocalDate, zone: ZoneId = ZoneId.SYSTEM): Times {
  return {
    startMs: date.atStartOfDay(zone).toInstant().toEpochMilli(),
    endMs: date.plusDays(1).atStartOfDay(zone).toInstant().toEpochMilli(),
  };
}

export const identityKey = (identity: OccurrenceIdentity) =>
  `${identity.calendarId}/${identity.eventId}/${identity.instanceTime ?? '-'}`;

/**
 * The one occurrence titled `title` that starts on the day. Every exact match counts, including ones
 * LiftLog cannot move or another workout owns, so a second Gym event never hides behind a filter.
 */
export function matchOccurrence(
  occurrences: Occurrence[],
  title: string,
  day: Times,
): { kind: 'one'; occurrence: Occurrence } | { kind: 'no-match' | 'ambiguous' } {
  const wanted = title.trim();
  const matches = new Map(
    occurrences
      .filter((o) => o.title.trim() === wanted && o.times.startMs >= day.startMs && o.times.startMs < day.endMs)
      .map((o) => [identityKey(o.identity), o]),
  );
  const [first, ...rest] = matches.values();
  if (!first) {
    return { kind: 'no-match' };
  }
  return rest.length ? { kind: 'ambiguous' } : { kind: 'one', occurrence: first };
}

/** A calendar LiftLog may move events in, as the settings show and store it. */
export type CalendarChoice = { id: number; displayName: string; accountName: string };

/** `blocked`: denied for good, so only the system settings can grant it now. */
export type CalendarAccess = 'granted' | 'denied' | 'blocked';

/** Why an occurrence could not be read or moved. */
export type OccurrenceProblem =
  | 'no-permission'
  | 'calendar-missing'
  | 'gone'
  | 'duplicate'
  | 'unsupported'
  | 'unsynced';

/** An instance as the provider lists it, before LiftLog decides whether it may move it. */
export type InstanceRow = {
  identity: OccurrenceIdentity;
  title: string;
  times: Times;
  allDay: boolean;
  /** The calendar allows edits, and the event is LiftLog's account's own or lets guests modify it. */
  editable: boolean;
  /** Repeats by an RDATE list only, which LiftLog does not move. */
  rdateOnly: boolean;
};

export function occurrenceOf({ identity, title, times, allDay, editable, rdateOnly }: InstanceRow): Occurrence {
  const series = identity.instanceTime !== undefined;
  const problem =
    allDay || !editable || rdateOnly ? 'unsupported' : series && !identity.syncId ? 'unsynced' : undefined;
  return { identity, title, times, problem };
}

/**
 * Everything LiftLog does to the phone's calendars. Only Android has one; elsewhere `supported` is false
 * and every call reports no permission.
 */
export interface PhoneCalendar {
  readonly supported: boolean;
  access(): Promise<CalendarAccess>;
  /** Asks the user. Only the settings page calls it. */
  requestAccess(): Promise<CalendarAccess>;
  /** Visible, syncing calendars where LiftLog may edit events. Empty without access. */
  calendars(): Promise<CalendarChoice[]>;
  /** The occurrences overlapping `window` in the calendar, without cancelled ones. */
  occurrences(
    calendar: { calendarId: number; accountName: string },
    window: Times,
  ): Promise<{ kind: 'found'; occurrences: Occurrence[] } | { kind: 'no-permission' | 'calendar-missing' }>;
  /** The occurrence's current time. Changes nothing. */
  inspect(identity: OccurrenceIdentity): Promise<{ kind: 'found'; times: Times } | { kind: OccurrenceProblem }>;
  /**
   * Moves only this occurrence to `next`, never its series and never another event, and only while it
   * still shows `expected`. Writes nothing when it already shows `next`.
   */
  move(
    identity: OccurrenceIdentity,
    next: Times,
    expected: Times,
  ): Promise<{ kind: 'moved' | 'unchanged' } | { kind: 'changed'; times: Times } | { kind: OccurrenceProblem }>;
}
