import { describe, it, expect } from 'vitest';
import { Weight } from '@/models/weight';
import {
  CompletedWeightedSet,
  CompletedWeightedSets,
  PotentialSet,
  RecordedSet,
  RecordedWeightedExercise,
} from '@/models/session-models/recorded-weighted-exercise';
import {
  emptyPotentialSet,
  filledPotentialSet,
  makeLegacyWeightedBlueprint,
  makeRecordedExercise,
  makeWeightedBlueprint,
  targets,
  tick,
} from '@/models/session-models/__test__/helpers';
import { IndexOutOfBoundsError } from '@/utils/index-out-of-bounds';

// ─── withNothingCompleted ─────────────────────────────────────────────────────

describe('RecordedWeightedExercise.withNothingCompleted', () => {
  it('clears all recorded sets and notes', () => {
    const bp = makeWeightedBlueprint();
    const t = tick();
    const exercise = new RecordedWeightedExercise(
      bp,
      [filledPotentialSet(10, t), filledPotentialSet(10, t.plusSeconds(30))],
      'some note',
    );

    const result = exercise.withNothingCompleted();

    expect(result.potentialSets.every((s) => s.set === undefined)).toBe(true);
    expect(result.notes).toBeUndefined();
  });

  it('preserves weights after clearing', () => {
    const bp = makeWeightedBlueprint();
    const t = tick();
    const weight = new Weight(80, 'kilograms');
    const exercise = new RecordedWeightedExercise(bp, [filledPotentialSet(10, t, weight)], undefined);

    const result = exercise.withNothingCompleted();
    expect(result.potentialSets[0]!.weight).toEqual(weight);
  });
});

// ─── getSet ───────────────────────────────────────────────────────────────────

describe('RecordedWeightedExercise.getSet', () => {
  it('returns the set at the index', () => {
    const set = emptyPotentialSet(60);
    const exercise = new RecordedWeightedExercise(makeWeightedBlueprint(), [set], undefined);
    expect(exercise.getSet(0)).toBe(set);
  });

  it('throws when the index is out of bounds', () => {
    const exercise = new RecordedWeightedExercise(makeWeightedBlueprint(), [], undefined);
    expect(() => exercise.getSet(0)).toThrow(IndexOutOfBoundsError);
  });
});

// ─── withRepCount ─────────────────────────────────────────────────────────────

describe('RecordedWeightedExercise.withRepCount', () => {
  it('records a set with the given reps', () => {
    const t = tick();
    const exercise = new RecordedWeightedExercise(makeWeightedBlueprint(), [emptyPotentialSet(60)], undefined);

    const result = exercise.withRepCount(0, 8, t);

    expect(result.potentialSets[0]!.set).toEqual(RecordedSet.of({ repsCompleted: 8, completionDateTime: t }));
  });

  it('clears the set when reps is undefined', () => {
    const t = tick();
    const exercise = new RecordedWeightedExercise(makeWeightedBlueprint(), [filledPotentialSet(10, t)], undefined);

    const result = exercise.withRepCount(0, undefined, t);

    expect(result.potentialSets[0]!.set).toBeUndefined();
  });
});

// ─── equals ───────────────────────────────────────────────────────────────────

describe('RecordedWeightedExercise.equals', () => {
  const t = tick();
  const base = () => new RecordedWeightedExercise(makeWeightedBlueprint(), [filledPotentialSet(10, t)], 'note');

  it('is false against undefined', () => {
    expect(base().equals(undefined)).toBe(false);
  });

  it('is true against itself', () => {
    const e = base();
    expect(e.equals(e)).toBe(true);
  });

  it('is true for a structurally equal exercise', () => {
    expect(base().equals(base())).toBe(true);
  });

  it('is false when notes differ', () => {
    expect(base().equals(base().with({ notes: 'other' }))).toBe(false);
  });

  it('is false when the number of sets differs', () => {
    expect(base().equals(base().with({ potentialSets: [] }))).toBe(false);
  });
});

// ─── derived properties ───────────────────────────────────────────────────────

describe('RecordedWeightedExercise derived values', () => {
  it('currentSetIndex points at the first uncompleted set', () => {
    const t = tick();
    const exercise = new RecordedWeightedExercise(
      makeWeightedBlueprint(),
      [filledPotentialSet(10, t), emptyPotentialSet(60)],
      undefined,
    );
    expect(exercise.currentSetIndex).toBe(1);
  });

  it('totalWeightLifted sums weight times reps completed', () => {
    const t = tick();
    const exercise = new RecordedWeightedExercise(
      makeWeightedBlueprint(),
      [
        filledPotentialSet(5, t, new Weight(100, 'kilograms')),
        filledPotentialSet(3, t, new Weight(50, 'kilograms')),
        emptyPotentialSet(50),
      ],
      undefined,
    );
    expect(exercise.totalWeightLifted).toEqual(new Weight(650, 'kilograms'));
  });

  it('duration spans the earliest to latest completion time', () => {
    const first = tick();
    const last = first.plusSeconds(90);
    const exercise = new RecordedWeightedExercise(
      makeWeightedBlueprint(),
      [filledPotentialSet(10, last), filledPotentialSet(10, first)],
      undefined,
    );
    expect(exercise.earliestTime).toEqual(first);
    expect(exercise.latestTime).toEqual(last);
    expect(exercise.duration!.seconds()).toBe(90);
  });

  it('duration is undefined when nothing is recorded', () => {
    const exercise = RecordedWeightedExercise.empty(makeWeightedBlueprint(), 'kilograms');
    expect(exercise.duration).toBeUndefined();
  });
});

// ─── bodyweight fold-in ────────────────────────────────────────────────────────

describe('RecordedWeightedExercise bodyweight fold-in', () => {
  const bodyweight = new Weight(80, 'kilograms');

  function bodyweightExercise(addedKg: number, reps = 5) {
    return new RecordedWeightedExercise(
      makeWeightedBlueprint({ name: 'Pull Up', resistance: 'bodyweight' }),
      [filledPotentialSet(reps, tick(), new Weight(addedKg, 'kilograms'))],
      undefined,
    );
  }

  it('effectiveWeight adds the bodyweight to the stored added load', () => {
    const ex = bodyweightExercise(10);
    expect(ex.effectiveWeight(ex.potentialSets[0]!, bodyweight)).toEqual(new Weight(90, 'kilograms'));
  });

  it('effectiveWeight subtracts assistance from the bodyweight', () => {
    const ex = bodyweightExercise(-20);
    expect(ex.effectiveWeight(ex.potentialSets[0]!, bodyweight)).toEqual(new Weight(60, 'kilograms'));
  });

  it('effectiveWeight falls back to only the added weight when bodyweight is unknown', () => {
    const ex = bodyweightExercise(10);
    expect(ex.effectiveWeight(ex.potentialSets[0]!, undefined)).toEqual(new Weight(10, 'kilograms'));
  });

  it('effectiveWeight ignores the bodyweight for a plain weighted exercise', () => {
    const ex = new RecordedWeightedExercise(
      makeWeightedBlueprint({ name: 'Row' }),
      [filledPotentialSet(5, tick(), new Weight(10, 'kilograms'))],
      undefined,
    );
    expect(ex.effectiveWeight(ex.potentialSets[0]!, bodyweight)).toEqual(new Weight(10, 'kilograms'));
  });

  it('totalWeightLiftedWith folds the bodyweight into every set', () => {
    const ex = bodyweightExercise(10, 5); // (80 + 10) × 5
    expect(ex.totalWeightLiftedWith(bodyweight)).toEqual(new Weight(450, 'kilograms'));
  });

  it('maxWeightWith reports the heaviest effective load', () => {
    const ex = bodyweightExercise(10);
    expect(ex.maxWeightWith(bodyweight)).toEqual(new Weight(90, 'kilograms'));
  });

  it('the zero-arg totalWeightLifted counts only the added weight', () => {
    const ex = bodyweightExercise(10, 5); // 10 × 5, no bodyweight folded in
    expect(ex.totalWeightLifted).toEqual(new Weight(50, 'kilograms'));
  });
});

// ─── JSON round-trips ─────────────────────────────────────────────────────────

describe('RecordedWeightedExercise JSON', () => {
  it('round-trips through toJSON/fromJSON', () => {
    const t = tick();
    const exercise = new RecordedWeightedExercise(
      makeWeightedBlueprint(),
      [filledPotentialSet(10, t), emptyPotentialSet(60)],
      'a note',
    );

    const restored = RecordedWeightedExercise.fromJSON(exercise.toJSON());

    expect(restored.equals(exercise)).toBe(true);
  });

  it('RecordedSet round-trips and compares by value', () => {
    const t = tick();
    const set = RecordedSet.of({ repsCompleted: 7, completionDateTime: t });
    const restored = RecordedSet.fromJSON(set.toJSON());
    expect(restored.equals(set)).toBe(true);
    expect(set.equals(undefined)).toBe(false);
    expect(set.equals(set)).toBe(true);
    expect(set.equals(RecordedSet.of({ repsCompleted: 8, completionDateTime: t }))).toBe(false);
  });

  it('PotentialSet round-trips and compares by value', () => {
    const t = tick();
    const filled = filledPotentialSet(10, t);
    const empty = emptyPotentialSet(60);
    expect(PotentialSet.fromJSON(filled.toJSON()).equals(filled)).toBe(true);
    expect(PotentialSet.fromJSON(empty.toJSON()).equals(empty)).toBe(true);
    expect(filled.equals(undefined)).toBe(false);
    expect(filled.equals(filled)).toBe(true);
    expect(filled.equals(empty)).toBe(false);
  });

  it('keeps a stored rep target through JSON, and compares sets without it', () => {
    const t = tick();
    const stored = filledPotentialSet(10, t, kg(60), { min: 8, max: 12 });

    expect(PotentialSet.fromJSON(stored.toJSON()).toJSON().target).toEqual({ reps: { min: 8, max: 12 } });
    expect(stored.equals(filledPotentialSet(10, t, kg(60), { min: 0, max: 0 }))).toBe(true);
  });
});

describe('RecordedWeightedExercise.empty', () => {
  const history = (weights: Record<number, number>) =>
    new Map(
      Object.entries(weights).map(([index, weight]): [number, CompletedWeightedSet] => [
        Number(index),
        { weight: kg(weight), reps: 5, context: { resistance: 'external' } },
      ]),
    );
  const weightsOfEmpty = (sets: number, previous?: CompletedWeightedSets) =>
    RecordedWeightedExercise.empty(makeWeightedBlueprint({ sets }), 'pounds', previous).potentialSets.map(
      (s) => `${s.weight.value.toString()} ${s.weight.unit}`,
    );

  it('takes each weight from its own index, else the nearest earlier one', () => {
    expect(weightsOfEmpty(4, history({ 0: 52, 1: 55 }))).toEqual([
      '52 kilograms',
      '55 kilograms',
      '55 kilograms',
      '55 kilograms',
    ]);
    expect(weightsOfEmpty(3, history({ 0: 52, 2: 62 }))).toEqual(['52 kilograms', '52 kilograms', '62 kilograms']);
  });

  it('starts at zero in the given unit until an index has history', () => {
    expect(weightsOfEmpty(2)).toEqual(['0 pounds', '0 pounds']);
    expect(weightsOfEmpty(2, history({ 1: 60 }))).toEqual(['0 pounds', '60 kilograms']);
  });

  it('is sized by the plan, never by history', () => {
    expect(weightsOfEmpty(1, history({ 0: 50, 1: 55, 2: 60 }))).toEqual(['50 kilograms']);
  });
});

// ─── set table changes ────────────────────────────────────────────────────────

const kg = (value: number) => new Weight(value, 'kilograms');
const weightsOf = (exercise: RecordedWeightedExercise) => exercise.potentialSets.map((s) => s.weight.value.toNumber());

describe('RecordedWeightedExercise.withWeightFrom', () => {
  const t = tick();
  const mixed = new RecordedWeightedExercise(
    makeWeightedBlueprint({ sets: 4 }),
    [filledPotentialSet(8, t, kg(70)), emptyPotentialSet(70), filledPotentialSet(8, t, kg(70)), emptyPotentialSet(70)],
    undefined,
  );

  it('carries the weight to later unchecked sets only', () => {
    expect(weightsOf(mixed.withWeightFrom(1, kg(80)))).toEqual([70, 80, 70, 80]);
  });

  it('edits a checked set in place and still carries to later unchecked sets', () => {
    expect(weightsOf(mixed.withWeightFrom(0, kg(72.5)))).toEqual([72.5, 72.5, 70, 72.5]);
  });

  it('returns the same exercise when the weight is unchanged', () => {
    expect(mixed.withWeightFrom(1, kg(70))).toBe(mixed);
  });
});

describe('RecordedWeightedExercise.withSetLogged', () => {
  const pyramid = new RecordedWeightedExercise(
    makeWeightedBlueprint(),
    [emptyPotentialSet(80), emptyPotentialSet(70), emptyPotentialSet(70)],
    undefined,
  );

  it('logs at the planned weight without flattening later sets', () => {
    const t = tick();
    const result = pyramid.withSetLogged(0, { weight: kg(80), reps: 8 }, t);
    expect(weightsOf(result)).toEqual([80, 70, 70]);
    expect(result.getSet(0).set).toEqual(RecordedSet.of({ repsCompleted: 8, completionDateTime: t }));
  });

  it('carries a changed weight forward to the unchecked sets after it', () => {
    expect(weightsOf(pyramid.withSetLogged(0, { weight: kg(85), reps: 8 }, tick()))).toEqual([85, 85, 85]);
  });

  it('keeps the first completion time when a checked set is re-logged', () => {
    const t1 = tick();
    const t2 = tick();
    const checked = pyramid.withSetLogged(0, { weight: kg(80), reps: 6 }, t1);
    const relogged = checked.withSetLogged(0, { weight: kg(80), reps: 8 }, t2);
    expect(relogged.getSet(0).set).toEqual(RecordedSet.of({ repsCompleted: 8, completionDateTime: t1 }));
  });

  it('returns the same exercise when the set is already logged with those values', () => {
    const checked = pyramid.withSetLogged(0, { weight: kg(80), reps: 6 }, tick());
    expect(checked.withSetLogged(0, { weight: kg(80), reps: 6 }, tick())).toBe(checked);
  });
});

describe('RecordedWeightedExercise.withSetUnlogged', () => {
  it('clears the set and keeps its weight', () => {
    const exercise = new RecordedWeightedExercise(
      makeWeightedBlueprint({ sets: 1 }),
      [filledPotentialSet(8, tick(), kg(72.5))],
      undefined,
    );
    const result = exercise.withSetUnlogged(0);
    expect(result.getSet(0).set).toBeUndefined();
    expect(weightsOf(result)).toEqual([72.5]);
  });
});

describe('RecordedWeightedExercise added sets', () => {
  const bp = makeLegacyWeightedBlueprint({ targets: targets(3, 8, 12) });
  const threeSets = makeRecordedExercise(bp, [undefined, undefined, undefined], kg(70));

  it('appends an unchecked set at the last weight, with no target, and leaves the plan alone', () => {
    const result = threeSets.withSetAdded();
    expect(result.potentialSets).toHaveLength(4);
    expect(result.getSet(3).set).toBeUndefined();
    expect(result.getSet(3).weight).toEqual(kg(70));
    expect(result.getSet(3).target).toEqual({ min: 0, max: 0 });
    expect(result.blueprint).toBe(bp);
  });

  it('only removes a trailing unchecked set beyond the plan', () => {
    const added = threeSets.withSetAdded();
    const addedAndChecked = added.withSetLogged(3, { weight: kg(70), reps: 8 }, tick());
    expect(threeSets.canRemoveLastSet).toBe(false);
    expect(added.canRemoveLastSet).toBe(true);
    expect(addedAndChecked.canRemoveLastSet).toBe(false);
    expect(added.withLastSetRemoved().potentialSets).toHaveLength(3);
    expect(threeSets.withLastSetRemoved()).toBe(threeSets);
    expect(addedAndChecked.withLastSetRemoved()).toBe(addedAndChecked);
  });
});
