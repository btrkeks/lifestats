import { describe, expect, it, vi } from 'vitest';
import {
  columnWeightUnit,
  parseRepsText,
  parseWeightText,
  resolveBlur,
  resolveCheck,
  SetRow,
  setRows,
  suggestedReps,
  WeightColumn,
  weightCellText,
} from '@/components/presentation/workout/weighted/set-rows';
import { RecordedWeightedExercise } from '@/models/session-models';
import { Weight } from '@/models/weight';
import {
  emptyPotentialSet,
  filledPotentialSet,
  makeWeightedBlueprint,
  tick,
} from '@/models/session-models/__test__/helpers';

vi.mock('expo-localization', () => ({
  getLocales: () => [{ decimalSeparator: '.', digitGroupingSeparator: ',' }],
}));

const kg = (value: number) => new Weight(value, 'kilograms');
const external: WeightColumn = { unit: 'kilograms', resistance: 'external' };
const bodyweight: WeightColumn = { unit: 'kilograms', resistance: 'bodyweight' };

function uncheckedRow(suggested: number | undefined, weight = kg(70)): SetRow {
  return {
    index: 0,
    previous: undefined,
    weight,
    target: { min: 1, max: 20 },
    status: { kind: 'unchecked', suggestedReps: suggested },
  };
}

function checkedRow(reps: number, weight = kg(70)): SetRow {
  return { index: 0, previous: undefined, weight, target: { min: 1, max: 20 }, status: { kind: 'checked', reps } };
}

describe('suggestedReps', () => {
  it('uses last time verbatim, including below the target', () => {
    expect(suggestedReps(6, { min: 1, max: 20 })).toBe(6);
    expect(suggestedReps(4, { min: 6, max: 6 })).toBe(4);
    expect(suggestedReps(0, { min: 5, max: 5 })).toBe(0);
  });

  it('falls back to a point target only', () => {
    expect(suggestedReps(undefined, { min: 5, max: 5 })).toBe(5);
    expect(suggestedReps(undefined, { min: 1, max: 20 })).toBeUndefined();
    expect(suggestedReps(undefined, { min: 0, max: 0 })).toBeUndefined();
  });
});

describe('setRows', () => {
  it('pairs each set with last time at the same index and suggests from it', () => {
    const t = tick();
    const bp = makeWeightedBlueprint({ sets: 3, repsConfig: { type: 'range', min: 1, max: 20 } });
    const range = { min: 1, max: 20 };
    const today = new RecordedWeightedExercise(
      bp,
      [filledPotentialSet(8, t, kg(72.5), range), emptyPotentialSet(72.5, range), emptyPotentialSet(72.5, range)],
      undefined,
    );
    const lastTime = new RecordedWeightedExercise(
      bp,
      [filledPotentialSet(7, t, kg(70), range), filledPotentialSet(6, t, kg(70), range), emptyPotentialSet(70, range)],
      undefined,
    );

    const rows = setRows(today, lastTime);

    expect(rows.map((r) => r.status)).toEqual([
      { kind: 'checked', reps: 8 },
      { kind: 'unchecked', suggestedReps: 6 },
      { kind: 'unchecked', suggestedReps: undefined },
    ]);
    expect(rows.map((r) => r.previous && [r.previous.weight.value.toNumber(), r.previous.reps])).toEqual([
      [70, 7],
      [70, 6],
      undefined,
    ]);
  });
});

describe('cell text', () => {
  it('parses reps as non-negative whole numbers', () => {
    expect(parseRepsText('8')).toBe(8);
    expect(parseRepsText(' 12 ')).toBe(12);
    expect(parseRepsText('')).toBeUndefined();
    expect(parseRepsText('-1')).toBeUndefined();
    expect(parseRepsText('8.5')).toBeUndefined();
  });

  it('parses weight into the column unit, negative only when allowed', () => {
    expect(parseWeightText('72.5', 'kilograms', false)).toEqual(kg(72.5));
    expect(parseWeightText('-10', 'kilograms', false)).toBeUndefined();
    expect(parseWeightText('-10', 'kilograms', true)).toEqual(kg(-10));
    expect(parseWeightText('', 'kilograms', false)).toBeUndefined();
    expect(parseWeightText('abc', 'kilograms', false)).toBeUndefined();
  });

  it('renders zero as empty and other weights in the column unit', () => {
    expect(weightCellText(Weight.NIL, 'kilograms')).toBe('');
    expect(weightCellText(kg(70), 'kilograms')).toBe('70');
    expect(weightCellText(kg(72.5), 'kilograms')).toBe('72.5');
  });

  it('takes the column unit from the first set that has one', () => {
    const mixed = new RecordedWeightedExercise(
      makeWeightedBlueprint({ sets: 2 }),
      [emptyPotentialSet(Weight.NIL), emptyPotentialSet(new Weight(45, 'pounds'))],
      undefined,
    );
    const allNil = new RecordedWeightedExercise(
      makeWeightedBlueprint({ sets: 1 }),
      [emptyPotentialSet(Weight.NIL)],
      undefined,
    );
    expect(columnWeightUnit(mixed, 'kilograms')).toBe('pounds');
    expect(columnWeightUnit(allNil, 'kilograms')).toBe('kilograms');
  });
});

describe('resolveCheck', () => {
  it('commits the suggestion at the row weight when nothing was typed', () => {
    expect(resolveCheck(uncheckedRow(6), {}, external)).toEqual({
      kind: 'commit',
      change: { kind: 'log', setIndex: 0, weight: kg(70), reps: 6 },
    });
  });

  it('commits typed values over the model and the suggestion', () => {
    expect(resolveCheck(uncheckedRow(6), { repsText: '8', weightText: '72.5' }, external)).toEqual({
      kind: 'commit',
      change: { kind: 'log', setIndex: 0, weight: kg(72.5), reps: 8 },
    });
  });

  it('focuses reps when there is nothing to log', () => {
    expect(resolveCheck(uncheckedRow(undefined), {}, external)).toEqual({ kind: 'focus', field: 'reps' });
  });

  it('focuses reps when they were cleared or mistyped instead of falling back', () => {
    expect(resolveCheck(uncheckedRow(6), { repsText: '' }, external)).toEqual({ kind: 'focus', field: 'reps' });
    expect(resolveCheck(uncheckedRow(6), { repsText: '8x' }, external)).toEqual({ kind: 'focus', field: 'reps' });
  });

  it('focuses kg when the typed weight is invalid', () => {
    expect(resolveCheck(uncheckedRow(6), { weightText: 'abc' }, external)).toEqual({ kind: 'focus', field: 'weight' });
    expect(resolveCheck(uncheckedRow(6), { weightText: '' }, external)).toEqual({ kind: 'focus', field: 'weight' });
  });

  it('reads a cleared bodyweight cell as plain bodyweight', () => {
    expect(resolveCheck(uncheckedRow(6, kg(10)), { weightText: '' }, bodyweight)).toEqual({
      kind: 'commit',
      change: { kind: 'log', setIndex: 0, weight: kg(0), reps: 6 },
    });
  });

  it('unchecks a checked row and keeps its reps as a draft', () => {
    expect(resolveCheck(checkedRow(7), {}, external)).toEqual({
      kind: 'uncheck',
      change: { kind: 'unlog', setIndex: 0 },
      keepRepsText: '7',
    });
  });
});

describe('resolveBlur', () => {
  it('re-logs a checked row with the edited reps', () => {
    expect(resolveBlur(checkedRow(7), 'reps', { repsText: '9' }, external)).toEqual({
      kind: 'log',
      setIndex: 0,
      weight: kg(70),
      reps: 9,
    });
  });

  it('keeps reps on an unchecked row as a draft', () => {
    expect(resolveBlur(uncheckedRow(6), 'reps', { repsText: '9' }, external)).toBeUndefined();
  });

  it('commits a changed weight and ignores an unchanged, untouched or invalid one', () => {
    expect(resolveBlur(uncheckedRow(6), 'weight', { weightText: '75' }, external)).toEqual({
      kind: 'weight',
      setIndex: 0,
      weight: kg(75),
    });
    expect(resolveBlur(uncheckedRow(6), 'weight', { weightText: '70' }, external)).toBeUndefined();
    expect(resolveBlur(uncheckedRow(6), 'weight', {}, external)).toBeUndefined();
    expect(resolveBlur(uncheckedRow(6), 'weight', { weightText: '' }, external)).toBeUndefined();
  });
});
