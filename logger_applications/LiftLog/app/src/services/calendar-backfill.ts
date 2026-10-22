import { claimOccurrence, enrollNewCalendarSession, readCalendarRevision, readClaimOwner } from '@/db/calendar-sync';
import { readFinishedSessions, readSessionsBetween, readWorkoutDates } from '@/db/sessions';
import {
  BackfillApplyResult,
  BackfillApplyRow,
  BackfillError,
  BackfillPreview,
  BackfillRequest,
  BackfillRow,
  BackfillWorkout,
  isBackfillPreviewCurrent,
} from '@/models/calendar-backfill';
import {
  dayWindow,
  identityKey,
  matchOccurrence,
  PhoneCalendar,
  sameTimes,
  workoutTimes,
} from '@/models/calendar-sync';
import { Session } from '@/models/session-models';
import { sessionSpan } from '@/services/lifestats-sync';
import { LocalDate } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

type Deps = { db: ExpoSQLiteDatabase; calendar: PhoneCalendar };

export function defaultBackfillRange(db: ExpoSQLiteDatabase) {
  const dates = readWorkoutDates(db).sort((a, b) => a.compareTo(b));
  return { from: dates.at(0) ?? LocalDate.now(), to: dates.at(-1) ?? LocalDate.now() };
}

function projection(session: Session): BackfillWorkout | undefined {
  const span = sessionSpan(session);
  const desired = span && workoutTimes(span);
  if (!span || !desired) return undefined;
  return {
    sessionId: session.id,
    date: session.date,
    name: session.blueprint.name,
    desired,
    completedSpan: { startMs: span.start.toInstant().toEpochMilli(), endMs: span.end.toInstant().toEpochMilli() },
  };
}

export async function previewCalendarBackfill(
  { db, calendar }: Deps,
  request: BackfillRequest,
): Promise<BackfillPreview> {
  if (request.range.from.isAfter(request.range.to)) throw new Error('Invalid calendar backfill range');
  const workouts = readSessionsBetween(db, request.range.from, request.range.to)
    .flatMap((session) => {
      const row = projection(session);
      return row ? [row] : [];
    })
    .sort((a, b) => a.date.compareTo(b.date) || a.sessionId.localeCompare(b.sessionId));
  const days = new Map<string, Awaited<ReturnType<PhoneCalendar['occurrences']>>>();
  const rows: BackfillRow[] = [];
  let accessProblem: BackfillError | undefined;
  try {
    accessProblem = !calendar.supported
      ? 'unsupported'
      : (await calendar.access()) !== 'granted'
        ? 'no-permission'
        : undefined;
  } catch {
    accessProblem = 'provider-error';
  }
  for (const workout of workouts) {
    if (readCalendarRevision(db, workout.sessionId) !== undefined) {
      rows.push({ ...workout, status: 'already-linked' });
      continue;
    }
    try {
      if (accessProblem) {
        rows.push({ ...workout, status: 'error', reason: accessProblem });
        continue;
      }
      const day = workout.date.toString();
      let found = days.get(day);
      if (!found) {
        found = await calendar.occurrences(request.target, dayWindow(workout.date));
        days.set(day, found);
      }
      if (found.kind !== 'found') {
        rows.push({ ...workout, status: 'error', reason: found.kind });
        continue;
      }
      const match = matchOccurrence(found.occurrences, request.target.title, dayWindow(workout.date));
      if (match.kind !== 'one') {
        rows.push({ ...workout, status: match.kind === 'no-match' ? 'missing' : 'ambiguous' });
        continue;
      }
      const candidate = { identity: match.occurrence.identity, current: match.occurrence.times };
      const status =
        readClaimOwner(db, candidate.identity) !== undefined
          ? 'claimed'
          : match.occurrence.problem
            ? 'unsupported'
            : 'ready';
      rows.push({ ...workout, candidate, status });
    } catch {
      rows.push({ ...workout, status: 'error', reason: 'provider-error' });
    }
  }
  const counts = new Map<string, number>();
  for (const row of rows) {
    if (row.status === 'ready') {
      const key = identityKey(row.candidate.identity);
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  return {
    target: { ...request.target },
    range: { ...request.range },
    rows: rows.map((row) =>
      row.status === 'ready' && (counts.get(identityKey(row.candidate.identity)) ?? 0) > 1
        ? { ...row, status: 'conflicting-workouts' }
        : row,
    ),
  };
}

class BackfillClaimConflict extends Error {}

export async function applyCalendarBackfill(
  { db, calendar }: Deps,
  {
    preview,
    selectedIds,
    getRequest,
  }: {
    preview: BackfillPreview;
    selectedIds: string[];
    getRequest: () => BackfillRequest | undefined;
  },
): Promise<BackfillApplyResult> {
  const ids = [...new Set(selectedIds)];
  const finish = (rows: BackfillApplyRow[]): BackfillApplyResult => ({
    rows,
    queued: rows.filter((row) => row.status === 'queued').length,
  });
  if (!ids.length) return finish([]);
  if (!isBackfillPreviewCurrent(preview, getRequest())) {
    return finish(ids.map((sessionId) => ({ sessionId, status: 'stale' })));
  }
  try {
    if (!calendar.supported) {
      return finish(ids.map((sessionId) => ({ sessionId, status: 'error', reason: 'unsupported' })));
    }
    if ((await calendar.access()) !== 'granted') {
      return finish(ids.map((sessionId) => ({ sessionId, status: 'error', reason: 'no-permission' })));
    }
  } catch {
    return finish(ids.map((sessionId) => ({ sessionId, status: 'error', reason: 'provider-error' })));
  }
  const results = ids.map((sessionId): BackfillApplyRow => {
    const row = preview.rows.find((entry) => entry.sessionId === sessionId);
    if (!row || row.status !== 'ready') return { sessionId, status: 'not-ready' };
    try {
      return db.transaction((tx): BackfillApplyRow => {
        if (!isBackfillPreviewCurrent(preview, getRequest())) return { sessionId, status: 'stale' };
        if (readCalendarRevision(tx, sessionId) !== undefined) return { sessionId, status: 'already-linked' };
        const session = readFinishedSessions(tx, [sessionId]).at(0);
        const current = session && projection(session);
        if (
          !current ||
          !current.date.equals(row.date) ||
          current.name !== row.name ||
          !sameTimes(current.completedSpan, row.completedSpan) ||
          !sameTimes(current.desired, row.desired)
        ) {
          return { sessionId, status: 'stale' };
        }
        if (readClaimOwner(tx, row.candidate.identity) !== undefined) return { sessionId, status: 'claimed' };
        if (!enrollNewCalendarSession(tx, sessionId, preview.target)) return { sessionId, status: 'already-linked' };
        if (
          !claimOccurrence(tx, sessionId, {
            identity: row.candidate.identity,
            date: row.date,
            original: row.candidate.current,
            attempt: row.desired,
          })
        )
          throw new BackfillClaimConflict();
        return { sessionId, status: 'queued' };
      });
    } catch (error) {
      return error instanceof BackfillClaimConflict
        ? { sessionId, status: 'claimed' }
        : { sessionId, status: 'error', reason: 'provider-error' };
    }
  });
  return finish(results);
}
