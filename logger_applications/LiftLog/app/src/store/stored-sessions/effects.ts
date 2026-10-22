import { AddEffectFn, AppDispatch } from '@/store/store';
import {
  closeSession,
  deleteExercise,
  deleteStoredSession,
  initializeStoredSessionsStateSlice,
  openSession,
  putStoredSession,
  restoreExercise,
  selectActiveSession,
  selectSession,
  sessionFinished,
  sessionsChanged,
  setActiveSessionId,
  setBuiltInExercises,
  setExercises,
  setHiddenBuiltInIds,
  setIsHydrated,
  updateExercise,
  updateStoredSession,
  upsertExercises,
  upsertStoredSessions,
} from './index';
import { fetchUpcomingSessions } from '@/store/program';
import { lifestatsDaysQueued } from '@/store/lifestats-sync';
import { calendarSessionsQueued, selectCalendarSyncTarget } from '@/store/calendar-sync';
import { setStatsIsDirty } from '@/store/stats';
import { setPreferredLanguage } from '@/store/settings';
import { exercisesSchema } from '@/db/schema';
import { deleteSession, readActiveSession, readActiveSessionId, setActiveSession, writeSession } from '@/db/sessions';
import { queueSessionSync, SyncQueued } from '@/db/session-sync';
import { eq, sql } from 'drizzle-orm';
import { toRecord } from '@/utils/reduce';
import { fromExerciseDescriptorJSON, toExerciseDescriptorJSON } from '@/models/exercise-models';
import { loadBuiltInExercises } from '@/services/exercise-catalog';
import { migrateLegacyCurrentSession } from '@/store/stored-sessions/legacy-current-session';
import { forgottenWorkoutAction } from '@/store/stored-sessions/forgotten-workout';
import { appForegroundChanged, showSnackbar } from '@/store/app';
import { formatDate } from '@/utils/format-date';
import { OffsetDateTime } from '@js-joda/core';

function wakeSyncs(dispatch: AppDispatch, queued: SyncQueued) {
  if (queued.lifestats) {
    dispatch(lifestatsDaysQueued());
  }
  if (queued.calendar) {
    dispatch(calendarSessionsQueued());
  }
}

// Built-ins the user deleted, so they stay hidden across restarts and locale switches.
const hiddenBuiltInExerciseIdsStorageKey = 'HiddenBuiltInExerciseIdList';
export function applyStoredSessionsEffects(addEffect: AddEffectFn) {
  // Dispatched AFTER settings, so we can safely access settings
  addEffect(
    initializeStoredSessionsStateSlice,
    async (_, { cancelActiveListeners, getState, dispatch, extra: { keyValueStore, db, logger } }) => {
      cancelActiveListeners();
      if (!getState().settings.isHydrated) {
        throw new Error('Settings must be hydrated before stored sessions');
      }
      await logger.time('initializeStoredSessions', async () => {
        const activeSession = readActiveSession(db);
        // Only when there is one: dispatching `undefined` would clear every flag in the table, and a
        // kill between that write and the migration below would lose the workout in progress.
        if (activeSession) {
          dispatch(openSession(activeSession));
          dispatch(setActiveSessionId(activeSession.id));
        }
      });

      await migrateLegacyCurrentSession(dispatch, getState, keyValueStore, logger);

      const savedExercises = (await db.select().from(exercisesSchema)).reduce(
        toRecord(
          (x) => x.id,
          (x) => fromExerciseDescriptorJSON(x.payload),
        ),
        {},
      );
      dispatch(setExercises(savedExercises));

      const builtInExercises = await loadBuiltInExercises(getState().settings.preferredLanguage);
      dispatch(setBuiltInExercises(builtInExercises));

      const hiddenBuiltInIds = JSON.parse(
        (await keyValueStore.getItem(hiddenBuiltInExerciseIdsStorageKey)) ?? '[]',
      ) as string[];
      dispatch(setHiddenBuiltInIds(hiddenBuiltInIds));

      dispatch(setIsHydrated(true));
      dispatch(fetchUpcomingSessions());
    },
  );

  // Re-resolve the built-in catalog when the language changes (startup load is handled above).
  addEffect(setPreferredLanguage, async (action, { getState, dispatch }) => {
    if (!getState().storedSessions.isHydrated) {
      return;
    }
    dispatch(setBuiltInExercises(await loadBuiltInExercises(action.payload)));
  });

  addEffect(sessionFinished, (action, { getState, dispatch, extra: { db } }) => {
    const state = getState();
    const workout = selectSession(state, action.payload);
    if (!workout) {
      return;
    }

    if (state.storedSessions.activeSessionId === workout.id) {
      dispatch(setActiveSessionId(undefined));
    }
    // Written here rather than left to the persist effect, so whoever re-reads on the revision bump sees it.
    // It queues no sync: the flag flip above queued a finished workout, and History edits were queued as
    // they were made, so closing History without a change sends nothing.
    db.transaction((tx) => writeSession(tx, workout));
    dispatch(sessionsChanged());
    dispatch(closeSession(workout.id));
    dispatch(setStatsIsDirty(true));
    dispatch(fetchUpcomingSessions());
  });

  // A workout left open would otherwise count as hours of training. Both actions carry `true` when the
  // stored workout has loaded or the app is back in the foreground.
  addEffect([setIsHydrated, appForegroundChanged], (action, { getState, dispatch, extra: { tolgee } }) => {
    const state = getState();
    const session = selectActiveSession(state);
    if (!action.payload || !state.storedSessions.isHydrated || !session) {
      return;
    }
    const decision = forgottenWorkoutAction(session, OffsetDateTime.now());
    if (decision === 'finish') {
      // Not through `useFinishWorkout`: an automatic finish leaves the plan alone.
      dispatch(sessionFinished(session.id));
      const day = formatDate(state.settings.preferredLanguage, session.date, { weekday: 'short' });
      dispatch(
        showSnackbar({
          text: tolgee.t('workout.auto_finished.message', { name: session.blueprint.name, day }),
          duration: 8000,
        }),
      );
    } else if (decision === 'discard') {
      dispatch(deleteStoredSession(session.id));
      dispatch(fetchUpcomingSessions());
    }
  });

  addEffect(deleteStoredSession, async (action, { dispatch, extra: { logger, db } }) => {
    const queued = await logger.time('deleteStoredSession', async () =>
      db.transaction((tx) => queueSessionSync(tx, [action.payload], () => deleteSession(tx, action.payload))),
    );
    dispatch(sessionsChanged());
    wakeSyncs(dispatch, queued);
  });

  // Content only. The `active` flag has a single writer below, so a recorded set never touches it.
  addEffect([putStoredSession, updateStoredSession], async (action, { dispatch, getState, extra: { db, logger } }) => {
    const sessionId = putStoredSession.match(action)
      ? action.payload.id
      : updateStoredSession.match(action)
        ? action.payload.sessionId
        : undefined;
    // Read at write time rather than from stateAfterReduce, so a slow write still stores the newest
    // content if a later edit overtakes it.
    const session = sessionId === undefined ? undefined : selectSession(getState(), sessionId);
    if (!session) {
      return;
    }
    // Queues a History edit for both syncs. The workout in progress is not finished, so it queues nothing.
    const queued = await logger.time('persistStoredSession', async () =>
      db.transaction((tx) => queueSessionSync(tx, [session.id], () => writeSession(tx, session))),
    );
    wakeSyncs(dispatch, queued);
  });

  // The only writer of `active`. It writes the session's content too, so it does not depend on the effect
  // above having written the row first - the two are dispatched together and race.
  addEffect(setActiveSessionId, async (action, { getState, dispatch, extra: { db, logger, phoneCalendar } }) => {
    const queued = await logger.time('setActiveSessionId', async () => {
      const state = getState();
      const session = action.payload === undefined ? undefined : selectSession(state, action.payload);
      // Finishing a workout puts it on its day and resuming one takes it off, in the same commit as the flag.
      // Finishing is also the only write that enrolls a workout in calendar sync.
      const enrollFinishedIn = phoneCalendar.supported ? selectCalendarSyncTarget(state) : undefined;
      return db.transaction((tx) => {
        const affected = [readActiveSessionId(tx), session?.id].filter((id) => id !== undefined);
        return queueSessionSync(tx, affected, () => setActiveSession(tx, session), { enrollFinishedIn });
      });
    });
    dispatch(sessionsChanged());
    wakeSyncs(dispatch, queued);
  });

  addEffect(upsertStoredSessions, async (action, { cancelActiveListeners, dispatch, extra: { db, logger } }) => {
    cancelActiveListeners();
    // Restored sessions are never active - a backup should not resume someone else's workout, and an
    // in-progress workout on this device keeps its flag because writing content never touches it.
    const queued = await logger.time('upsertStoredSessions', async () =>
      db.transaction((tx) =>
        queueSessionSync(
          tx,
          action.payload.map((session) => session.id),
          () => action.payload.forEach((session) => writeSession(tx, session)),
        ),
      ),
    );
    dispatch(sessionsChanged());
    wakeSyncs(dispatch, queued);
  });

  addEffect(deleteExercise, async (action, { stateAfterReduce, extra: { db, keyValueStore } }) => {
    if (stateAfterReduce.storedSessions.builtInExercises[action.payload]) {
      // Built-ins are tombstoned rather than removed; their override row (if any) is kept for undo.
      await keyValueStore.setItem(
        hiddenBuiltInExerciseIdsStorageKey,
        JSON.stringify(stateAfterReduce.storedSessions.hiddenBuiltInIds),
      );
    } else {
      await db.delete(exercisesSchema).where(eq(exercisesSchema.id, action.payload));
    }
  });

  addEffect(restoreExercise, async (_, { stateAfterReduce, extra: { keyValueStore } }) => {
    await keyValueStore.setItem(
      hiddenBuiltInExerciseIdsStorageKey,
      JSON.stringify(stateAfterReduce.storedSessions.hiddenBuiltInIds),
    );
  });

  addEffect(updateExercise, async (action, { extra: { db } }) => {
    await db
      .insert(exercisesSchema)
      .values({
        id: action.payload.id,
        payload: toExerciseDescriptorJSON(action.payload.exercise),
      })
      .onConflictDoUpdate({
        target: exercisesSchema.id,
        set: {
          payload: sql.raw(`excluded.${exercisesSchema.payload.name}`),
        },
      });
  });

  addEffect(upsertExercises, async (action, { extra: { db } }) => {
    const exercises = Object.entries(action.payload).map(([id, exercise]) => ({
      id,
      payload: toExerciseDescriptorJSON(exercise),
    }));
    if (!exercises.length) {
      return;
    }
    await db
      .insert(exercisesSchema)
      .values(exercises)
      .onConflictDoUpdate({
        target: exercisesSchema.id,
        set: {
          payload: sql.raw(`excluded.${exercisesSchema.payload.name}`),
        },
      });
  });

  addEffect(setExercises, async (action, { stateAfterReduce, extra: { db } }) => {
    if (!stateAfterReduce.storedSessions.isHydrated) {
      return;
    }
    const rows = Object.entries(action.payload).map(([id, exercise]) => ({
      id,
      payload: toExerciseDescriptorJSON(exercise),
    }));
    db.transaction((tx) => {
      tx.delete(exercisesSchema).run();
      if (rows.length) {
        tx.insert(exercisesSchema).values(rows).run();
      }
    });
  });
}
