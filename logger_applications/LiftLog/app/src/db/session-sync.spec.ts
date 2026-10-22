import { beforeEach, describe, expect, it } from 'vitest';
import { LocalDate } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { readPendingCalendarSessions } from '@/db/calendar-sync';
import { readSyncQueue } from '@/db/lifestats-sync';
import { queueSessionSync } from '@/db/session-sync';
import { deleteSession, setActiveSession, writeSession } from '@/db/sessions';
import { CalendarTarget } from '@/models/calendar-sync';
import { Session } from '@/models/session-models';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
import { createTestDb } from '@/utils/__test__/test-db';

const TARGET: CalendarTarget = { calendarId: 7, accountName: 'me@example.com', title: 'Gym' };
const day = (date: string) => LocalDate.parse(date);

describe('queueSessionSync', () => {
  let db: ExpoSQLiteDatabase;

  beforeEach(async () => {
    db = await createTestDb();
  });

  const queuedDays = () => readSyncQueue(db).map(({ date, seq }) => [date.toString(), seq]);
  const enrolled = () =>
    readPendingCalendarSessions(db).map(({ sessionId, seq, target }) => ({ sessionId, seq, target }));

  /** Starts `session` as the workout in progress, then finishes it the way `setActiveSessionId` does. */
  function startAndFinish(session: Session, target: CalendarTarget | undefined) {
    db.transaction((tx) => setActiveSession(tx, session));
    return db.transaction((tx) =>
      queueSessionSync(tx, [session.id], () => setActiveSession(tx, undefined), { enrollFinishedIn: target }),
    );
  }

  it('queues the old and new day of a moved workout for lifestats', () => {
    db.transaction((tx) => writeSession(tx, makeWorkoutOn('2026-10-09')));
    const moved = makeWorkoutOn('2026-10-09').withUpdatedDate(day('2026-10-10'));

    const queued = db.transaction((tx) => queueSessionSync(tx, [moved.id], () => writeSession(tx, moved)));

    expect(queued).toEqual({ lifestats: true, calendar: false });
    expect(queuedDays()).toEqual([
      ['2026-10-09', 1],
      ['2026-10-10', 1],
    ]);
  });

  it('enrolls a finished workout with the calendar and title it was finished under', () => {
    expect(startAndFinish(makeWorkoutOn('2026-10-11'), TARGET)).toEqual({ lifestats: true, calendar: true });

    expect(enrolled()).toEqual([{ sessionId: 'workout-2026-10-11', seq: 1, target: TARGET }]);
  });

  it('enrolls nothing while calendar sync is off, and later writes do not enroll it either', () => {
    const session = makeWorkoutOn('2026-10-11');
    expect(startAndFinish(session, undefined)).toEqual({ lifestats: true, calendar: false });

    db.transaction((tx) => queueSessionSync(tx, [session.id], () => writeSession(tx, session)));
    db.transaction((tx) => queueSessionSync(tx, [session.id], () => deleteSession(tx, session.id)));

    expect(enrolled()).toEqual([]);
  });

  it('does not enroll a workout finished with nothing completed', () => {
    expect(startAndFinish(makeWorkoutOn('2026-10-11').withNothingCompleted(), TARGET)).toEqual({
      lifestats: false,
      calendar: false,
    });
    expect(enrolled()).toEqual([]);
  });

  it('never enrolls restored or imported workouts, even with a target', () => {
    const restored = [makeWorkoutOn('2026-10-09'), makeWorkoutOn('2026-10-10')];

    db.transaction((tx) =>
      queueSessionSync(
        tx,
        restored.map((s) => s.id),
        () => restored.forEach((s) => writeSession(tx, s)),
        { enrollFinishedIn: TARGET },
      ),
    );

    expect(enrolled()).toEqual([]);
  });

  it('queues an enrolled workout again on History edits, resume and delete, keeping its first target', () => {
    const session = makeWorkoutOn('2026-10-11');
    startAndFinish(session, TARGET);

    const edited = db.transaction((tx) =>
      queueSessionSync(tx, [session.id], () => writeSession(tx, session.withUpdatedDate(day('2026-10-12')))),
    );
    db.transaction((tx) => queueSessionSync(tx, [session.id], () => setActiveSession(tx, session)));
    startAndFinish(session, { ...TARGET, calendarId: 8, title: 'Training' });
    db.transaction((tx) => queueSessionSync(tx, [session.id], () => deleteSession(tx, session.id)));

    expect(edited.calendar).toBe(true);
    expect(enrolled()).toEqual([{ sessionId: session.id, seq: 5, target: TARGET }]);
  });

  it('does not queue a resumed workout for each set recorded in it', () => {
    const session = makeWorkoutOn('2026-10-11', [0, 30]);
    startAndFinish(session, TARGET);
    db.transaction((tx) => queueSessionSync(tx, [session.id], () => setActiveSession(tx, session)));

    const recorded = db.transaction((tx) =>
      queueSessionSync(tx, [session.id], () => writeSession(tx, makeWorkoutOn('2026-10-11', [0, 30, 45]))),
    );

    expect(recorded).toEqual({ lifestats: false, calendar: false });
    expect(enrolled().map((s) => s.seq)).toEqual([2]);
  });

  it('rolls back the queues and the enrollment with a write that fails', () => {
    const session = makeWorkoutOn('2026-10-11');
    db.transaction((tx) => setActiveSession(tx, session));

    expect(() =>
      db.transaction((tx) =>
        queueSessionSync(
          tx,
          [session.id],
          () => {
            setActiveSession(tx, undefined);
            throw new Error('disk full');
          },
          { enrollFinishedIn: TARGET },
        ),
      ),
    ).toThrow('disk full');

    expect(queuedDays()).toEqual([]);
    expect(enrolled()).toEqual([]);
  });

  it('rolls back the enrollment when a later statement of the same transaction fails', () => {
    const session = makeWorkoutOn('2026-10-11');
    db.transaction((tx) => setActiveSession(tx, session));

    expect(() =>
      db.transaction((tx) => {
        queueSessionSync(tx, [session.id], () => setActiveSession(tx, undefined), { enrollFinishedIn: TARGET });
        throw new Error('later failure');
      }),
    ).toThrow('later failure');

    expect(enrolled()).toEqual([]);
    expect(queuedDays()).toEqual([]);
  });
});
