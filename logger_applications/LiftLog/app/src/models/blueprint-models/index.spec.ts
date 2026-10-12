import { describe, expect, it } from 'vitest';
import BigNumber from 'bignumber.js';
import { Duration } from '@js-joda/core';
import {
  CardioExerciseBlueprint,
  CardioExerciseSetBlueprint,
  ProgramBlueprint,
  ProgressionRule,
  SessionBlueprint,
  normalizeExerciseName,
  Rest,
  WeightedExerciseBlueprint,
  cardioTargetEquals,
} from '@/models/blueprint-models';
import { RecordedWeightedExercise } from '@/models/session-models';
import { makeLegacyWeightedBlueprint, targets } from '@/models/session-models/__test__/helpers';
import { LocalDate } from '@js-joda/core';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function bn(n: number) {
  return new BigNumber(n);
}

describe('blueprint models', () => {
  // ---------------------------------------------------------------------------
  // ProgressionRule
  // ---------------------------------------------------------------------------

  describe('ProgressionRule', () => {
    it('round-trips through JSON', () => {
      const rule = ProgressionRule.of({
        axis: 'reps',
        step: bn(1),
        scope: { type: 'lowestSets', pick: 'middle' },
        ceiling: bn(12),
        onCeiling: 'reset',
      });

      expect(ProgressionRule.fromJSON(rule.toJSON()).toJSON()).toEqual({
        axis: 'reps',
        step: '1',
        scope: { type: 'lowestSets', pick: 'middle' },
        ceiling: '12',
        onCeiling: 'reset',
        trigger: 'allSetsMetTarget',
      });
    });

    it('leaves an absent ceiling absent rather than writing null', () => {
      const json = ProgressionRule.load(bn(2.5)).toJSON();

      expect(json).not.toHaveProperty('ceiling');
      expect(json).not.toHaveProperty('onCeiling');
      expect(ProgressionRule.fromJSON(json).ceiling).toBeUndefined();
    });
  });

  // ---------------------------------------------------------------------------
  // normalizeExerciseName
  // ---------------------------------------------------------------------------

  describe('normalizeExerciseName', () => {
    it('lowercases and trims', () => {
      expect(normalizeExerciseName('  Bench Press  ')).toBe('bench pres');
    });

    it('strips trailing "s"', () => {
      expect(normalizeExerciseName('curls')).toBe('curl');
    });

    it('strips trailing "es"', () => {
      expect(normalizeExerciseName('lunges')).toBe('lung');
    });

    it('normalises "flies" → "flys" then strips the s', () => {
      expect(normalizeExerciseName('flies')).toBe('fly');
    });

    it('normalises "flyes" → "flys" then strips the s', () => {
      expect(normalizeExerciseName('Dumbbell Flyes')).toBe('dumbbell fly');
    });

    it('treats "Dumbbell Flies" and "Dumbbell Flyes" as equal', () => {
      expect(normalizeExerciseName('Dumbbell Flies')).toBe(normalizeExerciseName('Dumbbell Flyes'));
    });

    it('treats differently-cased names as equal', () => {
      expect(normalizeExerciseName('Squat')).toBe(normalizeExerciseName('squat'));
    });

    it('returns empty string for undefined/empty input', () => {
      expect(normalizeExerciseName('')).toBe('');
    });
  });

  // ---------------------------------------------------------------------------
  // cardioTargetEquals
  // ---------------------------------------------------------------------------

  describe('cardioTargetEquals', () => {
    it('two equal time targets are equal', () => {
      expect(
        cardioTargetEquals(
          { type: 'time', value: Duration.ofMinutes(30) },
          { type: 'time', value: Duration.ofMinutes(30) },
        ),
      ).toBe(true);
    });

    it('different time durations are not equal', () => {
      expect(
        cardioTargetEquals(
          { type: 'time', value: Duration.ofMinutes(20) },
          { type: 'time', value: Duration.ofMinutes(30) },
        ),
      ).toBe(false);
    });

    it('two equal distance targets are equal', () => {
      expect(
        cardioTargetEquals(
          { type: 'distance', value: { value: bn(5), unit: 'kilometre' } },
          { type: 'distance', value: { value: bn(5), unit: 'kilometre' } },
        ),
      ).toBe(true);
    });

    it('same distance value but different units are not equal', () => {
      expect(
        cardioTargetEquals(
          { type: 'distance', value: { value: bn(5), unit: 'kilometre' } },
          { type: 'distance', value: { value: bn(5), unit: 'mile' } },
        ),
      ).toBe(false);
    });

    it('different types are not equal', () => {
      expect(
        cardioTargetEquals(
          { type: 'time', value: Duration.ofMinutes(30) },
          { type: 'distance', value: { value: bn(5), unit: 'kilometre' } },
        ),
      ).toBe(false);
    });
  });

  // ---------------------------------------------------------------------------
  // progressionKey
  // ---------------------------------------------------------------------------

  describe('progressionKey', () => {
    it('is still computed for a weighted exercise built from scratch', () => {
      expect(WeightedExerciseBlueprint.of({ name: 'Squat', sets: 4 }).progressionKey()).toBe(
        'Squat_WeightedExerciseBlueprint_4_0',
      );
    });

    it('cardio exercise key encodes the target type of the first set', () => {
      const blueprint = CardioExerciseBlueprint.empty();
      // default first set target is 'time'
      expect(blueprint.progressionKey()).toContain('time');
    });

    it('cardio exercise key encodes distance when first set has a distance target', () => {
      const set = CardioExerciseSetBlueprint.empty().with({
        target: {
          type: 'distance',
          value: { value: bn(5), unit: 'kilometre' },
        },
      });
      const blueprint = new CardioExerciseBlueprint('Run', [set], '', '');
      expect(blueprint.progressionKey()).toContain('distance');
    });
  });

  // ---------------------------------------------------------------------------
  // progressionKey - frozen table. Every recorded exercise stores this key, and the relational
  // migration of old sessions computes it, so the formula must not drift.
  // ---------------------------------------------------------------------------

  describe('progressionKey - exact strings of stored plans', () => {
    const cases: [label: string, targets: { min: number; max: number }[], key: string][] = [
      ['fixed', targets(3, 5), 'Squat_WeightedExerciseBlueprint_3_5'],
      ['range', targets(4, 8, 12), 'Squat_WeightedExerciseBlueprint_4_8-12'],
      [
        'pyramid',
        [
          { min: 12, max: 12 },
          { min: 10, max: 10 },
          { min: 8, max: 8 },
        ],
        'Squat_WeightedExerciseBlueprint_3_12,10,8',
      ],
      [
        'bands per set',
        [
          { min: 8, max: 12 },
          { min: 6, max: 10 },
        ],
        'Squat_WeightedExerciseBlueprint_2_8-12,6-10',
      ],
    ];

    it.each(cases)('%s', (_label, planned, key) => {
      expect(makeLegacyWeightedBlueprint({ targets: planned }).progressionKey()).toBe(key);
    });

    it('leaves the rep scheme out for an exercise that tracks no load', () => {
      expect(
        makeLegacyWeightedBlueprint({ name: 'Crunch', targets: targets(3, 25), resistance: 'none' }).progressionKey(),
      ).toBe('Crunch_WeightedExerciseBlueprint_3');
    });

    it('leaves the rep scheme out where a reps rule moved the targets', () => {
      const doubleProgression = makeLegacyWeightedBlueprint({
        name: 'Chin Up',
        targets: targets(3, 8),
        progression: [
          ProgressionRule.of({ axis: 'reps', step: bn(1), ceiling: bn(12), onCeiling: 'reset' }),
          ProgressionRule.load(bn(2.5)),
        ],
      });

      expect(doubleProgression.progressionKey()).toBe('Chin Up_WeightedExerciseBlueprint_3');
    });
  });

  // ---------------------------------------------------------------------------
  // movementKey vs progressionKey
  // ---------------------------------------------------------------------------

  describe('movementKey vs progressionKey', () => {
    const fiveByFive = makeLegacyWeightedBlueprint({ name: 'Squats', targets: targets(5, 5) });
    const threeByEight = makeLegacyWeightedBlueprint({ name: 'Squats', targets: targets(3, 8) });

    it('the same movement under two rep schemes is one movement but two progressions', () => {
      expect(fiveByFive.movementKey()).toBe(threeByEight.movementKey());
      expect(fiveByFive.progressionKey()).not.toBe(threeByEight.progressionKey());
    });

    it('a differently-spelled name is the same movement but a different progression', () => {
      const singular = fiveByFive.with({ name: 'Squat' });
      expect(singular.movementKey()).toBe(fiveByFive.movementKey());
      expect(singular.progressionKey()).not.toBe(fiveByFive.progressionKey());
    });

    it('a recorded exercise keys the same as the blueprint it was built from', () => {
      const recorded = RecordedWeightedExercise.empty(fiveByFive, 'kilograms');
      expect(recorded.movementKey()).toBe(fiveByFive.movementKey());
      expect(recorded.progressionKey()).toBe(fiveByFive.progressionKey());
    });

    it('a weighted and a cardio exercise of the same name are different movements', () => {
      const rowMachine = new CardioExerciseBlueprint('Row', [CardioExerciseSetBlueprint.empty()], '', '');
      const barbellRow = fiveByFive.with({ name: 'Row' });

      expect(barbellRow.movementKey()).not.toBe(rowMachine.movementKey());
      expect(barbellRow.progressionKey()).not.toBe(rowMachine.progressionKey());
      expect(normalizeExerciseName(barbellRow.name)).toBe(normalizeExerciseName(rowMachine.name));
    });
  });
});

describe('WeightedExerciseBlueprint set count', () => {
  const pyramid = makeLegacyWeightedBlueprint({
    targets: [
      { min: 12, max: 12 },
      { min: 10, max: 10 },
      { min: 8, max: 8 },
    ],
  });
  const reps = (b: WeightedExerciseBlueprint) => b.plannedSets.map((s) => s.reps);

  it('builds sets that prescribe no reps and carry no rules', () => {
    const squat = WeightedExerciseBlueprint.of({ name: 'Squat', sets: 2 });

    expect(squat.toJSON().plannedSets).toEqual([{ reps: { min: 0, max: 0 } }, { reps: { min: 0, max: 0 } }]);
    expect(squat.toJSON().progression).toEqual([]);
    expect(WeightedExerciseBlueprint.of().plannedSets).toHaveLength(3);
  });

  it('round-trips the rep targets and rules of a stored plan unchanged', () => {
    const json = pyramid.toJSON();

    expect(WeightedExerciseBlueprint.fromJSON(json).toJSON()).toEqual(json);
    expect(json.plannedSets.map((s) => s.reps.max)).toEqual([12, 10, 8]);
    expect(json.progression).toEqual([
      { axis: 'load', step: '2.5', scope: { type: 'allSets' }, trigger: 'allSetsMetTarget' },
    ]);
  });

  describe('withSets', () => {
    it('grows with sets that prescribe nothing, keeping the stored ones', () => {
      expect(reps(pyramid.withSets(5))).toEqual([
        { min: 12, max: 12 },
        { min: 10, max: 10 },
        { min: 8, max: 8 },
        { min: 0, max: 0 },
        { min: 0, max: 0 },
      ]);
    });

    it('shrinks by truncating, and keeps the rules', () => {
      const shrunk = pyramid.withSets(2);
      expect(reps(shrunk)).toEqual([
        { min: 12, max: 12 },
        { min: 10, max: 10 },
      ]);
      expect(shrunk.progression).toBe(pyramid.progression);
    });

    it('leaves the list untouched when the count is unchanged', () => {
      expect(pyramid.withSets(3).plannedSets).toEqual(pyramid.plannedSets);
    });

    it('returns a new blueprint instance leaving the original unchanged', () => {
      expect(pyramid.withSets(5)).not.toBe(pyramid);
      expect(pyramid.plannedSets).toHaveLength(3);
    });

    it('clamps to a minimum of 1 set', () => {
      expect(pyramid.withSets(0).plannedSets).toHaveLength(1);
      expect(pyramid.withSets(-5).plannedSets).toHaveLength(1);
    });
  });

  describe('with', () => {
    it('resizes like withSets', () => {
      expect(reps(pyramid.with({ sets: 4 }))).toEqual(reps(pyramid.withSets(4)));
    });

    it('keeps the stored sets and rules when changing anything else', () => {
      const renamed = pyramid.with({ name: 'Front Squat' });
      expect(renamed.name).toBe('Front Squat');
      expect(renamed.toJSON().plannedSets).toEqual(pyramid.toJSON().plannedSets);
      expect(renamed.toJSON().progression).toEqual(pyramid.toJSON().progression);
    });
  });

  describe('equals', () => {
    const plain = WeightedExerciseBlueprint.of({ name: 'Squat', sets: 3, restBetweenSets: Rest.medium });

    it('ignores rep targets, rules and failure rest', () => {
      const failureOnly = { ...Rest.medium, failureRest: Duration.ofMinutes(20) };
      expect(pyramid.equals(plain)).toBe(true);
      expect(plain.with({ restBetweenSets: failureOnly }).equals(plain)).toBe(true);
    });

    it('tells a different set count or rest window apart', () => {
      expect(plain.withSets(4).equals(plain)).toBe(false);
      expect(plain.with({ restBetweenSets: { ...Rest.medium, maxRest: Duration.ofMinutes(4) } }).equals(plain)).toBe(
        false,
      );
    });
  });
});

describe('blueprint collections', () => {
  // ---------------------------------------------------------------------------
  // SessionBlueprint
  // ---------------------------------------------------------------------------

  describe('SessionBlueprint', () => {
    const squat = WeightedExerciseBlueprint.empty().with({ name: 'Squat' });
    const bench = WeightedExerciseBlueprint.empty().with({ name: 'Bench' });
    const session = new SessionBlueprint('Workout A', [squat, bench], '');
    const names = (s: SessionBlueprint) => s.exercises.map((x) => x.name);

    it('adds an exercise to the end', () => {
      const deadlift = WeightedExerciseBlueprint.empty().with({ name: 'Deadlift' });
      expect(names(session.withAddedExercise(deadlift))).toEqual(['Squat', 'Bench', 'Deadlift']);
      expect(names(session)).toEqual(['Squat', 'Bench']);
    });

    it('removes an exercise by value', () => {
      expect(names(session.withoutExercise(WeightedExerciseBlueprint.empty().with({ name: 'Squat' })))).toEqual([
        'Bench',
      ]);
    });

    it('replaces the exercise at an index', () => {
      const press = WeightedExerciseBlueprint.empty().with({ name: 'Overhead Press' });
      expect(names(session.withExercise(1, press))).toEqual(['Squat', 'Overhead Press']);
    });

    it('leaves the session alone for an index that is out of range', () => {
      const press = WeightedExerciseBlueprint.empty().with({ name: 'Overhead Press' });
      expect(session.withExercise(5, press)).toBe(session);
      expect(session.withExercise(-1, press)).toBe(session);
    });

    it('moves an exercise up and down', () => {
      expect(names(session.withExerciseMovedDown(squat))).toEqual(['Bench', 'Squat']);
      expect(names(session.withExerciseMovedUp(bench))).toEqual(['Bench', 'Squat']);
    });

    it('leaves the session alone when the move would fall off either end', () => {
      expect(session.withExerciseMovedUp(squat)).toBe(session);
      expect(session.withExerciseMovedDown(bench)).toBe(session);
    });

    it('leaves the session alone when the exercise is not in it', () => {
      const missing = WeightedExerciseBlueprint.empty().with({ name: 'Nowhere' });
      expect(session.withExerciseMovedUp(missing)).toBe(session);
      expect(session.withExerciseMovedDown(missing)).toBe(session);
    });
  });

  // ---------------------------------------------------------------------------
  // ProgramBlueprint
  // ---------------------------------------------------------------------------

  describe('ProgramBlueprint', () => {
    const workoutA = new SessionBlueprint('Workout A', [], '');
    const workoutB = new SessionBlueprint('Workout B', [], '');
    const program = new ProgramBlueprint('Plan', [workoutA, workoutB], LocalDate.of(2026, 8, 6));
    const names = (p: ProgramBlueprint) => p.sessions.map((x) => x.name);

    it('updates the workout at an index, leaving the others alone', () => {
      const updated = program.withSession(1, (session) => session.withName('Leg Day'));

      expect(names(updated)).toEqual(['Workout A', 'Leg Day']);
      expect(names(program)).toEqual(['Workout A', 'Workout B']);
    });

    it('leaves the plan alone when the workout index does not exist', () => {
      expect(program.withSession(7, (session) => session.withName('Nowhere'))).toBe(program);
      expect(program.withSession(-1, (session) => session.withName('Nowhere'))).toBe(program);
    });

    it('adds and removes workouts', () => {
      const workoutC = new SessionBlueprint('Workout C', [], '');

      expect(names(program.withAddedSession(workoutC))).toEqual(['Workout A', 'Workout B', 'Workout C']);
      expect(names(program.withoutSession(new SessionBlueprint('Workout A', [], '')))).toEqual(['Workout B']);
    });

    it('moves a workout up and down', () => {
      expect(names(program.withSessionMovedDown(workoutA))).toEqual(['Workout B', 'Workout A']);
      expect(names(program.withSessionMovedUp(workoutB))).toEqual(['Workout B', 'Workout A']);
      expect(program.withSessionMovedUp(workoutA)).toBe(program);
      expect(program.withSessionMovedDown(workoutB)).toBe(program);
    });
  });
});
