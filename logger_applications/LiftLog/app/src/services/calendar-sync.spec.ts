import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { LocalDate, OffsetDateTime } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import {
  countPendingCalendarSessions,
  enrollCalendarSessions,
  readCalendarAttention,
  readPendingCalendarSessions,
  requeueCalendarAttention,
  recordCalendarOutcome,
} from '@/db/calendar-sync';
import { calendarSyncSessionsSchema } from '@/db/schema';
import { queueSessionSync } from '@/db/session-sync';
import { deleteSession, setActiveSession, writeSession } from '@/db/sessions';
import { SessionBlueprint } from '@/models/blueprint-models';
import { CalendarTarget } from '@/models/calendar-sync';
import { Session } from '@/models/session-models';
import { makeRecordedExercise, makeWeightedBlueprint } from '@/models/session-models/__test__/helpers';
import { syncCalendarSessions } from '@/services/calendar-sync';
import { at, CALENDAR, FakePhoneCalendar, span } from '@/services/__test__/fake-phone-calendar';
import { createTestDb } from '@/utils/__test__/test-db';

const TARGET: CalendarTarget = { ...CALENDAR, title: 'Gym' };

/** A finished workout whose completed sets are at the given UTC times (or instants) of `date`. */
function workout(id: string, date: string, ...times: string[]): Session {
  const blueprint = makeWeightedBlueprint();
  return new Session(
    id,
    new SessionBlueprint('Push', [blueprint], ''),
    [
      makeRecordedExercise(
        blueprint,
        times.map(() => 5),
        undefined,
        (i) => OffsetDateTime.parse(times[i]!.includes('T') ? times[i]! : `${date}T${times[i]}:00Z`),
      ),
    ],
    LocalDate.parse(date),
    undefined,
    undefined,
  );
}

describe('syncCalendarSessions', () => {
  let db: ExpoSQLiteDatabase;
  let calendar: FakePhoneCalendar;
  const logger = { warn: vi.fn() };

  // The day window and the matching day follow the phone's time zone.
  beforeAll(() => {
    vi.stubEnv('TZ', 'UTC');
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });

  beforeEach(async () => {
    db = await createTestDb();
    calendar = new FakePhoneCalendar();
  });

  const run = () => syncCalendarSessions({ db, calendar, logger, enabled: () => true });
  const finish = (session: Session, target = TARGET) =>
    db.transaction((tx) => {
      writeSession(tx, session);
      enrollCalendarSessions(tx, [session.id], target);
    });
  const edit = (session: Session) =>
    db.transaction((tx) => queueSessionSync(tx, [session.id], () => writeSession(tx, session)));
  const outcomes = () =>
    db
      .select({ sessionId: calendarSyncSessionsSchema.sessionId, outcome: calendarSyncSessionsSchema.outcome })
      .from(calendarSyncSessionsSchema)
      .all()
      .map(({ sessionId, outcome }) => `${sessionId}: ${outcome}`);

  it('moves the day’s Gym occurrence to the workout, and the same one again after a History edit', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    const nextDay = calendar.addOccurrence(span('2026-10-12T17:00', '2026-10-12T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    expect(await run()).toEqual({ noPermission: false, retry: false });
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));

    edit(workout('w', '2026-10-11', '18:05', '19:40'));
    await run();

    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:40'));
    expect(calendar.timesOf(nextDay)).toEqual(span('2026-10-12T17:00', '2026-10-12T18:00'));
    expect(new Set(calendar.writes.map((w) => w.key)).size).toBe(1);
    expect(outcomes()).toEqual(['w: adjusted']);
    expect(countPendingCalendarSessions(db)).toBe(0);
  });

  it('writes whole seconds that cover the workout', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '2026-10-11T18:05:00.600Z', '2026-10-11T19:10:00.400Z'));

    await run();

    expect(calendar.timesOf(gym)).toEqual({ startMs: at('2026-10-11T18:05'), endMs: at('2026-10-11T19:10:01') });
  });

  it('creates nothing on a day without a Gym event, and lists the workout', async () => {
    calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'), { title: 'Dentist' });
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    await run();

    expect(calendar.writes).toEqual([]);
    expect(readCalendarAttention(db)).toEqual([
      { sessionId: 'w', date: LocalDate.parse('2026-10-11'), outcome: 'no-match' },
    ]);
  });

  it('counts every Gym on the day, including one it could not move, before choosing', async () => {
    calendar.addOccurrence(span('2026-10-11T07:00', '2026-10-11T08:00'));
    calendar.addOccurrence(span('2026-10-11T00:00', '2026-10-12T00:00'), { problem: 'unsupported' });
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    await run();

    expect(calendar.writes).toEqual([]);
    expect(outcomes()).toEqual(['w: ambiguous']);
  });

  it('blocks a lone Gym event it may not move, visibly', async () => {
    calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'), { problem: 'unsupported' });
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    await run();

    expect(calendar.writes).toEqual([]);
    expect(outcomes()).toEqual(['w: unsupported']);
  });

  it('matches a title with stray spaces, but not the previous day’s event that ends on this one', async () => {
    calendar.addOccurrence(span('2026-10-10T23:30', '2026-10-11T00:30'));
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'), { title: ' Gym ' });
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    await run();

    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
  });

  it('never lets a second workout take the day’s event, even after the first is deleted', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('first', '2026-10-11', '08:00', '09:00'));
    await run();
    finish(workout('second', '2026-10-11', '18:05', '19:10'));
    await run();

    expect(outcomes()).toEqual(['first: adjusted', 'second: claimed']);

    db.transaction((tx) => queueSessionSync(tx, ['first'], () => deleteSession(tx, 'first')));
    db.transaction((tx) => requeueCalendarAttention(tx));
    await run();

    expect(outcomes()).toEqual(['first: idle', 'second: claimed']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T08:00', '2026-10-11T09:00'));
    expect(calendar.writes).toHaveLength(1);
  });

  it('leaves the event where it is when the workout is resumed or loses its sets', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    const session = workout('w', '2026-10-11', '18:05', '19:10');
    finish(session);
    await run();

    db.transaction((tx) => queueSessionSync(tx, ['w'], () => setActiveSession(tx, session)));
    await run();
    expect(outcomes()).toEqual(['w: idle']);

    db.transaction((tx) => queueSessionSync(tx, ['w'], () => setActiveSession(tx, undefined)));
    edit(session.withNothingCompleted());
    await run();

    expect(outcomes()).toEqual(['w: idle']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
    expect(calendar.writes).toHaveLength(1);
  });

  it('writes nothing when the workout moves to another day, and follows it again once it is back', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    const nextDay = calendar.addOccurrence(span('2026-10-12T17:00', '2026-10-12T18:00'));
    const session = workout('w', '2026-10-11', '18:05', '19:10');
    finish(session);
    await run();

    edit(session.withUpdatedDate(LocalDate.parse('2026-10-12')));
    await run();

    expect(readCalendarAttention(db)).toEqual([
      { sessionId: 'w', date: LocalDate.parse('2026-10-11'), outcome: 'moved-date' },
    ]);
    expect(calendar.writes).toHaveLength(1);
    expect(calendar.timesOf(nextDay)).toEqual(span('2026-10-12T17:00', '2026-10-12T18:00'));

    edit(workout('w', '2026-10-11', '18:00', '19:30'));
    await run();
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:00', '2026-10-11T19:30'));
  });

  it('does not overwrite a time changed in the calendar, and writes again once it is put back', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    await run();
    calendar.editOutside(gym, span('2026-10-11T20:00', '2026-10-11T21:00'));

    edit(workout('w', '2026-10-11', '18:05', '19:40'));
    await run();

    expect(outcomes()).toEqual(['w: changed']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T20:00', '2026-10-11T21:00'));

    calendar.editOutside(gym, span('2026-10-11T18:05', '2026-10-11T19:10'));
    db.transaction((tx) => requeueCalendarAttention(tx));
    await run();

    expect(outcomes()).toEqual(['w: adjusted']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:40'));
  });

  it('does not take over an event changed between matching and the first write', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    const occurrences = calendar.occurrences.bind(calendar);
    calendar.occurrences = async (...args) => {
      const found = await occurrences(...args);
      calendar.editOutside(gym, span('2026-10-11T16:00', '2026-10-11T17:00'));
      return found;
    };

    await run();

    expect(outcomes()).toEqual(['w: changed']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T16:00', '2026-10-11T17:00'));
  });

  it('keeps workouts pending without calendar access, and moves them once it is granted', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    calendar.accessState = 'denied';
    finish(workout('w', '2026-10-11', '18:05', '19:10'));

    expect(await run()).toEqual({ noPermission: true, retry: false });
    expect(countPendingCalendarSessions(db)).toBe(1);

    calendar.accessState = 'granted';
    await run();

    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
    expect(countPendingCalendarSessions(db)).toBe(0);
  });

  it('recognises a write that landed before a crash, without writing it twice', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    calendar.afterWrite = () => {
      throw new Error('killed');
    };

    expect(await run()).toEqual({ noPermission: false, retry: true });
    expect(readPendingCalendarSessions(db)[0]?.claim?.attempt).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));

    calendar.afterWrite = () => {};
    await run();

    expect(calendar.writes).toHaveLength(1);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
    expect(outcomes()).toEqual(['w: adjusted']);
    expect(readCalendarAttention(db)).toEqual([]);
  });

  it('settles an attempt that never landed, then writes the newest workout', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    calendar.beforeWrite = () => {
      throw new Error('killed');
    };
    await run();
    calendar.beforeWrite = () => {};

    edit(workout('w', '2026-10-11', '18:05', '19:50'));
    await run();

    expect(calendar.writes.map((w) => w.times)).toEqual([span('2026-10-11T18:05', '2026-10-11T19:50')]);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:50'));
  });

  it('keeps a workout edited during a write pending, so the edit is written next', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    calendar.afterWrite = () => {
      calendar.afterWrite = () => {};
      edit(workout('w', '2026-10-11', '18:05', '19:30'));
    };

    await run();
    expect(countPendingCalendarSessions(db)).toBe(1);
    await run();

    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:30'));
    expect(countPendingCalendarSessions(db)).toBe(0);
  });

  it.each(['pause', 'background', 'resume', 'delete', 'date', 'span'] as const)(
    'leaves the revision pending after %s during occurrence lookup',
    async (change) => {
      const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
      const session = workout('w', '2026-10-11', '18:05', '19:10');
      finish(session);
      let enabled = true;
      const original = calendar.occurrences.bind(calendar);
      const lookup = vi.spyOn(calendar, 'occurrences').mockImplementationOnce(async (...args) => {
        const found = await original(...args);
        switch (change) {
          case 'pause':
          case 'background':
            enabled = false;
            break;
          case 'resume':
            db.transaction((tx) => queueSessionSync(tx, ['w'], () => setActiveSession(tx, session)));
            break;
          case 'delete':
            db.transaction((tx) => queueSessionSync(tx, ['w'], () => deleteSession(tx, 'w')));
            break;
          case 'date':
            edit(session.withUpdatedDate(LocalDate.parse('2026-10-12')));
            break;
          case 'span':
            edit(workout('w', '2026-10-11', '18:05', '19:50'));
            break;
        }
        return found;
      });
      await syncCalendarSessions({ db, calendar, logger, enabled: () => enabled });
      expect(calendar.writes).toEqual([]);
      expect(readPendingCalendarSessions(db)[0]?.claim).toBeUndefined();
      expect(countPendingCalendarSessions(db)).toBe(1);
      lookup.mockRestore();
      if (change === 'span') {
        await run();
        expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:50'));
      }
    },
  );

  it.each(['pause', 'resume', 'delete', 'date', 'span'] as const)(
    'leaves a claimed revision pending after %s during inspection',
    async (change) => {
      const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
      const session = workout('w', '2026-10-11', '18:05', '19:10');
      finish(session);
      await run();
      edit(workout('w', '2026-10-11', '18:05', '19:40'));
      let enabled = true;
      const original = calendar.inspect.bind(calendar);
      vi.spyOn(calendar, 'inspect').mockImplementationOnce(async (...args) => {
        const found = await original(...args);
        switch (change) {
          case 'pause':
            enabled = false;
            break;
          case 'resume':
            db.transaction((tx) => queueSessionSync(tx, ['w'], () => setActiveSession(tx, session)));
            break;
          case 'delete':
            db.transaction((tx) => queueSessionSync(tx, ['w'], () => deleteSession(tx, 'w')));
            break;
          case 'date':
            edit(session.withUpdatedDate(LocalDate.parse('2026-10-12')));
            break;
          case 'span':
            edit(workout('w', '2026-10-11', '18:05', '19:50'));
            break;
        }
        return found;
      });
      await syncCalendarSessions({ db, calendar, logger, enabled: () => enabled });
      expect(calendar.writes).toHaveLength(1);
      expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
      expect(countPendingCalendarSessions(db)).toBe(1);
      if (change === 'span') {
        await run();
        expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:50'));
      }
    },
  );

  it.each([false, true])(
    'reconciles a saved attempt with landed=%s before reading edits during recovery',
    async (landed) => {
      const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
      finish(workout('w', '2026-10-11', '18:05', '19:10'));
      const crash = () => {
        throw new Error('killed');
      };
      if (landed) calendar.afterWrite = crash;
      else calendar.beforeWrite = crash;
      await run();
      calendar.afterWrite = () => {};
      calendar.beforeWrite = () => {};
      const inspect = calendar.inspect.bind(calendar);
      vi.spyOn(calendar, 'inspect').mockImplementationOnce(async (...args) => {
        const found = await inspect(...args);
        edit(workout('w', '2026-10-11', '18:05', '19:50'));
        return found;
      });
      const before = calendar.writes.length;
      await run();
      expect(calendar.writes.slice(before).map((w) => w.times)).toEqual([span('2026-10-11T18:05', '2026-10-11T19:50')]);
      expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:50'));
      expect(countPendingCalendarSessions(db)).toBe(0);
    },
  );

  it('persists the bounded source identity with its claim', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'), {
      organizer: 'me@example.com',
      sourceTimezone: 'Europe/Berlin',
      sourceDuration: 'P3600S',
    });
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    await run();
    edit(workout('w', '2026-10-11', '18:05', '19:40'));
    expect(readPendingCalendarSessions(db)[0]?.claim?.identity).toEqual(gym.identity);
  });

  it('never regresses a newer outcome or its acknowledged revision', () => {
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    edit(workout('w', '2026-10-11', '18:05', '19:40'));
    db.transaction((tx) => recordCalendarOutcome(tx, { sessionId: 'w', seq: 2, outcome: 'changed', date: undefined }));
    db.transaction((tx) => recordCalendarOutcome(tx, { sessionId: 'w', seq: 1, outcome: 'adjusted', date: undefined }));
    const row = db.select().from(calendarSyncSessionsSchema).get();
    expect(row).toMatchObject({ seq: 2, doneSeq: 2, outcome: 'changed' });
    expect(countPendingCalendarSessions(db)).toBe(0);
  });

  it('checks the pause predicate before every queued workout', async () => {
    calendar.addOccurrence(span('2026-10-10T17:00', '2026-10-10T18:00'));
    calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('a', '2026-10-10', '18:05', '19:10'));
    finish(workout('b', '2026-10-11', '18:05', '19:10'));
    let enabled = true;
    calendar.afterWrite = () => {
      enabled = false;
    };
    await syncCalendarSessions({ db, calendar, logger, enabled: () => enabled });
    expect(calendar.writes).toHaveLength(1);
    expect(readPendingCalendarSessions(db).map((row) => row.sessionId)).toEqual(['b']);
  });

  it('keeps going past blocked workouts, and retries one waiting for its series to sync', async () => {
    calendar.addOccurrence(span('2026-10-10T17:00', '2026-10-10T18:00'), { problem: 'unsynced' });
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('a-unsynced', '2026-10-10', '18:05', '19:10'));
    finish(workout('b-nothing', '2026-10-09', '18:05', '19:10'));
    finish(workout('c-moves', '2026-10-11', '18:05', '19:10'));

    expect(await run()).toEqual({ noPermission: false, retry: true });

    expect(outcomes()).toEqual(['a-unsynced: unsynced', 'b-nothing: no-match', 'c-moves: adjusted']);
    expect(calendar.timesOf(gym)).toEqual(span('2026-10-11T18:05', '2026-10-11T19:10'));
    expect(readPendingCalendarSessions(db).map((s) => s.sessionId)).toEqual(['a-unsynced']);
  });

  it('keeps looking in the calendar a workout was finished with', async () => {
    calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'), { ...TARGET, calendarId: 99 });

    await run();

    expect(outcomes()).toEqual(['w: calendar-missing']);
    expect(calendar.writes).toEqual([]);
  });

  it('reports an event deleted from the calendar and never recreates it', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T17:00', '2026-10-11T18:00'));
    finish(workout('w', '2026-10-11', '18:05', '19:10'));
    await run();
    calendar.remove(gym);

    edit(workout('w', '2026-10-11', '18:05', '19:40'));
    await run();

    expect(outcomes()).toEqual(['w: gone']);
    expect(calendar.writes).toHaveLength(1);
  });
});
