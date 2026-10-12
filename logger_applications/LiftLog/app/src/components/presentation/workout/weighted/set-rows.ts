import { Resistance } from '@/models/blueprint-models';
import {
  CompletedWeightedSet,
  CompletedWeightedSets,
  RecordedWeightedExercise,
  WeightedChange,
} from '@/models/session-models';
import { Weight, WeightUnit } from '@/models/weight';
import { localeFormatBigNumber, localeParseBigNumber } from '@/utils/locale-bignumber';

export type RowStatus = { kind: 'checked'; reps: number } | { kind: 'unchecked'; suggestedReps: number | undefined };

export interface SetRow {
  index: number;
  /** The last time this index was completed, in any finished session. */
  previous: CompletedWeightedSet | undefined;
  /** Planned while unchecked, performed once checked. */
  weight: Weight;
  status: RowStatus;
}

/**
 * Text typed but not yet committed. `undefined` is untouched (the cell shows the model or the
 * suggestion), `''` is deliberately cleared, anything else is typed.
 */
export interface RowDraft {
  weightText?: string | undefined;
  repsText?: string | undefined;
}

/** How the kg column reads and parses for one exercise. */
export interface WeightColumn {
  unit: WeightUnit;
  resistance: Resistance;
}

export type CheckOutcome =
  | { kind: 'commit'; change: Extract<WeightedChange, { kind: 'log' }> }
  | { kind: 'uncheck'; change: Extract<WeightedChange, { kind: 'unlog' }>; keepRepsText: string }
  | { kind: 'focus'; field: 'weight' | 'reps' };

/** History only suggests: the exercise's own weight is what the row shows and logs. */
export function setRows(exercise: RecordedWeightedExercise, previous: CompletedWeightedSets | undefined): SetRow[] {
  return exercise.potentialSets.map((ps, index) => {
    const previousSet = previous?.get(index);
    return {
      index,
      previous: previousSet,
      weight: ps.weight,
      status: ps.set
        ? { kind: 'checked', reps: ps.set.repsCompleted }
        : { kind: 'unchecked', suggestedReps: previousSet?.reps },
    };
  });
}

/** Typed values parse into this unit; untouched weights are never re-parsed, so stored units do not drift. */
export function columnWeightUnit(exercise: RecordedWeightedExercise, preferred: WeightUnit): WeightUnit {
  return exercise.potentialSets.find((s) => s.weight.unit !== 'nil')?.weight.unit ?? preferred;
}

/** Zero renders empty so the cell's "0" placeholder shows instead. */
export function weightCellText(weight: Weight, unit: WeightUnit): string {
  if (weight.value.isZero()) {
    return '';
  }
  const converted = weight.convertTo(unit);
  return localeFormatBigNumber(converted.unit === weight.unit ? converted.value : converted.value.decimalPlaces(1));
}

/** `''` or garbage parse to undefined. Negative only when allowed (assisted bodyweight). */
export function parseWeightText(text: string, unit: WeightUnit, allowNegative: boolean): Weight | undefined {
  const trimmed = text.trim();
  if (trimmed === '') {
    return undefined;
  }
  const value = localeParseBigNumber(trimmed);
  if (!value.isFinite() || (value.isNegative() && !allowNegative)) {
    return undefined;
  }
  return new Weight(value, unit);
}

/** A non-negative whole number, else undefined. */
export function parseRepsText(text: string): number | undefined {
  const trimmed = text.trim();
  return /^\d+$/.test(trimmed) ? Number.parseInt(trimmed, 10) : undefined;
}

type DraftWeight = { kind: 'untouched' } | { kind: 'valid'; weight: Weight } | { kind: 'invalid' };

function draftWeight(draft: RowDraft, column: WeightColumn): DraftWeight {
  if (draft.weightText === undefined) {
    return { kind: 'untouched' };
  }
  // A cleared bodyweight cell is plain bodyweight; a cleared external cell is a missing number.
  if (draft.weightText.trim() === '' && column.resistance === 'bodyweight') {
    return { kind: 'valid', weight: new Weight(0, column.unit) };
  }
  const weight = parseWeightText(draft.weightText, column.unit, column.resistance === 'bodyweight');
  return weight ? { kind: 'valid', weight } : { kind: 'invalid' };
}

/**
 * The ✓ press. Unchecking keeps the reps as a draft so ✓ again restores them. Checking takes typed
 * values, else the model weight and the suggested reps, and never silently falls back from a field
 * the user cleared or mistyped: that field gets focus instead.
 */
export function resolveCheck(row: SetRow, draft: RowDraft, column: WeightColumn): CheckOutcome {
  if (row.status.kind === 'checked') {
    return {
      kind: 'uncheck',
      change: { kind: 'unlog', setIndex: row.index },
      keepRepsText: draft.repsText ?? String(row.status.reps),
    };
  }
  const weight = draftWeight(draft, column);
  if (weight.kind === 'invalid') {
    return { kind: 'focus', field: 'weight' };
  }
  const reps = draft.repsText === undefined ? row.status.suggestedReps : parseRepsText(draft.repsText);
  if (reps === undefined) {
    return { kind: 'focus', field: 'reps' };
  }
  return {
    kind: 'commit',
    change: {
      kind: 'log',
      setIndex: row.index,
      weight: weight.kind === 'valid' ? weight.weight : row.weight,
      reps,
    },
  };
}

/**
 * A cell losing focus. kg commits on any row when it parses and differs. Reps commit only on a
 * checked row, keeping its completion time; on an unchecked row they stay a draft until ✓.
 */
export function resolveBlur(
  row: SetRow,
  field: 'weight' | 'reps',
  draft: RowDraft,
  column: WeightColumn,
): WeightedChange | undefined {
  if (field === 'weight') {
    const weight = draftWeight(draft, column);
    return weight.kind === 'valid' && !row.weight.equals(weight.weight, true)
      ? { kind: 'weight', setIndex: row.index, weight: weight.weight }
      : undefined;
  }
  if (row.status.kind !== 'checked' || draft.repsText === undefined) {
    return undefined;
  }
  const reps = parseRepsText(draft.repsText);
  return reps !== undefined && reps !== row.status.reps
    ? { kind: 'log', setIndex: row.index, weight: row.weight, reps }
    : undefined;
}
