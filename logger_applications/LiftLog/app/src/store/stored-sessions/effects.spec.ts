import { describe, it, expect, vi, beforeEach } from 'vitest';
import { LocalDate } from '@js-joda/core';
import { drizzle } from 'drizzle-orm/expo-sqlite';
import type { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { openDatabaseAsync } from 'expo-sqlite';
import { eq } from 'drizzle-orm';
import { DatabaseMigrationService } from '@/services/database-migration-service';
import { applyStoredSessionsEffects } from '@/store/stored-sessions/effects';
import {
  deleteStoredSession,
  initializeStoredSessionsStateSlice,
  putStoredSession,
  sessionFinished,
  sessionsChanged,
  setActiveSessionId,
  closeSession,
  openSession,
  upsertExercises,
  upsertStoredSessions,
} from '@/store/stored-sessions';
import { setStatsIsDirty } from '@/store/stats';
import { createAddEffectTestBed } from '@/utils/__test__/add-effect-testbed';
import { exercisesSchema, sessionsSchema } from '@/db/schema';
import { readSession, setActiveSession, writeSession } from '@/db/sessions';
import { readSyncQueue } from '@/db/lifestats-sync';
import { lifestatsDaysQueued } from '@/store/lifestats-sync';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
import { Session } from '@/models/session-models';
import { toJsonString } from '@/models/storage/versions/latest';
import type { RootState } from '@/store/store';

async function createTestDb(): Promise<ExpoSQLiteDatabase> {
  const expoDb = await openDatabaseAsync(':memory:');
  const db = drizzle(expoDb);
  await new DatabaseMigrationService(
    db,
    { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn(), time: timeStub } as never,
    { importOldData: async () => {} },
  ).migrate();
  return db;
}

function makeKvStore(initial: Record<string, string> = {}) {
  const store: Record<string, string> = { ...initial };
  return {
    getItem: vi.fn().mockImplementation((key: string) => Promise.resolve(store[key] ?? null)),
    getItemBytes: vi.fn().mockResolvedValue(null),
    setItem: vi.fn().mockImplementation((key: string, val: string) => {
      store[key] = val;
      return Promise.resolve();
    }),
    removeItem: vi.fn().mockImplementation((key: string) => {
      delete store[key];
      return Promise.resolve();
    }),
    raw: store,
  };
}

const timeStub = async (_: string, action: () => unknown) => action();

const logger = { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn(), time: timeStub };

describe('stored-sessions effects', () => {
  let db: ExpoSQLiteDatabase;

  beforeEach(async () => {
    db = await createTestDb();
  });

  function bed(options: { keyValueStore?: ReturnType<typeof makeKvStore>; state?: Partial<RootState> }) {
    const testBed = createAddEffectTestBed({
      initialState: {
        settings: { isHydrated: true, preferredLanguage: 'en' },
        storedSessions: { openSessions: {}, activeSessionId: undefined },
        ...options.state,
      },
      services: {
        db,
        logger,
        keyValueStore: options.keyValueStore ?? makeKvStore(),
      },
    });
    applyStoredSessionsEffects(testBed.addEffect);
    return testBed;
  }

  describe('the active session in SQLite', () => {
    it('persists content without ever claiming the active flag', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [session.id]: session } } } as Partial<RootState>,
      });

      await testBed.dispatchHandled(putStoredSession(session));

      const [row] = await db.select().from(sessionsSchema).where(eq(sessionsSchema.id, session.id));
      expect(row).toBeDefined();
      expect(row!.active).toBe(false);
    });

    it('keeps exactly one row active as the workout changes', async () => {
      const first = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const second = Session.freeformSession(LocalDate.of(2026, 4, 11), undefined);
      const openSessions = { [first.id]: first, [second.id]: second };
      const testBed = bed({ state: { storedSessions: { openSessions } } as Partial<RootState> });

      await testBed.dispatchHandled(setActiveSessionId(first.id));
      await testBed.dispatchHandled(setActiveSessionId(second.id));

      const active = (await db.select().from(sessionsSchema)).filter((x) => x.active);
      expect(active.map((x) => x.id)).toEqual([second.id]);
    });

    it('inserts the row itself, so it does not depend on the content write landing first', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [session.id]: session } } } as Partial<RootState>,
      });

      await testBed.dispatchHandled(setActiveSessionId(session.id));

      const [row] = await db.select().from(sessionsSchema).where(eq(sessionsSchema.id, session.id));
      expect(row?.active).toBe(true);
    });

    it('clears the flag when the workout ends', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [session.id]: session } } } as Partial<RootState>,
      });
      await testBed.dispatchHandled(setActiveSessionId(session.id));

      await testBed.dispatchHandled(setActiveSessionId(undefined));

      expect((await db.select().from(sessionsSchema)).filter((x) => x.active)).toHaveLength(0);
    });

    it('announces that finished sessions changed once a deleted session is gone', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [session.id]: session } } } as Partial<RootState>,
      });
      await testBed.dispatchHandled(putStoredSession(session));

      await testBed.dispatchHandled(deleteStoredSession(session.id));

      expect(testBed.dispatchedActions.map((x) => x.type)).toContain(sessionsChanged.type);
      expect(readSession(db, session.id)).toBeUndefined();
    });

    it('restored backups land inactive', async () => {
      const restored = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({});

      await testBed.dispatchHandled(upsertStoredSessions([restored]));

      const [row] = await db.select().from(sessionsSchema).where(eq(sessionsSchema.id, restored.id));
      expect(row?.active).toBe(false);
    });

    it('restoring a backup leaves a workout in progress on this device alone', async () => {
      const active = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [active.id]: active } } } as Partial<RootState>,
      });
      await testBed.dispatchHandled(setActiveSessionId(active.id));

      await testBed.dispatchHandled(upsertStoredSessions([active]));

      const [row] = await db.select().from(sessionsSchema).where(eq(sessionsSchema.id, active.id));
      expect(row?.active).toBe(true);
    });
  });

  describe('queueing days for lifestats', () => {
    const queued = () => readSyncQueue(db).map(({ date }) => date.toString());
    const withOpen = (...sessions: Session[]) =>
      ({ storedSessions: { openSessions: Object.fromEntries(sessions.map((s) => [s.id, s])) } }) as Partial<RootState>;

    it('queues a workout when it is finished, not when it starts', async () => {
      const started = makeWorkoutOn('2026-10-11').withNothingCompleted();
      const testBed = bed({ state: withOpen(started) });
      await testBed.dispatchHandled(putStoredSession(started));
      await testBed.dispatchHandled(setActiveSessionId(started.id));
      expect(queued()).toEqual([]);
      testBed.expectNotDispatched(lifestatsDaysQueued);

      testBed.setState(withOpen(makeWorkoutOn('2026-10-11')));
      await testBed.dispatchHandled(putStoredSession(makeWorkoutOn('2026-10-11')));
      expect(queued()).toEqual([]);

      await testBed.dispatchHandled(setActiveSessionId(undefined));
      expect(queued()).toEqual(['2026-10-11']);
      expect(testBed.dispatchedActions.map((x) => x.type)).toContain(lifestatsDaysQueued.type);
    });

    it('queues the day of a finished workout that is resumed, which leaves it until finished again', async () => {
      const finished = makeWorkoutOn('2026-10-09');
      db.transaction((tx) => writeSession(tx, finished));
      const testBed = bed({ state: withOpen(finished) });

      await testBed.dispatchHandled(setActiveSessionId(finished.id));

      expect(queued()).toEqual(['2026-10-09']);
    });

    it('queues nothing when History closes unchanged, and both days when a History edit moves a workout', async () => {
      const finished = makeWorkoutOn('2026-10-09');
      db.transaction((tx) => writeSession(tx, finished));
      const testBed = bed({ state: withOpen(finished) });

      await testBed.dispatchHandled(sessionFinished(finished.id));
      expect(queued()).toEqual([]);

      const moved = finished.withUpdatedDate(LocalDate.of(2026, 10, 10));
      testBed.setState(withOpen(moved));
      await testBed.dispatchHandled(putStoredSession(moved));
      expect(queued()).toEqual(['2026-10-09', '2026-10-10']);
    });

    it('queues the day of a deleted workout', async () => {
      const finished = makeWorkoutOn('2026-10-09');
      db.transaction((tx) => writeSession(tx, finished));
      const testBed = bed({});

      await testBed.dispatchHandled(deleteStoredSession(finished.id));

      expect(queued()).toEqual(['2026-10-09']);
    });

    it('queues the old and new days of imported or restored workouts', async () => {
      db.transaction((tx) => writeSession(tx, makeWorkoutOn('2026-10-09')));
      const restoredEarlier = makeWorkoutOn('2026-10-09').withUpdatedDate(LocalDate.of(2026, 10, 1));
      const testBed = bed({});

      await testBed.dispatchHandled(upsertStoredSessions([restoredEarlier, makeWorkoutOn('2026-10-05')]));

      expect(queued()).toEqual(['2026-10-01', '2026-10-05', '2026-10-09']);
    });
  });

  it('persists restored exercises', async () => {
    const exercise = {
      name: 'Custom exercise',
      force: null,
      level: '',
      mechanic: null,
      equipment: null,
      muscles: [],
      instructions: '',
      category: '',
    };
    const testBed = bed({});

    await testBed.dispatchHandled(upsertExercises({ custom: exercise }));

    const [row] = await db.select().from(exercisesSchema).where(eq(exercisesSchema.id, 'custom'));
    expect(row?.payload).toEqual(exercise);
  });

  describe('sessionFinished', () => {
    it('clears the active pointer and marks stats dirty', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: {
          storedSessions: { openSessions: { [session.id]: session }, activeSessionId: session.id },
        } as Partial<RootState>,
      });

      await testBed.dispatchHandled(sessionFinished(session.id));

      expect(testBed.getDispatchedAction(setStatsIsDirty).payload).toBe(true);
      expect(testBed.getDispatchedAction(setActiveSessionId).payload).toBeUndefined();
    });

    it('writes the session, announces that finished sessions changed, and closes it', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const testBed = bed({
        state: { storedSessions: { openSessions: { [session.id]: session } } } as Partial<RootState>,
      });

      await testBed.dispatchHandled(sessionFinished(session.id));

      expect(testBed.dispatchedActions.map((x) => x.type)).toContain(sessionsChanged.type);
      expect(testBed.getDispatchedAction(closeSession).payload).toBe(session.id);
      expect(readSession(db, session.id)?.id).toBe(session.id);
    });

    it('leaves the active pointer alone when finishing some other session', async () => {
      const active = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const edited = Session.freeformSession(LocalDate.of(2026, 3, 1), undefined);
      const testBed = bed({
        state: {
          storedSessions: {
            openSessions: { [active.id]: active, [edited.id]: edited },
            activeSessionId: active.id,
          },
        } as Partial<RootState>,
      });

      await testBed.dispatchHandled(sessionFinished(edited.id));

      testBed.expectNotDispatched(setActiveSessionId);
    });
  });

  describe('migrating off CurrentSessionStateV1', () => {
    it('lifts a v3 in-progress workout into the table and marks it active', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const keyValueStore = makeKvStore({
        'CurrentSessionStateV1-Version': '3',
        CurrentSessionStateV1: toJsonString(session.toJSON()),
      });
      const testBed = bed({ keyValueStore });

      await testBed.dispatchHandled(initializeStoredSessionsStateSlice());

      expect(testBed.getDispatchedAction(putStoredSession).payload.id).toBe(session.id);
      expect(testBed.getDispatchedAction(setActiveSessionId).payload).toBe(session.id);
    });

    it('removes the legacy keys so it runs at most once', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const keyValueStore = makeKvStore({
        'CurrentSessionStateV1-Version': '3',
        CurrentSessionStateV1: toJsonString(session.toJSON()),
      });
      const testBed = bed({ keyValueStore });

      await testBed.dispatchHandled(initializeStoredSessionsStateSlice());

      expect(keyValueStore.raw.CurrentSessionStateV1).toBeUndefined();
      expect(keyValueStore.raw['CurrentSessionStateV1-Version']).toBeUndefined();
    });

    it('does nothing on a fresh install, where neither key exists', async () => {
      const keyValueStore = makeKvStore();
      const testBed = bed({ keyValueStore });

      await testBed.dispatchHandled(initializeStoredSessionsStateSlice());

      testBed.expectNotDispatched(putStoredSession);
      testBed.expectNotDispatched(setActiveSessionId);
    });

    it('restores only the active session from the table on a later launch', async () => {
      const session = Session.freeformSession(LocalDate.of(2026, 4, 10), undefined);
      const finished = Session.freeformSession(LocalDate.of(2026, 4, 9), undefined);
      db.transaction((tx) => {
        writeSession(tx, finished);
        setActiveSession(tx, session);
      });
      const testBed = bed({});

      await testBed.dispatchHandled(initializeStoredSessionsStateSlice());

      expect(testBed.getDispatchedAction(openSession).payload.id).toBe(session.id);
      expect(testBed.dispatchedActions.filter((x) => openSession.match(x))).toHaveLength(1);
      expect(testBed.getDispatchedAction(setActiveSessionId).payload).toBe(session.id);
    });
  });
});
