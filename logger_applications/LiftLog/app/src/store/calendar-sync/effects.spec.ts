import * as calendarDb from '@/db/calendar-sync';
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { combineReducers } from '@reduxjs/toolkit';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { countPendingCalendarSessions, enrollCalendarSessions } from '@/db/calendar-sync';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
import { CALENDAR, FakePhoneCalendar, span } from '@/services/__test__/fake-phone-calendar';
import { appForegroundChanged } from '@/store/app';
import { calendarRetryRequested, calendarSessionsQueued, calendarSyncReducer } from '@/store/calendar-sync';
import { applyCalendarSyncEffects } from '@/store/calendar-sync/effects';
import { setIsHydrated as setSettingsHydrated, setCalendarSyncEnabled, settingsReducer } from '@/store/settings';
import { setIsHydrated, storedSessionsReducer } from '@/store/stored-sessions';
import { createAddEffectTestBed } from '@/utils/__test__/add-effect-testbed';
import { createTestDb } from '@/utils/__test__/test-db';

// makeWorkoutOn's sets are at 18:00 and 18:30 on its day, in +02:00.
const WORKOUT = span('2026-10-11T16:00', '2026-10-11T16:30');

describe('calendar sync effects', () => {
  let db: ExpoSQLiteDatabase;
  let calendar: FakePhoneCalendar;

  // The day window and the matching day follow the phone's time zone.
  beforeAll(() => {
    vi.stubEnv('TZ', 'UTC');
  });
  afterAll(() => {
    vi.unstubAllEnvs();
  });

  beforeEach(async () => {
    vi.useFakeTimers();
    const workout = makeWorkoutOn('2026-10-11', [0, 30]);
    db = await createTestDb([workout]);
    db.transaction((tx) => enrollCalendarSessions(tx, [workout.id], { ...CALENDAR, title: 'Gym' }));
    calendar = new FakePhoneCalendar();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function bed({ enabled = true, hydrated = true, settingsHydrated = true } = {}) {
    const testBed = createAddEffectTestBed({
      reducer: combineReducers({
        settings: settingsReducer,
        storedSessions: storedSessionsReducer,
        calendarSync: calendarSyncReducer,
      }),
      initialState: {
        settings: { calendarSyncEnabled: enabled, isHydrated: settingsHydrated },
        storedSessions: { isHydrated: hydrated },
      },
      services: { db, phoneCalendar: calendar },
    });
    applyCalendarSyncEffects(testBed.addEffect);
    return testBed;
  }

  it('moves the event shortly after a workout is queued, and reports what is left', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    const testBed = bed();

    await testBed.dispatchHandled(calendarSessionsQueued());
    expect(testBed.getState().calendarSync.pending).toBe(1);
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
    expect(testBed.getState().calendarSync).toMatchObject({ pending: 0, running: false, attention: [] });
  });

  it('keeps workouts pending while paused, and moves them once sync is turned back on', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    const testBed = bed({ enabled: false });
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(60_000);
    expect(calendar.writes).toEqual([]);
    expect(countPendingCalendarSessions(db)).toBe(1);

    await testBed.dispatchHandled(setCalendarSyncEnabled(true));
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
  });

  it('picks up pending workouts on launch, once stored sessions have loaded', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    const testBed = bed({ hydrated: false });

    await testBed.dispatchHandled(setIsHydrated(true));
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
  });

  it('shows missing access and moves the event when the app returns with access granted', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    calendar.accessState = 'denied';
    const testBed = bed();
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    expect(testBed.getState().calendarSync).toMatchObject({ noPermission: true, pending: 1 });

    calendar.accessState = 'granted';
    await testBed.dispatchHandled(appForegroundChanged(true));
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
    expect(testBed.getState().calendarSync).toMatchObject({ noPermission: false, pending: 0 });
  });

  it('retries a workout waiting for its series to sync with backoff, only in the foreground', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'), { problem: 'unsynced' });
    const occurrences = vi.spyOn(calendar, 'occurrences');
    const testBed = bed();
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    await vi.advanceTimersByTimeAsync(15_000);
    expect(occurrences).toHaveBeenCalledTimes(2);

    await testBed.dispatchHandled(appForegroundChanged(false));
    await vi.advanceTimersByTimeAsync(600_000);
    expect(occurrences).toHaveBeenCalledTimes(2);

    calendar.remove(gym);
    const synced = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    await testBed.dispatchHandled(appForegroundChanged(true));
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(synced)).toEqual(WORKOUT);
    expect(testBed.getState().calendarSync.attention).toEqual([]);
  });

  it.each(['pause', 'background'] as const)(
    'stops a native lookup on %s and does not claim or write',
    async (change) => {
      calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
      const testBed = bed();
      const original = calendar.occurrences.bind(calendar);
      vi.spyOn(calendar, 'occurrences').mockImplementationOnce(async (...args) => {
        const result = await original(...args);
        await testBed.dispatchHandled(change === 'pause' ? setCalendarSyncEnabled(false) : appForegroundChanged(false));
        return result;
      });
      await testBed.dispatchHandled(calendarSessionsQueued());
      await vi.advanceTimersByTimeAsync(1_500);
      expect(calendar.writes).toEqual([]);
      expect(testBed.getState().calendarSync).toMatchObject({ running: false, pending: 1 });
      await vi.advanceTimersByTimeAsync(60_000);
      expect(calendar.writes).toEqual([]);
    },
  );

  it('waits for settings hydration and wakes when settings load', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    const testBed = bed({ settingsHydrated: false });
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    expect(calendar.writes).toEqual([]);
    await testBed.dispatchHandled(setSettingsHydrated(true));
    await vi.advanceTimersByTimeAsync(1_500);
    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
  });

  it('clears the running indicator after a failure outside a single session and can retry', async () => {
    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    const testBed = bed();
    const pending = vi.spyOn(calendarDb, 'readPendingCalendarSessions').mockImplementationOnce(() => {
      throw new Error('SQLite unavailable');
    });
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    expect(testBed.getState().calendarSync).toMatchObject({ running: false, pending: 1 });
    pending.mockRestore();
    await vi.advanceTimersByTimeAsync(15_000);
    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
    expect(testBed.getState().calendarSync.running).toBe(false);
  });

  it('lists a workout that needs attention, and Try again looks again', async () => {
    const testBed = bed();
    await testBed.dispatchHandled(calendarSessionsQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    expect(testBed.getState().calendarSync.attention.map((a) => a.outcome)).toEqual(['no-match']);

    const gym = calendar.addOccurrence(span('2026-10-11T15:00', '2026-10-11T16:00'));
    await testBed.dispatchHandled(calendarRetryRequested());
    await vi.advanceTimersByTimeAsync(1_500);

    expect(calendar.timesOf(gym)).toEqual(WORKOUT);
    expect(testBed.getState().calendarSync.attention).toEqual([]);
  });
});
