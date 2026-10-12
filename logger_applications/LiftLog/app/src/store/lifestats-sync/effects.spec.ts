import { afterEach, beforeEach, describe, expect, it, Mock, vi } from 'vitest';
import { combineReducers } from '@reduxjs/toolkit';
import { LocalDate } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { enqueueSyncDates, readSyncQueue } from '@/db/lifestats-sync';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
import { appForegroundChanged } from '@/store/app';
import { lifestatsDaysQueued, lifestatsSyncAllRequested, lifestatsSyncReducer } from '@/store/lifestats-sync';
import { applyLifestatsSyncEffects } from '@/store/lifestats-sync/effects';
import { setLifestatsSyncEnabled, settingsReducer } from '@/store/settings';
import { storedSessionsReducer } from '@/store/stored-sessions';
import { createAddEffectTestBed } from '@/utils/__test__/add-effect-testbed';
import { createTestDb } from '@/utils/__test__/test-db';

const ACCEPTED = '{"statistic_id":129,"days_updated":1,"days_cleared":0,"days_unchanged":0}';

describe('lifestats sync effects', () => {
  let db: ExpoSQLiteDatabase;
  let fetch: Mock<typeof globalThis.fetch>;

  beforeEach(async () => {
    vi.useFakeTimers();
    db = await createTestDb([makeWorkoutOn('2026-10-09'), makeWorkoutOn('2026-10-11')]);
    fetch = vi.fn<typeof globalThis.fetch>().mockImplementation(() => Promise.resolve(new Response(ACCEPTED)));
    vi.stubGlobal('fetch', fetch);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  function bed({ enabled = true } = {}) {
    const testBed = createAddEffectTestBed({
      reducer: combineReducers({
        settings: settingsReducer,
        storedSessions: storedSessionsReducer,
        lifestatsSync: lifestatsSyncReducer,
      }),
      initialState: {
        settings: { lifestatsSyncEnabled: enabled, lifestatsSyncUrl: 'https://lifestats.test/' },
        storedSessions: { isHydrated: true },
      },
      services: { db },
    });
    applyLifestatsSyncEffects(testBed.addEffect);
    return testBed;
  }

  const queue = (...dates: string[]) =>
    db.transaction((tx) =>
      enqueueSyncDates(
        tx,
        dates.map((d) => LocalDate.parse(d)),
      ),
    );
  const queued = () => readSyncQueue(db).map((d) => d.date.toString());
  const sentDays = () =>
    fetch.mock.calls.map(([, init]) =>
      (JSON.parse(init?.body as string) as { days: { date: string }[] }).days.map((d) => d.date),
    );

  it('pushes queued days shortly after they are queued and reports the result', async () => {
    const testBed = bed();
    queue('2026-10-11');

    await testBed.dispatchHandled(lifestatsDaysQueued());
    expect(testBed.getState().lifestatsSync.pendingDays).toBe(1);
    await vi.advanceTimersByTimeAsync(1_500);

    expect(fetch.mock.calls.map(([url]) => url)).toEqual(['https://lifestats.test/integrations/liftlog/days']);
    expect(queued()).toEqual([]);
    const { pendingDays, pushing, lastPush } = testBed.getState().lifestatsSync;
    expect({ pendingDays, pushing, outcome: lastPush?.outcome }).toEqual({
      pendingDays: 0,
      pushing: false,
      outcome: { kind: 'delivered', days: 1 },
    });
  });

  it('holds queued days while sync is off and sends them once it is turned on', async () => {
    const testBed = bed({ enabled: false });
    queue('2026-10-11');
    await testBed.dispatchHandled(lifestatsDaysQueued());
    await vi.advanceTimersByTimeAsync(60_000);
    expect(sentDays()).toEqual([]);

    await testBed.dispatchHandled(setLifestatsSyncEnabled(true));
    await vi.advanceTimersByTimeAsync(1_500);

    expect(sentDays()).toEqual([['2026-10-11']]);
  });

  it('retries with backoff while in the foreground, and not in the background', async () => {
    fetch.mockImplementation(() => Promise.resolve(new Response('down', { status: 503 })));
    const testBed = bed();
    queue('2026-10-11');
    await testBed.dispatchHandled(lifestatsDaysQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    await vi.advanceTimersByTimeAsync(15_000);
    expect(fetch).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(60_000);
    expect(fetch).toHaveBeenCalledTimes(3);

    await testBed.dispatchHandled(appForegroundChanged(false));
    await vi.advanceTimersByTimeAsync(600_000);
    expect(fetch).toHaveBeenCalledTimes(3);

    fetch.mockImplementation(() => Promise.resolve(new Response(ACCEPTED)));
    await testBed.dispatchHandled(appForegroundChanged(true));
    await vi.advanceTimersByTimeAsync(1_500);
    expect(fetch).toHaveBeenCalledTimes(4);
    expect(queued()).toEqual([]);
  });

  it('runs one push at a time, and one more pass for days queued during it', async () => {
    let release: () => void = () => {};
    fetch.mockImplementationOnce(() => new Promise((resolve) => (release = () => resolve(new Response(ACCEPTED)))));
    const testBed = bed();
    queue('2026-10-09');
    await testBed.dispatchHandled(lifestatsDaysQueued());
    await vi.advanceTimersByTimeAsync(1_500);

    queue('2026-10-11');
    await testBed.dispatchHandled(lifestatsDaysQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    await testBed.dispatchHandled(lifestatsDaysQueued());
    await vi.advanceTimersByTimeAsync(1_500);
    release();
    await vi.advanceTimersByTimeAsync(0);

    expect(sentDays()).toEqual([['2026-10-09'], ['2026-10-11']]);
    expect(queued()).toEqual([]);
  });

  it('syncs all history: every day with a finished workout', async () => {
    const testBed = bed();

    await testBed.dispatchHandled(lifestatsSyncAllRequested());
    await vi.advanceTimersByTimeAsync(1_500);

    expect(sentDays()).toEqual([['2026-10-09', '2026-10-11']]);
  });
});
