import {
  CalendarAccess,
  identityKey,
  Occurrence,
  OccurrenceIdentity,
  PhoneCalendar,
  sameTimes,
  Times,
} from '@/models/calendar-sync';
import { LocalDateTime, ZoneOffset } from '@js-joda/core';

export const CALENDAR = { calendarId: 7, accountName: 'me@example.com' };

/** Epoch millis of a UTC wall time, e.g. `at('2026-10-11T18:00')`. */
export const at = (time: string) => LocalDateTime.parse(time).toInstant(ZoneOffset.UTC).toEpochMilli();
export const span = (start: string, end: string): Times => ({ startMs: at(start), endMs: at(end) });

/**
 * A phone calendar that holds occurrences by identity, the way the native module resolves them, and
 * records every write. `beforeWrite` and `afterWrite` let a spec crash or edit around a write.
 */
export class FakePhoneCalendar implements PhoneCalendar {
  readonly supported = true;
  accessState: CalendarAccess = 'granted';
  readonly writes: { key: string; times: Times }[] = [];
  beforeWrite: () => void = () => {};
  afterWrite: () => void = () => {};
  private readonly events = new Map<string, Occurrence>();
  private nextEventId = 100;

  /** Adds an occurrence of a synced daily Gym series, or of whatever the overrides say. */
  addOccurrence(times: Times, overrides: Partial<Omit<Occurrence, 'identity'>> & Partial<OccurrenceIdentity> = {}) {
    const { title = 'Gym', problem, ...identity } = overrides;
    const occurrence: Occurrence = {
      identity: {
        ...CALENDAR,
        eventId: this.nextEventId++,
        instanceTime: times.startMs,
        syncId: 'series-sync-id',
        rrule: 'FREQ=DAILY',
        title,
        organizer: undefined,
        sourceStartMs: times.startMs,
        sourceDuration: 'P3600S',
        sourceTimezone: 'UTC',
        ...identity,
      },
      title,
      times,
      problem,
    };
    this.events.set(identityKey(occurrence.identity), occurrence);
    return occurrence;
  }

  timesOf(occurrence: Occurrence): Times | undefined {
    return this.events.get(identityKey(occurrence.identity))?.times;
  }

  /** An edit made in the calendar app. */
  editOutside(occurrence: Occurrence, times: Times) {
    this.events.set(identityKey(occurrence.identity), { ...occurrence, times });
  }

  remove(occurrence: Occurrence) {
    this.events.delete(identityKey(occurrence.identity));
  }

  access() {
    return Promise.resolve(this.accessState);
  }

  requestAccess() {
    return Promise.resolve(this.accessState);
  }

  calendars() {
    return Promise.resolve(
      this.accessState === 'granted'
        ? [{ id: CALENDAR.calendarId, displayName: 'Personal', accountName: CALENDAR.accountName }]
        : [],
    );
  }

  occurrences(
    calendar: { calendarId: number; accountName: string },
    window: Times,
  ): ReturnType<PhoneCalendar['occurrences']> {
    if (this.accessState !== 'granted') {
      return Promise.resolve({ kind: 'no-permission' as const });
    }
    if (calendar.calendarId !== CALENDAR.calendarId || calendar.accountName !== CALENDAR.accountName) {
      return Promise.resolve({ kind: 'calendar-missing' as const });
    }
    return Promise.resolve({
      kind: 'found' as const,
      occurrences: [...this.events.values()].filter(
        (o) => o.times.startMs < window.endMs && o.times.endMs > window.startMs,
      ),
    });
  }

  inspect(identity: OccurrenceIdentity): ReturnType<PhoneCalendar['inspect']> {
    if (this.accessState !== 'granted') {
      return Promise.resolve({ kind: 'no-permission' as const });
    }
    const occurrence = this.events.get(identityKey(identity));
    return Promise.resolve(
      occurrence ? { kind: 'found' as const, times: occurrence.times } : { kind: 'gone' as const },
    );
  }

  async move(identity: OccurrenceIdentity, next: Times, expected: Times): ReturnType<PhoneCalendar['move']> {
    const seen = await this.inspect(identity);
    if (seen.kind !== 'found') {
      return seen;
    }
    if (sameTimes(seen.times, next)) {
      return { kind: 'unchanged' as const };
    }
    if (!sameTimes(seen.times, expected)) {
      return { kind: 'changed' as const, times: seen.times };
    }
    this.beforeWrite();
    const key = identityKey(identity);
    this.events.set(key, { ...this.events.get(key)!, times: next });
    this.writes.push({ key, times: next });
    this.afterWrite();
    return { kind: 'moved' as const };
  }
}
