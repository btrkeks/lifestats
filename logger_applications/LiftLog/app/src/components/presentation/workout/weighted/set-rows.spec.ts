import { describe, expect, it, vi } from 'vitest';
import {
  columnWeightUnit,
  parseRepsText,
  parseWeightText,
  resolveBlur,
  resolveCheck,
  resolveSubmit,
  rowCue,
  SetRow,
  setRows,
  WeightColumn,
  weightCellText,
} from '@/components/presentation/workout/weighted/set-rows';
import { CompletedWeightedSet, RecordedWeightedExercise } from '@/models/session-models';
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
    status: { kind: 'unchecked', suggestedReps: suggested },
  };
}

function checkedRow(reps: number, weight = kg(70)): SetRow {
  return { index: 0, previous: undefined, weight, status: { kind: 'checked', reps } };
}

describe('setRows', () => {
  const completed = (weight: number, reps: number): CompletedWeightedSet => ({
    weight: kg(weight),
    reps,
    context: { resistance: 'external' },
  });
  const history = new Map([
    [0, completed(52, 9)],
    [1, completed(55, 7)],
    [2, completed(62, 5)],
  ]);

  it('pairs each set with the last time its index was completed and suggests those reps', () => {
    const today = new RecordedWeightedExercise(
      makeWeightedBlueprint({ sets: 3 }),
      [emptyPotentialSet(52), emptyPotentialSet(55), emptyPotentialSet(62)],
      undefined,
    );

    const rows = setRows(today, history);

    expect(rows.map((r) => r.status)).toEqual([
      { kind: 'unchecked', suggestedReps: 9 },
      { kind: 'unchecked', suggestedReps: 7 },
      { kind: 'unchecked', suggestedReps: 5 },
    ]);
    expect(rows.map((r) => r.previous && [r.previous.weight.value.toNumber(), r.previous.reps])).toEqual([
      [52, 9],
      [55, 7],
      [62, 5],
    ]);
  });

  it('shows what was performed over history, and keeps the exercise weight over the previous one', () => {
    const t = tick();
    const today = new RecordedWeightedExercise(
      makeWeightedBlueprint({ sets: 2 }),
      [filledPotentialSet(8, t, kg(57.5)), emptyPotentialSet(60), emptyPotentialSet(60)],
      undefined,
    );

    const rows = setRows(today, history);

    expect(rows.map((r) => r.status)).toEqual([
      { kind: 'checked', reps: 8 },
      { kind: 'unchecked', suggestedReps: 7 },
      { kind: 'unchecked', suggestedReps: 5 },
    ]);
    expect(rows.map((r) => r.weight.value.toNumber())).toEqual([57.5, 60, 60]);
  });

  it('suggests nothing without history', () => {
    const today = new RecordedWeightedExercise(makeWeightedBlueprint({ sets: 1 }), [emptyPotentialSet(0)], undefined);

    expect(setRows(today, undefined)).toEqual([
      { index: 0, previous: undefined, weight: kg(0), status: { kind: 'unchecked', suggestedReps: undefined } },
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

describe('resolveSubmit', () => {
  const second = (row: SetRow): SetRow => ({ ...row, index: 1 });
  const notLast = false;
  const last = true;

  it('leaves kg to its blur and moves to the same row reps', () => {
    expect(resolveSubmit(second(uncheckedRow(6)), 'weight', { weightText: '75' }, external, last)).toEqual({
      log: undefined,
      next: { kind: 'focus', setIndex: 1, field: 'reps' },
    });
  });

  it('logs the set from reps and moves to the next row kg', () => {
    expect(resolveSubmit(second(uncheckedRow(6)), 'reps', { repsText: '8' }, external, notLast)).toEqual({
      log: { kind: 'log', setIndex: 1, weight: kg(70), reps: 8 },
      next: { kind: 'focus', setIndex: 2, field: 'weight' },
    });
  });

  it('moves to the next row reps when the exercise has no kg column', () => {
    const none: WeightColumn = { unit: 'kilograms', resistance: 'none' };

    expect(resolveSubmit(uncheckedRow(6, kg(0)), 'reps', {}, none, notLast)).toEqual({
      log: { kind: 'log', setIndex: 0, weight: kg(0), reps: 6 },
      next: { kind: 'focus', setIndex: 1, field: 'reps' },
    });
  });

  it('logs the last row and closes the keyboard instead of moving to the next exercise', () => {
    expect(resolveSubmit(second(uncheckedRow(6)), 'reps', {}, external, last)).toEqual({
      log: { kind: 'log', setIndex: 1, weight: kg(70), reps: 6 },
      next: { kind: 'dismiss' },
    });
  });

  it('stays on the row and focuses the field ✓ would refuse', () => {
    expect(resolveSubmit(uncheckedRow(6), 'reps', { weightText: 'abc', repsText: '8' }, external, notLast)).toEqual({
      log: undefined,
      next: { kind: 'focus', setIndex: 0, field: 'weight' },
    });
    expect(resolveSubmit(uncheckedRow(undefined), 'reps', {}, external, last)).toEqual({
      log: undefined,
      next: { kind: 'focus', setIndex: 0, field: 'reps' },
    });
  });

  it('leaves edited reps on a checked row to their blur, then advances', () => {
    expect(resolveSubmit(checkedRow(7), 'reps', { repsText: '9' }, external, notLast)).toEqual({
      log: undefined,
      next: { kind: 'focus', setIndex: 1, field: 'weight' },
    });
    expect(resolveSubmit(checkedRow(7), 'reps', {}, external, last)).toEqual({
      log: undefined,
      next: { kind: 'dismiss' },
    });
  });
});

describe('rowCue', () => {
  const lastTime: CompletedWeightedSet = { weight: kg(60), reps: 10, context: { resistance: 'external' } };
  const aiming = (weight: Weight): SetRow => ({
    index: 0,
    previous: lastTime,
    weight,
    status: { kind: 'unchecked', suggestedReps: 10 },
  });
  const live = { resistance: 'external' } as const;

  it('aims at the weight being typed, and says nothing while it does not parse', () => {
    expect(rowCue(aiming(kg(60)), {}, external, live)).toEqual({ kind: 'target', reps: 11 });
    expect(rowCue(aiming(kg(60)), { weightText: '62.5' }, external, live)).toEqual({ kind: 'target', reps: 9 });
    expect(rowCue(aiming(kg(60)), { weightText: 'x' }, external, live)).toEqual({ kind: 'hidden' });
  });

  it('reads a cleared bodyweight cell as plain bodyweight', () => {
    const context = { resistance: 'bodyweight', bodyweight: kg(80) } as const;
    const row: SetRow = { ...aiming(kg(10)), previous: { weight: kg(0), reps: 10, context } };

    expect(rowCue(row, { weightText: '' }, bodyweight, context)).toEqual({ kind: 'target', reps: 11 });
  });

  it('judges a checked row on what it logged, whatever is being typed', () => {
    const done: SetRow = { ...aiming(kg(62.5)), status: { kind: 'checked', reps: 9 } };

    expect(rowCue(done, { weightText: '50' }, external, live)).toEqual({ kind: 'better' });
  });

  it('leaves ✓ logging the previous reps, not the target', () => {
    expect(rowCue(aiming(kg(62.5)), {}, external, live)).toEqual({ kind: 'target', reps: 9 });
    expect(resolveCheck(aiming(kg(62.5)), {}, external)).toEqual({
      kind: 'commit',
      change: { kind: 'log', setIndex: 0, weight: kg(62.5), reps: 10 },
    });
  });
});
