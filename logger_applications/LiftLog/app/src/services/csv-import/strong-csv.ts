import { cell, parseCsvTable, parseOptionalNumber } from '@/services/csv-import/csv-parse-utils';
import { CsvImportOptions, NormalizedExercise, NormalizedImportSession } from '@/services/csv-import/csv-to-sessions';
import { WeightUnit } from '@/models/weight';
import { LocalDateTime } from '@js-joda/core';

/** What a Strong `Set Order` cell says about its row. Warm-ups and unknown markers never reach a row. */
type StrongEntry = { type: 'set'; reps: number; weight: number; notes: string } | { type: 'rest'; seconds: number };

/** One working-set or rest-timer row from a Strong CSV export. */
export type StrongCsvRow = {
  /** `Workout #` when the export has it, else empty (older exports group by date alone). */
  workoutNumber: string;
  date: string;
  startedAt: LocalDateTime;
  workoutName: string;
  /** `Duration (sec)`, the workout's length as Strong recorded it; may be absent or nonsense. */
  durationSeconds: number | undefined;
  exercise: string;
  entry: StrongEntry;
};

export type ParseStrongCsvResult =
  | { ok: true; rows: StrongCsvRow[]; weightUnit: WeightUnit | undefined }
  | { ok: false; error: string };

const REQUIRED_HEADERS = ['Date', 'Exercise Name', 'Set Order', 'Reps'] as const;

/** `Weight (kg)` / `Weight (lbs)`; a bare `Weight` header leaves the unit to the app preference. */
function unitFromWeightHeader(fields: string[]): { field: string; unit: WeightUnit | undefined } | undefined {
  for (const field of fields) {
    const match = field.match(/^Weight\s*(?:\(\s*([^)]+)\s*\))?$/i);
    if (!match) {
      continue;
    }
    const token = match[1]?.trim().toLowerCase();
    const unit =
      token === 'kg' || token === 'kgs' ? 'kilograms' : token === 'lb' || token === 'lbs' ? 'pounds' : undefined;
    return { field, unit };
  }
  return undefined;
}

/** Locales that delimit with `;` may also write decimal commas (`62,5`). */
function parseStrongNumber(raw: string): number | undefined {
  return parseOptionalNumber(raw.replace(',', '.'));
}

function parseStrongDateTime(raw: string): LocalDateTime | undefined {
  try {
    return LocalDateTime.parse(raw.trim().replace(' ', 'T'));
  } catch {
    return undefined;
  }
}

function parseEntry(raw: Record<string, string>, weightField: string | undefined): StrongEntry | undefined {
  const setOrder = cell(raw, 'Set Order').toUpperCase();
  if (setOrder === 'REST TIMER') {
    const seconds = parseStrongNumber(cell(raw, 'Seconds'));
    return seconds !== undefined && seconds > 0 ? { type: 'rest', seconds } : undefined;
  }
  // Numbers are working sets; D (drop) and F (failure) count as working sets, W (warm-up) does not.
  if (!/^\d+$/.test(setOrder) && setOrder !== 'D' && setOrder !== 'F') {
    return undefined;
  }
  const reps = parseStrongNumber(cell(raw, 'Reps'));
  if (reps === undefined) {
    return undefined;
  }
  const weight = weightField ? parseStrongNumber(cell(raw, weightField)) : undefined;
  return { type: 'set', reps, weight: weight ?? 0, notes: cell(raw, 'Notes') };
}

/**
 * Parse a Strong workout CSV export: one row per set, plus `Rest Timer` rows that carry the rest in
 * `Seconds`. The delimiter (`;` or `,`) is read from the header line.
 */
export function parseStrongCsv(csvText: string): ParseStrongCsvResult {
  const headerLine = csvText.replace(/^﻿/, '').split(/\r?\n/, 1)[0] ?? '';
  const delimiter = headerLine.split(';').length > headerLine.split(',').length ? ';' : ',';
  const table = parseCsvTable(csvText, delimiter);
  if (!table.ok) {
    return table;
  }

  const fields = table.fields;
  const missing = REQUIRED_HEADERS.filter((h) => !fields.some((f) => f.toLowerCase() === h.toLowerCase()));
  if (missing.length > 0) {
    return { ok: false, error: `Missing CSV columns: ${missing.join(', ')}` };
  }

  const weightColumn = unitFromWeightHeader(fields);
  const rows: StrongCsvRow[] = [];
  for (const raw of table.data) {
    const date = cell(raw, 'Date');
    const startedAt = parseStrongDateTime(date);
    const exercise = cell(raw, 'Exercise Name');
    const entry = parseEntry(raw, weightColumn?.field);
    if (!startedAt || !exercise || !entry) {
      continue;
    }
    rows.push({
      workoutNumber: cell(raw, 'Workout #'),
      date,
      startedAt,
      workoutName: cell(raw, 'Workout Name'),
      durationSeconds: parseStrongNumber(cell(raw, 'Duration (sec)')),
      exercise,
      entry,
    });
  }

  if (!rows.some((r) => r.entry.type === 'set')) {
    return { ok: false, error: 'No workout rows found in CSV' };
  }

  return { ok: true, rows, weightUnit: weightColumn?.unit };
}

/**
 * Strong's `Duration (sec)` is wrong for workouts it never stopped (days long) or whose clock moved
 * (negative). Those get the median length of the plausible workouts in
 * the export this was written against instead.
 */
export const FALLBACK_WORKOUT_SECONDS = 52 * 60;
const MAX_PLAUSIBLE_WORKOUT_SECONDS = 6 * 60 * 60;

export function plausibleWorkoutSeconds(seconds: number | undefined): number {
  return seconds !== undefined && seconds >= 60 && seconds <= MAX_PLAUSIBLE_WORKOUT_SECONDS
    ? seconds
    : FALLBACK_WORKOUT_SECONDS;
}

/**
 * Group Strong rows into one session per workout. The key is `Workout #` plus the start time, so
 * importing the same export again yields the same session ids.
 */
export function strongRowsToNormalized(
  rows: StrongCsvRow[],
  weightUnit: WeightUnit | undefined,
  options: CsvImportOptions = {},
): NormalizedImportSession[] {
  const unit = weightUnit ?? options.defaultWeightUnit ?? 'kilograms';

  type ExerciseGroup = { sets: NormalizedExercise['sets']; notes: string[]; restSeconds?: number };
  type WorkoutGroup = { first: StrongCsvRow; exercises: Map<string, ExerciseGroup> };

  const workouts = new Map<string, WorkoutGroup>();
  for (const row of rows) {
    const key = `strong|${row.workoutNumber}|${row.date}`;
    let workout = workouts.get(key);
    if (!workout) {
      workout = { first: row, exercises: new Map() };
      workouts.set(key, workout);
    }
    let exercise = workout.exercises.get(row.exercise);
    if (!exercise) {
      exercise = { sets: [], notes: [] };
      workout.exercises.set(row.exercise, exercise);
    }
    if (row.entry.type === 'rest') {
      exercise.restSeconds ??= row.entry.seconds;
    } else {
      exercise.sets.push({ reps: row.entry.reps, weight: row.entry.weight, unit });
      if (row.entry.notes) {
        exercise.notes.push(row.entry.notes);
      }
    }
  }

  const chronological = [...workouts.entries()].sort(([, a], [, b]) => a.first.startedAt.compareTo(b.first.startedAt));
  const sessions: NormalizedImportSession[] = [];
  for (const [key, { first, exercises }] of chronological) {
    const normalizedExercises = [...exercises.entries()]
      .filter(([, e]) => e.sets.length > 0)
      .map(([name, e]): NormalizedExercise => ({
        name,
        notes: e.notes.length > 0 ? e.notes.join('\n') : undefined,
        sets: e.sets,
        restSeconds: e.restSeconds,
      }));
    if (normalizedExercises.length === 0) {
      continue;
    }
    sessions.push({
      contentDateKey: key,
      date: first.startedAt.toLocalDate(),
      startedAt: first.startedAt,
      durationSeconds: plausibleWorkoutSeconds(first.durationSeconds),
      sessionName: first.workoutName || options.sessionName || 'Imported',
      exercises: normalizedExercises,
    });
  }

  return sessions;
}
