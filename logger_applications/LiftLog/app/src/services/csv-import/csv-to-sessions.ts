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
    let setOrdinal = 0;

    for (const exercise of normalized.exercises) {
      if (exercise.sets.length === 0) {
        continue;
      }
      const blueprint = exerciseBlueprintFor(exercise);
      const start = sessionStart(normalized);

      const potentialSets = exercise.sets.map((set) => {
        // +1s per set keeps history order deterministic; CSV exports carry no per-set timestamps.
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

    result.push(
      new Session(
        sessionIdFromCsvContent(normalized.contentDateKey),
        sessionBlueprint,
        recordedExercises,
        normalized.date,
        normalized.bodyweight,
        undefined,
      ),
    );
  }

  return result;
}

function sessionStart(session: NormalizedImportSession): OffsetDateTime {
  return session.startedAt
    ? session.startedAt.atZone(ZoneId.SYSTEM).toOffsetDateTime()
    : session.date.atTime(LocalTime.NOON).atOffset(ZoneOffset.UTC);
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
