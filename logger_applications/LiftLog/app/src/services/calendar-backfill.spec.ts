import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { LocalDate } from '@js-joda/core';
import { sql } from 'drizzle-orm';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { claimOccurrence, enrollCalendarSessions, readPendingCalendarSessions } from '@/db/calendar-sync';
import { calendarClaimsSchema, calendarSyncSessionsSchema } from '@/db/schema';
import { deleteSession, setActiveSession, writeSession } from '@/db/sessions';
import { BackfillRequest, parseBackfillRange } from '@/models/calendar-backfill';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
import { applyCalendarBackfill, defaultBackfillRange, previewCalendarBackfill } from '@/services/calendar-backfill';
import { syncCalendarSessions } from '@/services/calendar-sync';
import { CALENDAR, FakePhoneCalendar, span } from '@/services/__test__/fake-phone-calendar';
import { createTestDb } from '@/utils/__test__/test-db';

const target = { ...CALENDAR, title: 'GYM🏋️' };
const range = { from: LocalDate.parse('2026-10-10'), to: LocalDate.parse('2026-10-12') };
const request: BackfillRequest = { target, range };
const workout = (id: string, date = '2026-10-11', minutes = [0, 30]) => makeWorkoutOn(date, minutes).with({ id });

describe('explicit calendar history backfill', () => {
  let db: ExpoSQLiteDatabase;
  let calendar: FakePhoneCalendar;
  beforeAll(() => vi.stubEnv('TZ', 'UTC'));
  afterAll(() => vi.unstubAllEnvs());
  beforeEach(async () => {
    db = await createTestDb();
    calendar = new FakePhoneCalendar();
  });
  const preview = () => previewCalendarBackfill({ db, calendar }, request);
  const gym = (date = '2026-10-11') =>
    calendar.addOccurrence(span(`${date}T15:00`, `${date}T16:00`), { title: target.title });
  const save = (...sessions: ReturnType<typeof workout>[]) =>
    db.transaction((tx) => sessions.forEach((s) => writeSession(tx, s)));
  const stored = () => ({
    sessions: db.select().from(calendarSyncSessionsSchema).all(),
    claims: db.select().from(calendarClaimsSchema).all(),
  });
  const apply = (snapshot: Awaited<ReturnType<typeof preview>>, selectedIds: string[], getRequest = () => request) =>
    applyCalendarBackfill({ db, calendar }, { preview: snapshot, selectedIds, getRequest });
  const drain = () => syncCalendarSessions({ db, calendar, enabled: () => true, logger: { warn: vi.fn() } });

  it('previews old and new spans without changing SQLite or the provider', async () => {
    save(workout('a'));
    const event = gym();
    const before = stored();
    const result = await preview();
    expect(result.rows).toHaveLength(1);
    expect(result.rows[0]).toMatchObject({
      sessionId: 'a',
      name: 'Push',
      status: 'ready',
      completedSpan: span('2026-10-11T16:00', '2026-10-11T16:30'),
      desired: span('2026-10-11T16:00', '2026-10-11T16:30'),
      candidate: { identity: event.identity, current: event.times },
    });
    expect(stored()).toEqual(before);
    expect(calendar.writes).toEqual([]);
  });

  it('skips active, empty, zero-length and out-of-range sessions', async () => {
    save(
      workout('ready'),
      workout('empty').withNothingCompleted(),
      workout('zero', '2026-10-11', [0]),
      workout('outside', '2026-10-09'),
    );
    db.transaction((tx) => setActiveSession(tx, workout('active')));
    gym();
    expect((await preview()).rows.map((row) => row.sessionId)).toEqual(['ready']);
    expect(defaultBackfillRange(db)).toEqual({
      from: LocalDate.parse('2026-10-09'),
      to: LocalDate.parse('2026-10-11'),
    });
  });

  it('shows already enrolled history without rematching or changing its target', async () => {
    save(workout('a'));
    db.transaction((tx) => enrollCalendarSessions(tx, ['a'], { ...target, title: 'Old target' }));
    const lookup = vi.spyOn(calendar, 'occurrences');
    const before = stored();
    expect((await preview()).rows[0]?.status).toBe('already-linked');
    expect(lookup).not.toHaveBeenCalled();
    expect(stored()).toEqual(before);
  });

  it('uses exact configured title and counts unsupported matches before suitability', async () => {
    save(workout('a'));
    calendar.addOccurrence(span('2026-10-11T09:00', '2026-10-11T10:00'), { title: 'Gym' });
    expect((await preview()).rows[0]?.status).toBe('missing');
    gym();
    calendar.addOccurrence(span('2026-10-11T00:00', '2026-10-12T00:00'), {
      title: target.title,
      problem: 'unsupported',
    });
    expect((await preview()).rows[0]?.status).toBe('ambiguous');
  });

  it('shows a sole unsupported occurrence without enrolling it', async () => {
    save(workout('a'));
    calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'), { title: target.title, problem: 'unsynced' });
    const snapshot = await preview();
    expect(snapshot.rows[0]?.status).toBe('unsupported');
    expect((await apply(snapshot, ['a'])).queued).toBe(0);
    expect(stored().sessions).toEqual([]);
  });

  it('marks every workout competing for one occurrence and applies none of that group', async () => {
    save(workout('a'), workout('b'));
    gym();
    const lookup = vi.spyOn(calendar, 'occurrences');
    const snapshot = await preview();
    expect(snapshot.rows.map((row) => row.status)).toEqual(['conflicting-workouts', 'conflicting-workouts']);
    expect(lookup).toHaveBeenCalledOnce();
    expect((await apply(snapshot, ['a', 'b'])).rows.map((row) => row.status)).toEqual(['not-ready', 'not-ready']);
    expect(stored()).toEqual({ sessions: [], claims: [] });
  });

  it('queues only selected ready rows and pins their preview identity and spans', async () => {
    save(workout('a'), workout('b', '2026-10-12'));
    const event = gym();
    gym('2026-10-12');
    const snapshot = await preview();
    expect(await apply(snapshot, ['a'])).toEqual({ queued: 1, rows: [{ sessionId: 'a', status: 'queued' }] });
    expect(calendar.writes).toEqual([]);
    expect(readPendingCalendarSessions(db)).toMatchObject([
      {
        sessionId: 'a',
        target,
        claim: {
          identity: event.identity,
          original: event.times,
          observed: event.times,
          attempt: span('2026-10-11T16:00', '2026-10-11T16:30'),
        },
      },
    ]);
    const lookup = vi.spyOn(calendar, 'occurrences');
    await drain();
    expect(lookup).not.toHaveBeenCalled();
    expect(calendar.timesOf(event)).toEqual(span('2026-10-11T16:00', '2026-10-11T16:30'));
  });

  it('makes duplicate apply and a restarted drain idempotent', async () => {
    save(workout('a'));
    gym();
    const snapshot = await preview();
    await apply(snapshot, ['a', 'a']);
    const before = stored();
    expect((await apply(snapshot, ['a'])).rows[0]?.status).toBe('already-linked');
    expect(stored()).toEqual(before);
    calendar.afterWrite = () => {
      throw new Error('interrupted after provider write');
    };
    await drain();
    calendar.afterWrite = () => {};
    await drain();
    await drain();
    expect(calendar.writes).toHaveLength(1);
    expect(stored().claims).toHaveLength(1);
    expect((await preview()).rows[0]?.status).toBe('already-linked');
  });

  it.each(['span', 'date', 'resume', 'delete'] as const)(
    'rejects a workout changed by %s since preview',
    async (change) => {
      const session = workout('a');
      save(session);
      gym();
      const snapshot = await preview();
      switch (change) {
        case 'span':
          save(workout('a', '2026-10-11', [0, 50]));
          break;
        case 'date':
          save(session.withUpdatedDate(LocalDate.parse('2026-10-12')));
          break;
        case 'resume':
          db.transaction((tx) => setActiveSession(tx, session));
          break;
        case 'delete':
          db.transaction((tx) => deleteSession(tx, session.id));
          break;
      }
      expect((await apply(snapshot, ['a'])).rows[0]?.status).toBe('stale');
      expect(stored()).toEqual({ sessions: [], claims: [] });
    },
  );

  it('rejects changed configuration and date range, including changes during permission lookup', async () => {
    save(workout('a'));
    gym();
    const snapshot = await preview();
    let current = request;
    calendar.access = async () => {
      current = { ...request, target: { ...target, title: 'Changed' } };
      return 'granted';
    };
    expect((await apply(snapshot, ['a'], () => current)).rows[0]?.status).toBe('stale');
    current = { ...request, range: { ...range, from: range.from.plusDays(1) } };
    expect((await apply(snapshot, ['a'], () => current)).rows[0]?.status).toBe('stale');
    expect(stored().sessions).toEqual([]);
  });

  it('never rematches or overwrites an event changed after preview', async () => {
    save(workout('a'));
    const event = gym();
    const snapshot = await preview();
    const external = span('2026-10-11T20:00', '2026-10-11T21:00');
    calendar.editOutside(event, external);
    await apply(snapshot, ['a']);
    const lookup = vi.spyOn(calendar, 'occurrences');
    await drain();
    expect(lookup).not.toHaveBeenCalled();
    expect(calendar.writes).toEqual([]);
    expect(calendar.timesOf(event)).toEqual(external);
    expect(stored().sessions[0]?.outcome).toBe('changed');
    expect(stored().claims[0]?.eventId).toBe(event.identity.eventId);
  });

  it('retains the preview identity when a replacement matching event appears', async () => {
    save(workout('a'));
    const original = gym();
    const snapshot = await preview();
    calendar.remove(original);
    const replacement = gym();
    await apply(snapshot, ['a']);
    await drain();
    expect(calendar.writes).toEqual([]);
    expect(stored().claims[0]?.eventId).toBe(original.identity.eventId);
    expect(stored().sessions[0]?.outcome).toBe('gone');
    expect(calendar.timesOf(replacement)).toEqual(replacement.times);
  });

  it('rereads the actual workout after the awaited permission check on apply', async () => {
    save(workout('a'));
    gym();
    const snapshot = await preview();
    calendar.access = async () => {
      save(workout('a', '2026-10-11', [0, 50]));
      return 'granted';
    };
    expect((await apply(snapshot, ['a'])).rows[0]?.status).toBe('stale');
    expect(stored()).toEqual({ sessions: [], claims: [] });
  });

  it('retains an existing owner even if the claim appears after preview', async () => {
    save(workout('a'));
    const event = gym();
    const snapshot = await preview();
    db.transaction((tx) => {
      enrollCalendarSessions(tx, ['owner'], target);
      claimOccurrence(tx, 'owner', {
        identity: event.identity,
        date: LocalDate.parse('2026-10-11'),
        original: event.times,
        attempt: event.times,
      });
    });
    const before = stored();
    expect((await apply(snapshot, ['a'])).rows[0]?.status).toBe('claimed');
    expect((await preview()).rows[0]?.status).toBe('claimed');
    expect(stored()).toEqual(before);
  });

  it('rolls back new enrollment if pinning its claim fails', async () => {
    save(workout('a'));
    gym();
    const snapshot = await preview();
    db.run(
      sql.raw(
        "CREATE TRIGGER fail_claim BEFORE INSERT ON calendar_claim BEGIN SELECT RAISE(FAIL, 'test failure'); END",
      ),
    );
    expect((await apply(snapshot, ['a'])).rows[0]?.status).toBe('error');
    expect(stored()).toEqual({ sessions: [], claims: [] });
  });

  it('shows denied permission and safely refuses apply if permission is revoked', async () => {
    save(workout('a'));
    gym();
    const snapshot = await preview();
    calendar.accessState = 'denied';
    expect((await preview()).rows[0]).toMatchObject({ status: 'error', reason: 'no-permission' });
    expect((await apply(snapshot, ['a'])).rows[0]).toMatchObject({ status: 'error', reason: 'no-permission' });
    expect(stored().sessions).toEqual([]);
  });

  it('shows provider errors and removed calendars without changing history', async () => {
    save(workout('a'));
    const missing = await previewCalendarBackfill(
      { db, calendar },
      { ...request, target: { ...target, calendarId: 99 } },
    );
    expect(missing.rows[0]).toMatchObject({ status: 'error', reason: 'calendar-missing' });
    vi.spyOn(calendar, 'occurrences').mockRejectedValueOnce(new Error('provider unavailable'));
    expect((await preview()).rows[0]).toMatchObject({ status: 'error', reason: 'provider-error' });
    expect(stored().sessions).toEqual([]);
  });

  it('accepts only a valid inclusive ISO date range', () => {
    expect(parseBackfillRange('2026-10-10', '2026-10-12')).toEqual(range);
    expect(parseBackfillRange('2026-02-30', '2026-10-12')).toBeUndefined();
    expect(parseBackfillRange('2026-10-12', '2026-10-10')).toBeUndefined();
    expect(parseBackfillRange('10/10/2026', '2026-10-12')).toBeUndefined();
  });
});
