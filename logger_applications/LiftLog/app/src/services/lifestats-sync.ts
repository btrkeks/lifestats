import { ackSyncDates, readSyncQueue } from '@/db/lifestats-sync';
import { readFinishedSessionsOn } from '@/db/sessions';
import { Session } from '@/models/session-models';
import { DateTimeFormatter, Duration, LocalDate, OffsetDateTime } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

/** `PUT {baseUrl}/integrations/liftlog/days` replaces the workout time of exactly the days it names. */
export type SyncDay = { date: string; workouts: { id: string; start: string; end: string }[] };

export type WorkoutSpan = { start: OffsetDateTime; end: OffsetDateTime };

export type PushOutcome =
  | { kind: 'delivered'; days: number }
  | { kind: 'failed'; days: number; retryable: boolean; error: string };

const DAYS_PER_REQUEST = 100;
const REQUEST_TIMEOUT_MS = 15_000;
// RFC 3339 needs the seconds, which OffsetDateTime.toString() drops when they are zero.
const RFC_3339 = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm:ssxxx");

/** The HTTPS origin (and optional path) the user typed, without trailing slashes; undefined if unusable. */
export function syncBaseUrl(raw: string | undefined): string | undefined {
  const url = raw?.trim().replace(/\/+$/, '');
  return url && /^https:\/\/[^\s/?#]+(\/[^\s?#]*)?$/i.test(url) ? url : undefined;
}

/**
 * From the earliest completed set to the latest, where a timed cardio set ends when its timer does.
 * Undefined when nothing was completed, so the session adds no time to its day.
 */
export function sessionSpan(session: Session): WorkoutSpan | undefined {
  const moments = session.recordedExercises.flatMap((exercise) =>
    exercise.type === 'RecordedWeightedExercise'
      ? exercise.potentialSets.flatMap((s) => (s.set ? [s.set.completionDateTime] : []))
      : exercise.sets.flatMap((s) =>
          s.completionDateTime ? [s.completionDateTime, s.completionDateTime.plus(s.duration ?? Duration.ZERO)] : [],
        ),
  );
  const [first, ...rest] = moments;
  if (!first) {
    return undefined;
  }
  return rest.reduce(
    (span, t) => ({ start: t.isBefore(span.start) ? t : span.start, end: t.isAfter(span.end) ? t : span.end }),
    { start: first, end: first },
  );
}

/** Everything lifestats should hold for `date`: the span of each finished workout on it, earliest first. */
export function syncDay(date: LocalDate, finishedSessions: Session[]): SyncDay {
  const workouts = finishedSessions
    .filter((session) => session.date.equals(date))
    .flatMap((session) => {
      const span = sessionSpan(session);
      return span ? [{ id: session.id, ...span }] : [];
    })
    .sort((a, b) => a.start.compareTo(b.start))
    .map(({ id, start, end }) => ({ id, start: start.format(RFC_3339), end: end.format(RFC_3339) }));
  return { date: date.toString(), workouts };
}

/**
 * Sends every queued day, a hundred per request, and acknowledges each request's days once lifestats
 * accepts them. Stops at the first failure, leaving that request's days and the rest queued. Never throws.
 */
export async function pushQueuedDays({
  db,
  baseUrl,
  fetch,
}: {
  db: ExpoSQLiteDatabase;
  baseUrl: string;
  fetch: typeof globalThis.fetch;
}): Promise<PushOutcome> {
  const queued = readSyncQueue(db);
  let days = 0;
  for (let i = 0; i < queued.length; i += DAYS_PER_REQUEST) {
    const chunk = queued.slice(i, i + DAYS_PER_REQUEST);
    // Read just before sending, so the request carries the newest content of each day it names.
    const sessions = readFinishedSessionsOn(
      db,
      chunk.map((d) => d.date),
    );
    const failure = await put(fetch, `${baseUrl}/integrations/liftlog/days`, {
      days: chunk.map(({ date }) => syncDay(date, sessions)),
    });
    if (failure) {
      return { kind: 'failed', days, ...failure };
    }
    db.transaction((tx) => ackSyncDates(tx, chunk));
    days += chunk.length;
  }
  return { kind: 'delivered', days };
}

async function put(
  fetch: typeof globalThis.fetch,
  url: string,
  body: { days: SyncDay[] },
): Promise<{ retryable: boolean; error: string } | undefined> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(url, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
      signal: controller.signal,
    });
    const text = await response.text();
    if (!response.ok) {
      return {
        retryable: response.status >= 500 || response.status === 408 || response.status === 429,
        error: `HTTP ${response.status}: ${errorMessage(text)}`,
      };
    }
    // A 200 from anything but lifestats (a captive portal, a wrong URL) must not acknowledge the days.
    return isSyncResponse(text) ? undefined : { retryable: false, error: 'Not a lifestats response' };
  } catch (e) {
    return {
      retryable: true,
      error: controller.signal.aborted ? 'Timed out' : e instanceof Error ? e.message : String(e),
    };
  } finally {
    clearTimeout(timeout);
  }
}

function parseJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

function isSyncResponse(text: string): boolean {
  const json = parseJson(text);
  return typeof json === 'object' && json !== null && 'statistic_id' in json && typeof json.statistic_id === 'number';
}

function errorMessage(text: string): string {
  const json = parseJson(text);
  return typeof json === 'object' && json !== null && 'error' in json && typeof json.error === 'string'
    ? json.error
    : text.slice(0, 200);
}
