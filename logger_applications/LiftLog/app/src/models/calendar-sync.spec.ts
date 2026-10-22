import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';
import { LocalDate, OffsetDateTime } from '@js-joda/core';
import { dayWindow, matchOccurrence, Occurrence, occurrenceOf, workoutTimes } from '@/models/calendar-sync';

const identity = {
  calendarId: 1,
  accountName: 'me',
  eventId: 5,
  instanceTime: 0,
  syncId: 's',
  rrule: 'FREQ=DAILY',
  title: 'Gym',
  organizer: undefined,
  sourceStartMs: 0,
  sourceDuration: 'P3600S',
  sourceTimezone: 'UTC',
};
const ms = (instant: string) => OffsetDateTime.parse(instant).toInstant().toEpochMilli();

describe('calendar sync model', () => {
  // The day window and the matching day follow the phone's time zone.
  beforeAll(() => {
    vi.stubEnv('TZ', 'Europe/Berlin');
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });

  it('spans a local day from midnight to midnight, 25 hours when summer time ends', () => {
    expect(dayWindow(LocalDate.parse('2026-10-25'))).toEqual({
      startMs: ms('2026-10-25T00:00:00+02:00'),
      endMs: ms('2026-10-26T00:00:00+01:00'),
    });
    const spring = dayWindow(LocalDate.parse('2026-03-29'));
    expect(spring.endMs - spring.startMs).toBe(23 * 3_600_000);
  });

  it('widens a workout to whole seconds and leaves one without length alone', () => {
    const start = OffsetDateTime.parse('2026-10-11T18:05:00.250+02:00');
    expect(workoutTimes({ start, end: start.plusMinutes(70) })).toEqual({
      startMs: ms('2026-10-11T18:05:00+02:00'),
      endMs: ms('2026-10-11T19:15:01+02:00'),
    });
    expect(workoutTimes({ start, end: start })).toBeUndefined();
  });

  describe('matchOccurrence', () => {
    const day = dayWindow(LocalDate.parse('2026-10-11'));
    const gym = (start: string, overrides: Partial<Occurrence> = {}, instanceTime = ms(start)): Occurrence => ({
      identity: { ...identity, instanceTime },
      title: 'Gym',
      times: { startMs: ms(start), endMs: ms(start) + 3_600_000 },
      problem: undefined,
      ...overrides,
    });

    it('takes the one exact title that starts on the day', () => {
      const one = gym('2026-10-11T18:00:00+02:00');
      expect(
        matchOccurrence(
          [one, gym('2026-10-10T23:30:00+02:00'), gym('2026-10-11T08:00:00+02:00', { title: 'gym' })],
          ' Gym',
          day,
        ),
      ).toEqual({ kind: 'one', occurrence: one });
    });

    it('reports none, and two even when one of them cannot be moved', () => {
      expect(matchOccurrence([], 'Gym', day)).toEqual({ kind: 'no-match' });
      expect(
        matchOccurrence(
          [gym('2026-10-11T08:00:00+02:00'), gym('2026-10-11T18:00:00+02:00', { problem: 'unsupported' })],
          'Gym',
          day,
        ),
      ).toEqual({ kind: 'ambiguous' });
    });

    it('counts the same occurrence listed twice once', () => {
      const one = gym('2026-10-11T18:00:00+02:00');
      expect(matchOccurrence([one, { ...one }], 'Gym', day)).toEqual({ kind: 'one', occurrence: one });
    });
  });

  it('says why an occurrence cannot be moved', () => {
    const row = {
      identity,
      title: 'Gym',
      times: { startMs: 0, endMs: 1 },
      allDay: false,
      editable: true,
      rdateOnly: false,
    };
    expect(occurrenceOf(row).problem).toBeUndefined();
    expect(occurrenceOf({ ...row, allDay: true }).problem).toBe('unsupported');
    expect(occurrenceOf({ ...row, editable: false }).problem).toBe('unsupported');
    expect(occurrenceOf({ ...row, rdateOnly: true }).problem).toBe('unsupported');
    expect(occurrenceOf({ ...row, identity: { ...identity, syncId: undefined } }).problem).toBe('unsynced');
    expect(
      occurrenceOf({ ...row, identity: { ...identity, instanceTime: undefined, syncId: undefined } }).problem,
    ).toBeUndefined();
  });
});
