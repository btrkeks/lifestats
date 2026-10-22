import {
  CalendarClaim,
  checkpointCalendarClaim,
  claimOccurrence,
  PendingCalendarSession,
  readClaimOwner,
  readCalendarRevision,
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

type Deps = {
  db: ExpoSQLiteDatabase;
  calendar: PhoneCalendar;
  logger: Pick<Logger, 'warn'>;
  enabled: () => boolean;
};

/**
 * Brings each pending workout's event in line with the workout, one workout at a time, and records what
 * it found. A workout that is blocked or waiting does not hold up the others. A calendar error is recorded
 * on its workout rather than thrown.
 */
export async function syncCalendarSessions(deps: Deps): Promise<CalendarRun> {
  let retry = false;
  for (const session of readPendingCalendarSessions(deps.db)) {
    if (!deps.enabled()) break;
    const outcome = await syncSession(deps, session);
    if (outcome === 'pending') {
      retry = true;
      continue;
    }
    if (outcome === 'no-permission') {
      return { noPermission: true, retry: false };
    }
    retry ||= !calendarOutcomes[outcome].settled;
  }
  return { noPermission: false, retry };
}

async function syncSession(
  { db, calendar, logger, enabled }: Deps,
  { sessionId, seq, target, claim }: PendingCalendarSession,
): Promise<CalendarOutcome | 'no-permission' | 'pending'> {
  let date: LocalDate | undefined = claim?.date;
  const record = (outcome: CalendarOutcome) => {
    db.transaction((tx) => recordCalendarOutcome(tx, { sessionId, seq, outcome, date }));
    return outcome;
  };
  const eligibleNow = (next: Times, on: LocalDate) => {
    const current = readFinishedSessions(db, [sessionId]).at(0);
    const span = current && sessionSpan(current);
    const times = span && workoutTimes(span);
    return (
      enabled() &&
      readCalendarRevision(db, sessionId) === seq &&
      current?.date.equals(on) &&
      times !== undefined &&
      sameTimes(times, next)
    );
  };
  const settleMove = async (owned: CalendarClaim, next: Times) => {
    if (!eligibleNow(next, owned.date)) return 'pending' as const;
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
      seq = readCalendarRevision(db, sessionId) ?? seq;
    }

    if (!enabled()) return 'pending';
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
      const inspected = await calendar.inspect(owned.identity);
      if (!eligibleNow(times, session.date)) return 'pending';
      if (inspected.kind !== 'found') {
        return inspected.kind === 'no-permission' ? inspected.kind : record(inspected.kind);
      }
      if (!sameTimes(inspected.times, owned.observed)) return record('changed');
      db.transaction((tx) => saveCalendarAttempt(tx, sessionId, times));
      return await settleMove(owned, times);
    }

    date = session.date;
    const window = dayWindow(session.date);
    const found = await calendar.occurrences(target, window);
    if (!eligibleNow(times, session.date)) return 'pending';
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
    if (!eligibleNow(times, session.date)) return 'pending';
    if (!db.transaction((tx) => claimOccurrence(tx, sessionId, fresh))) {
      return record('claimed');
    }
    return await settleMove(fresh, times);
  } catch (e) {
    logger.warn('Calendar sync failed', { sessionId, error: e instanceof Error ? e.message : String(e) });
    return record('error');
  }
}

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
