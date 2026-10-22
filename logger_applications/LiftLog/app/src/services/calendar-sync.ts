import {
  CalendarClaim,
  checkpointCalendarClaim,
  claimOccurrence,
  PendingCalendarSession,
  readClaimOwner,
  readPendingCalendarSessions,
  recordCalendarOutcome,
  saveCalendarAttempt,
} from '@/db/calendar-sync';
import { readFinishedSessions } from '@/db/sessions';
import {
  CalendarOutcome,
  calendarOutcomes,
  dayWindow,
  matchOccurrence,
  PhoneCalendar,
  sameTimes,
  Times,
  workoutTimes,
} from '@/models/calendar-sync';
import { Logger } from '@/services/logger';
import { sessionSpan } from '@/services/lifestats-sync';
import { LocalDate } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

export type CalendarRun = {
  /** LiftLog lost calendar access; every pending workout waits for it. */
  noPermission: boolean;
  /** Some workout is waiting on something that may resolve by itself (a sync, a provider error). */
  retry: boolean;
};

type Deps = { db: ExpoSQLiteDatabase; calendar: PhoneCalendar; logger: Pick<Logger, 'warn'> };

/**
 * Brings each pending workout's event in line with the workout, one workout at a time, and records what
 * it found. A workout that is blocked or waiting does not hold up the others. A calendar error is recorded
 * on its workout rather than thrown.
 */
export async function syncCalendarSessions(deps: Deps): Promise<CalendarRun> {
  let retry = false;
  for (const session of readPendingCalendarSessions(deps.db)) {
    const outcome = await syncSession(deps, session);
    if (outcome === 'no-permission') {
      return { noPermission: true, retry: false };
    }
    retry ||= !calendarOutcomes[outcome].settled;
  }
  return { noPermission: false, retry };
}

async function syncSession(
  { db, calendar, logger }: Deps,
  { sessionId, seq, target, claim }: PendingCalendarSession,
): Promise<CalendarOutcome | 'no-permission'> {
  // The day the outcome is about: the claimed day once there is a claim, else the workout's day.
  let date: LocalDate | undefined = claim?.date;
  const record = (outcome: CalendarOutcome) => {
    db.transaction((tx) => recordCalendarOutcome(tx, { sessionId, seq, outcome, date }));
    return outcome;
  };
  const settleMove = async (owned: CalendarClaim, next: Times) => {
    const moved = await calendar.move(owned.identity, next, owned.observed);
    switch (moved.kind) {
      case 'moved':
      case 'unchanged':
        db.transaction((tx) => checkpointCalendarClaim(tx, sessionId, next));
        return record('adjusted');
      case 'changed':
        return record('changed');
      case 'no-permission':
        return moved.kind;
      default:
        return record(moved.kind);
    }
  };

  try {
    // A write from an earlier run may or may not have reached the calendar. Find out before anything else.
    let owned = claim;
    if (owned?.attempt) {
      const settled = await settleAttempt(db, calendar, sessionId, owned, owned.attempt);
      if (settled === 'no-permission') {
        return settled;
      }
      if (typeof settled === 'string') {
        return record(settled);
      }
      owned = { ...owned, observed: settled.observed, attempt: undefined };
    }

    const session = readFinishedSessions(db, [sessionId]).at(0);
    const span = session && sessionSpan(session);
    const times = span && workoutTimes(span);
    if (!session || !times) {
      return record('idle');
    }

    if (owned) {
      if (!session.date.equals(owned.date)) {
        return record('moved-date');
      }
      if (sameTimes(times, owned.observed)) {
        return record('adjusted');
      }
      db.transaction((tx) => saveCalendarAttempt(tx, sessionId, times));
      return await settleMove(owned, times);
    }

    date = session.date;
    const window = dayWindow(session.date);
    const found = await calendar.occurrences(target, window);
    if (found.kind !== 'found') {
      return found.kind === 'no-permission' ? found.kind : record(found.kind);
    }
    const matched = matchOccurrence(found.occurrences, target.title, window);
    if (matched.kind !== 'one') {
      return record(matched.kind);
    }
    const { identity, times: original, problem } = matched.occurrence;
    if (readClaimOwner(db, identity) !== undefined) {
      return record('claimed');
    }
    if (problem) {
      return record(problem);
    }
    const fresh = { identity, date: session.date, original, observed: original, attempt: times };
    if (!db.transaction((tx) => claimOccurrence(tx, sessionId, fresh))) {
      return record('claimed');
    }
    return await settleMove(fresh, times);
  } catch (e) {
    logger.warn('Calendar sync failed', { sessionId, error: e instanceof Error ? e.message : String(e) });
    return record('error');
  }
}

/**
 * Checkpoints a saved attempt by what the calendar shows now: the attempt (it landed), or the time
 * LiftLog last saw (it never did). Anything else was changed outside LiftLog, and is left alone.
 */
async function settleAttempt(
  db: ExpoSQLiteDatabase,
  calendar: PhoneCalendar,
  sessionId: string,
  claim: CalendarClaim,
  attempt: Times,
): Promise<{ observed: Times } | 'no-permission' | CalendarOutcome> {
  const seen = await calendar.inspect(claim.identity);
  if (seen.kind === 'no-permission') {
    return seen.kind;
  }
  if (seen.kind !== 'found') {
    return seen.kind;
  }
  const now = [attempt, claim.observed].find((times) => sameTimes(times, seen.times));
  if (!now) {
    return 'changed';
  }
  db.transaction((tx) => checkpointCalendarClaim(tx, sessionId, now));
  return { observed: now };
}
