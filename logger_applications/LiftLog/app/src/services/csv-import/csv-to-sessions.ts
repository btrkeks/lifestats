import { ProgramBlueprint, SessionBlueprint, WeightedExerciseBlueprint } from '@/models/blueprint-models';
import { PotentialSet, RecordedSet, RecordedWeightedExercise, Session } from '@/models/session-models';
import { Weight, WeightUnit } from '@/models/weight';
import { uuidFromName } from '@/utils/uuid';
import { Duration, LocalDate, LocalDateTime, LocalTime, OffsetDateTime, ZoneId, ZoneOffset } from '@js-joda/core';

export type CsvImportOptions = {
  /** Fallback unit when a row's Weight Unit is missing/unknown. */
  defaultWeightUnit?: WeightUnit;
  /** Session blueprint name when the format does not supply one. */
  sessionName?: string;
};

/**
 * Namespace for UUID v5 session IDs from third-party CSV imports.
 * Same grouping key → same session id → re-import skips instead of duplicating.
 */
export const CSV_IMPORT_SESSION_NAMESPACE = 'a7f3c8e2-4b1d-5e9a-8c2f-1d0e9b7a6c5f';

export type NormalizedSet = { reps: number; weight: number; unit: WeightUnit };
export type NormalizedExercise = {
  name: string;
  notes?: string;
  sets: NormalizedSet[];
  restSeconds?: number;
};
export type NormalizedImportSession = {
  /** Grouping key (FitNotes date, StrongLifts `date|w{Workout}`, or Strong `strong|{Workout #}|{Date}`). */
  contentDateKey: string;
  date: LocalDate;
  /** Wall-clock start in the device's zone, when the file records one. */
  startedAt?: LocalDateTime;
  /** How long the workout took, when the file records it. Set completions are spread across it. */
  durationSeconds?: number;
  sessionName: string;
  bodyweight?: Weight;
  exercises: NormalizedExercise[];
};

/**
 * Stable id from the session grouping key only (not sets or notes).
 * First import wins; delete the session and import again to refresh.
 */
export function sessionIdFromCsvContent(dateKey: string): string {
  return uuidFromName(`csv-import-v1\n${dateKey}`, CSV_IMPORT_SESSION_NAMESPACE);
}

/** Build Session domain objects from format-normalized import sessions. */
export function sessionsFromNormalized(sessions: NormalizedImportSession[]): Session[] {
  const result: Session[] = [];

  for (const normalized of sessions) {
    if (normalized.exercises.length === 0) {
      continue;
    }

    const recordedExercises: RecordedWeightedExercise[] = [];
    const start = sessionStart(normalized);
    let setOrdinal = 0;

    for (const exercise of normalized.exercises) {
      if (exercise.sets.length === 0) {
        continue;
      }
      const blueprint = exerciseBlueprintFor(exercise);

      const potentialSets = exercise.sets.map((set) => {
        // +1s per set keeps history order deterministic; CSV exports carry no per-set timestamps. A
        // known duration re-times them below.
        const completionDateTime = start.plusSeconds(setOrdinal);
        setOrdinal += 1;
        return new PotentialSet(new RecordedSet(set.reps, completionDateTime), new Weight(set.weight, set.unit));
      });

      recordedExercises.push(new RecordedWeightedExercise(blueprint, potentialSets, exercise.notes));
    }

    if (recordedExercises.length === 0) {
      continue;
    }

    const sessionBlueprint = new SessionBlueprint(
      normalized.sessionName,
      recordedExercises.map((e) => e.blueprint),
      '',
    );

    const session = new Session(
      sessionIdFromCsvContent(normalized.contentDateKey),
      sessionBlueprint,
      recordedExercises,
      normalized.date,
      normalized.bodyweight,
      undefined,
    );
    result.push(
      normalized.durationSeconds === undefined
        ? session
        : spreadSetCompletions(session, start, start.plusSeconds(normalized.durationSeconds)),
    );
  }

  return result;
}

/**
 * Re-times the completed weighted sets, in order, evenly from `start` to `end`, so the session's
 * first and last set mark when the workout began and ended. A lone set completes at `end`.
 */
export function spreadSetCompletions(session: Session, start: OffsetDateTime, end: OffsetDateTime): Session {
  const total = session.recordedExercises
    .filter((e) => e instanceof RecordedWeightedExercise)
    .reduce((count, e) => count + e.potentialSets.filter((s) => s.set).length, 0);
  const seconds = Duration.between(start, end).seconds();
  let ordinal = 0;
  const completionAt = () => {
    const offset = total === 1 ? seconds : Math.round((seconds * ordinal) / (total - 1));
    ordinal += 1;
    return start.plusSeconds(offset);
  };
  return session.with({
    recordedExercises: session.recordedExercises.map((exercise) =>
      exercise instanceof RecordedWeightedExercise
        ? exercise.withAllSets((ps) =>
            ps.set ? ps.with({ set: ps.set.with({ completionDateTime: completionAt() }) }) : ps,
          )
        : exercise,
    ),
  });
}

function sessionStart(session: NormalizedImportSession): OffsetDateTime {
  return session.startedAt
    ? session.startedAt.atZone(ZoneId.SYSTEM).toOffsetDateTime()
    : session.date.atTime(LocalTime.NOON).atOffset(ZoneOffset.UTC);
}

/**
 * `existing` with its sets re-timed to span what the freshly `imported` copy of it spans, kept on the
 * date `existing` was moved to. Only completion times change; sets, weights, reps and notes stay.
 */
export function retimedLikeImport(existing: Session, imported: Session): Session {
  const completions = imported.recordedExercises
    .flatMap((e) => (e instanceof RecordedWeightedExercise ? e.potentialSets : []))
    .flatMap((s) => (s.set ? [s.set.completionDateTime] : []))
    .sort((a, b) => a.compareTo(b));
  const [first, last] = [completions.at(0), completions.at(-1)];
  if (!first || !last) {
    return existing;
  }
  const movedDays = existing.date.toEpochDay() - imported.date.toEpochDay();
  return spreadSetCompletions(existing, first.plusDays(movedDays), last.plusDays(movedDays));
}

/** Shared by history and plan so both describe the exercise alike. */
function exerciseBlueprintFor(exercise: NormalizedExercise): WeightedExerciseBlueprint {
  const rest =
    exercise.restSeconds === undefined
      ? undefined
      : {
          minRest: Duration.ofSeconds(exercise.restSeconds),
          maxRest: Duration.ofSeconds(exercise.restSeconds),
          failureRest: Duration.ofSeconds(exercise.restSeconds),
        };
  return WeightedExerciseBlueprint.of({ name: exercise.name, sets: exercise.sets.length, restBetweenSets: rest });
}

/** Stable plan id from the plan name, so importing again replaces the plan instead of adding another. */
export function programIdFromCsvImport(programName: string): string {
  return uuidFromName(`csv-import-plan-v1\n${programName}`, CSV_IMPORT_SESSION_NAMESPACE);
}

/**
 * A plan with one workout per session name that occurs at least twice, each copying the most recent
 * session of that name. Undefined when no name repeats.
 */
export function programFromNormalized(
  programName: string,
  sessions: NormalizedImportSession[],
): ProgramBlueprint | undefined {
  const byName = new Map<string, NormalizedImportSession[]>();
  for (const session of sessions) {
    if (session.exercises.some((e) => e.sets.length > 0)) {
      const occurrences = byName.get(session.sessionName) ?? [];
      occurrences.push(session);
      byName.set(session.sessionName, occurrences);
    }
  }
  // Ties go to the later one in file order.
  const latestOfEachName = [...byName.entries()]
    .filter(([, occurrences]) => occurrences.length >= 2)
    .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))
    .map(([, occurrences]) =>
      occurrences.reduce((latest, s) => (sessionStart(s).isBefore(sessionStart(latest)) ? latest : s)),
    );
  if (latestOfEachName.length === 0) {
    return undefined;
  }

  const lastEdited = latestOfEachName.map((s) => s.date).reduce((a, b) => (b.isAfter(a) ? b : a));
  return new ProgramBlueprint(
    programName,
    latestOfEachName.map(
      (s) =>
        new SessionBlueprint(s.sessionName, s.exercises.filter((e) => e.sets.length > 0).map(exerciseBlueprintFor), ''),
    ),
    lastEdited,
  );
}
