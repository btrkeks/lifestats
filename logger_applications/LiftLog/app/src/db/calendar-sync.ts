import { calendarClaimsSchema, calendarSyncSessionsSchema } from '@/db/schema';
import {
  CalendarOutcome,
  calendarOutcomes,
  CalendarTarget,
  isCalendarOutcome,
  OccurrenceIdentity,
  Times,
} from '@/models/calendar-sync';
import { fromLocalDateJSON, toLocalDateJSON } from '@/models/storage/versions/libs';
import { LocalDate } from '@js-joda/core';
import { and, asc, count, eq, gt, inArray, isNotNull, sql } from 'drizzle-orm';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

const sessions = calendarSyncSessionsSchema;
const claims = calendarClaimsSchema;

// `instanceTime` of an event that does not repeat. NOT NULL keeps the unique index effective for them.
const SINGLE_EVENT = -1;

export type CalendarClaim = {
  identity: OccurrenceIdentity;
  /** The day it was matched on. */
  date: LocalDate;
  /** The occurrence's time before LiftLog first moved it. */
  original: Times;
  observed: Times;
  attempt: Times | undefined;
};

export type PendingCalendarSession = {
  sessionId: string;
  seq: number;
  target: CalendarTarget;
  claim: CalendarClaim | undefined;
};

export type CalendarAttention = { sessionId: string; date: LocalDate | undefined; outcome: CalendarOutcome };

const pending = gt(sessions.seq, sessions.doneSeq);

/** Enrolls workouts that were just finished, or queues them again if they already are. */
export function enrollCalendarSessions(tx: ExpoSQLiteDatabase, ids: string[], target: CalendarTarget): void {
  if (!ids.length) {
    return;
  }
  tx.insert(sessions)
    .values(ids.map((sessionId) => ({ sessionId, ...target })))
    .onConflictDoUpdate({ target: sessions.sessionId, set: { seq: sql`${sessions.seq} + 1` } })
    .run();
}

/** Queues the enrolled ones of `ids` again. Returns whether any of them is enrolled. */
export function requeueCalendarSessions(tx: ExpoSQLiteDatabase, ids: string[]): boolean {
  if (!ids.length) {
    return false;
  }
  return (
    tx
      .update(sessions)
      .set({ seq: sql`${sessions.seq} + 1` })
      .where(inArray(sessions.sessionId, ids))
      .run().changes > 0
  );
}

/** Try again: queues every workout whose last outcome needs attention. Returns whether there was one. */
export function requeueCalendarAttention(tx: ExpoSQLiteDatabase): boolean {
  const outcomes = attentionOutcomes();
  return (
    tx
      .update(sessions)
      .set({ seq: sql`${sessions.seq} + 1` })
      .where(inArray(sessions.outcome, outcomes))
      .run().changes > 0
  );
}

export function readPendingCalendarSessions(db: ExpoSQLiteDatabase): PendingCalendarSession[] {
  return db
    .select()
    .from(sessions)
    .leftJoin(claims, eq(claims.sessionId, sessions.sessionId))
    .where(pending)
    .orderBy(asc(sessions.sessionId))
    .all()
    .map(({ calendar_sync_session: row, calendar_claim: claim }) => ({
      sessionId: row.sessionId,
      seq: row.seq,
      target: { calendarId: row.calendarId, accountName: row.accountName, title: row.title },
      claim: claim ? claimFromRow(claim) : undefined,
    }));
}

export function readCalendarRevision(db: ExpoSQLiteDatabase, sessionId: string): number | undefined {
  return db.select({ seq: sessions.seq }).from(sessions).where(eq(sessions.sessionId, sessionId)).get()?.seq;
}

export function countPendingCalendarSessions(db: ExpoSQLiteDatabase): number {
  return db.select({ n: count() }).from(sessions).where(pending).get()?.n ?? 0;
}

export function readCalendarAttention(db: ExpoSQLiteDatabase): CalendarAttention[] {
  return db
    .select({ sessionId: sessions.sessionId, date: sessions.outcomeDate, outcome: sessions.outcome })
    .from(sessions)
    .where(and(isNotNull(sessions.outcome), inArray(sessions.outcome, attentionOutcomes())))
    .orderBy(asc(sessions.outcomeDate))
    .all()
    .flatMap(({ sessionId, date, outcome }) =>
      outcome ? [{ sessionId, date: date ? fromLocalDateJSON(date) : undefined, outcome }] : [],
    );
}

/** The workout that owns this occurrence, if any. */
export function readClaimOwner(db: ExpoSQLiteDatabase, identity: OccurrenceIdentity): string | undefined {
  return db
    .select({ sessionId: claims.sessionId })
    .from(claims)
    .where(
      and(
        eq(claims.calendarId, identity.calendarId),
        eq(claims.eventId, identity.eventId),
        eq(claims.instanceTime, identity.instanceTime ?? SINGLE_EVENT),
      ),
    )
    .get()?.sessionId;
}

/**
 * Makes the workout the owner of the occurrence, with the write it is about to make saved as its attempt.
 * Returns false when another workout owns it already.
 */
export function claimOccurrence(
  tx: ExpoSQLiteDatabase,
  sessionId: string,
  claim: { identity: OccurrenceIdentity; date: LocalDate; original: Times; attempt: Times },
): boolean {
  const { identity, original, attempt } = claim;
  return (
    tx
      .insert(claims)
      .values({
        sessionId,
        calendarId: identity.calendarId,
        accountName: identity.accountName,
        eventId: identity.eventId,
        instanceTime: identity.instanceTime ?? SINGLE_EVENT,
        syncId: identity.syncId,
        rrule: identity.rrule,
        title: identity.title,
        organizer: identity.organizer,
        sourceStartMs: identity.sourceStartMs,
        sourceDuration: identity.sourceDuration,
        sourceTimezone: identity.sourceTimezone,
        date: toLocalDateJSON(claim.date),
        originalStartMs: original.startMs,
        originalEndMs: original.endMs,
        observedStartMs: original.startMs,
        observedEndMs: original.endMs,
        attemptStartMs: attempt.startMs,
        attemptEndMs: attempt.endMs,
      })
      .onConflictDoNothing()
      .run().changes > 0
  );
}

/** Saves the span a write is about to send, before it is sent. */
export function saveCalendarAttempt(tx: ExpoSQLiteDatabase, sessionId: string, attempt: Times): void {
  tx.update(claims)
    .set({ attemptStartMs: attempt.startMs, attemptEndMs: attempt.endMs })
    .where(eq(claims.sessionId, sessionId))
    .run();
}

/**
 * Settles the saved attempt: the calendar shows `observed` now, either the attempt that reached it or the
 * time it still had.
 */
export function checkpointCalendarClaim(tx: ExpoSQLiteDatabase, sessionId: string, observed: Times): void {
  tx.update(claims)
    .set({
      observedStartMs: observed.startMs,
      observedEndMs: observed.endMs,
      attemptStartMs: null,
      attemptEndMs: null,
    })
    .where(eq(claims.sessionId, sessionId))
    .run();
}

/**
 * Records what an attempt that read `seq` found. A settled outcome marks that seq done, so an edit made
 * while the attempt ran keeps the workout pending.
 */
export function recordCalendarOutcome(
  tx: ExpoSQLiteDatabase,
  result: { sessionId: string; seq: number; outcome: CalendarOutcome; date: LocalDate | undefined },
): void {
  const { sessionId, seq, outcome, date } = result;
  tx.update(sessions)
    .set({
      outcome: sql`CASE WHEN ${sessions.seq} = ${seq} THEN ${outcome} ELSE ${sessions.outcome} END`,
      outcomeDate: sql`CASE WHEN ${sessions.seq} = ${seq} THEN ${date ? toLocalDateJSON(date) : null} ELSE ${sessions.outcomeDate} END`,
      ...(calendarOutcomes[outcome].settled ? { doneSeq: sql`max(${sessions.doneSeq}, ${seq})` } : {}),
    })
    .where(eq(sessions.sessionId, sessionId))
    .run();
}

function attentionOutcomes(): CalendarOutcome[] {
  return Object.keys(calendarOutcomes)
    .filter(isCalendarOutcome)
    .filter((outcome) => calendarOutcomes[outcome].attention);
}

function claimFromRow(row: typeof claims.$inferSelect): CalendarClaim {
  return {
    identity: {
      calendarId: row.calendarId,
      accountName: row.accountName,
      eventId: row.eventId,
      instanceTime: row.instanceTime === SINGLE_EVENT ? undefined : row.instanceTime,
      syncId: row.syncId ?? undefined,
      rrule: row.rrule ?? undefined,
      title: row.title,
      organizer: row.organizer ?? undefined,
      sourceStartMs: row.sourceStartMs ?? undefined,
      sourceDuration: row.sourceDuration ?? undefined,
      sourceTimezone: row.sourceTimezone ?? undefined,
    },
    date: fromLocalDateJSON(row.date),
    original: { startMs: row.originalStartMs, endMs: row.originalEndMs },
    observed: { startMs: row.observedStartMs, endMs: row.observedEndMs },
    attempt:
      row.attemptStartMs !== null && row.attemptEndMs !== null
        ? { startMs: row.attemptStartMs, endMs: row.attemptEndMs }
        : undefined,
  };
}
