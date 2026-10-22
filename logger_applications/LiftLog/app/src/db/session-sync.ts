import { enrollCalendarSessions, requeueCalendarSessions } from '@/db/calendar-sync';
import { enqueueSyncDates } from '@/db/lifestats-sync';
import { readActiveSessionId, readWorkoutDates, readWorkoutSessionIds } from '@/db/sessions';
import { CalendarTarget } from '@/models/calendar-sync';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

/** Which syncs a session write queued work for. */
export type SyncQueued = { lifestats: boolean; calendar: boolean };

/**
 * Runs `write` over the given sessions inside its transaction, and queues what each sync needs from it, so
 * the queues and the sessions commit or roll back together.
 *
 * - Lifestats gets every day whose workout time the write may have changed: the dates where those sessions
 *   counted before it, and where they count after it.
 * - The calendar re-queues those of the sessions that are enrolled and were finished workouts before or
 *   after the write, so recording sets in a resumed workout queues nothing. With `enrollFinishedIn`, it
 *   also enrolls the workout in progress if this write finished it. Nothing else enrolls a session, so
 *   restores, imports and History edits never do, and turning sync on never reaches back into history.
 */
export function queueSessionSync(
  tx: ExpoSQLiteDatabase,
  sessionIds: string[],
  write: () => void,
  options: { enrollFinishedIn?: CalendarTarget } = {},
): SyncQueued {
  const datesBefore = readWorkoutDates(tx, sessionIds);
  const workoutsBefore = readWorkoutSessionIds(tx, sessionIds);
  const activeBefore = readActiveSessionId(tx);
  write();
  const workoutsAfter = readWorkoutSessionIds(tx, sessionIds);
  const lifestats = enqueueSyncDates(tx, [...datesBefore, ...readWorkoutDates(tx, sessionIds)]);

  const target = options.enrollFinishedIn;
  const enrolled =
    target && activeBefore !== undefined && sessionIds.includes(activeBefore) && workoutsAfter.has(activeBefore)
      ? activeBefore
      : undefined;
  if (target && enrolled) {
    enrollCalendarSessions(tx, [enrolled], target);
  }
  const requeued = requeueCalendarSessions(
    tx,
    sessionIds.filter((id) => id !== enrolled && (workoutsBefore.has(id) || workoutsAfter.has(id))),
  );
  return { lifestats, calendar: enrolled !== undefined || requeued };
}
