import { describe, expect, it } from 'vitest';
import { getImportForStrong, sessionIdFromCsvContent } from '@/services/csv-import';
import { programIdFromCsvImport } from '@/services/csv-import/csv-to-sessions';
import { parseStrongCsv } from '@/services/csv-import/strong-csv';
import { STRONG_EXPORT_KG } from '@/services/csv-import/__test__/strong-fixture';
import { readCsvImportFixture } from '@/services/csv-import/__test__/fixtures';
import { ProgramBlueprint, WeightedExerciseBlueprint } from '@/models/blueprint-models';
import { RecordedWeightedExercise, Session } from '@/models/session-models';

function importStrong(csv = STRONG_EXPORT_KG) {
  return getImportForStrong(new TextEncoder().encode(csv));
}

/** Exercise name → [reps, weight] per set, the shape a user reads off History. */
function setsOf(session: Session) {
  return session.recordedExercises.map((e) => {
    const exercise = e as RecordedWeightedExercise;
    return [
      exercise.blueprint.name,
      exercise.potentialSets.map((s) => [s.set?.repsCompleted, s.weight.value.toNumber()]),
    ];
  });
}

function onlyPlan(programs: Record<string, ProgramBlueprint>): [string, ProgramBlueprint] {
  const entries = Object.entries(programs);
  expect(entries).toHaveLength(1);
  return entries[0]!;
}

function planSummary(plan: ProgramBlueprint) {
  return plan.sessions.map((s) => [
    s.name,
    (s.exercises as WeightedExerciseBlueprint[]).map((e) => ({
      name: e.name,
      sets: e.plannedSets.length,
      reps: e.repsTargetForSet(0),
      restSeconds: [e.restBetweenSets.minRest, e.restBetweenSets.maxRest, e.restBetweenSets.failureRest].map((d) =>
        d.seconds(),
      ),
    })),
  ]);
}

describe('parseStrongCsv', () => {
  it('keeps working, drop and failure sets and rest timers, and drops warm-ups and rows without reps', () => {
    const result = parseStrongCsv(STRONG_EXPORT_KG);
    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }
    expect(result.weightUnit).toBe('kilograms');
    const workoutFour = result.rows.filter((r) => r.workoutNumber === '4').map((r) => [r.exercise, r.entry]);
    expect(workoutFour).toEqual([
      ['Bench Press (Barbell)', { type: 'set', reps: 10, weight: 62.5, notes: 'felt strong' }],
      ['Bench Press (Barbell)', { type: 'rest', seconds: 120 }],
      ['Bench Press (Barbell)', { type: 'set', reps: 9, weight: 62.5, notes: '' }],
      ['Bench Press (Barbell)', { type: 'rest', seconds: 150 }],
      ['Bench Press (Barbell)', { type: 'set', reps: 8, weight: 62.5, notes: '' }],
      ['Chest Fly', { type: 'set', reps: 12, weight: 27.5, notes: '' }],
      ['Chest Fly', { type: 'set', reps: 9, weight: 27.5, notes: '' }],
      ['Chest Fly', { type: 'rest', seconds: 90 }],
    ]);
    const workoutOneChestFly = result.rows.filter((r) => r.workoutNumber === '1' && r.exercise === 'Chest Fly');
    expect(workoutOneChestFly.map((r) => r.entry)).toEqual([
      { type: 'set', reps: 12, weight: 25, notes: '' },
      { type: 'rest', seconds: 90 },
    ]);
  });

  it('reads a comma-delimited export with pounds and unquoted empty fields', () => {
    const csv = `Workout #,Date,Workout Name,Duration (sec),Exercise Name,Set Order,Weight (lbs),Reps,RPE,Distance (meters),Seconds,Notes,Workout Notes
1,2026-02-01 07:05:00,Push,2400,Bench Press (Barbell),1,135,8,,,,,
1,2026-02-01 07:05:00,Push,2400,Bench Press (Barbell),Rest Timer,,,,,180.0,,
1,2026-02-01 07:05:00,Push,2400,Bench Press (Barbell),D,95,12,,,,,
`;
    const result = parseStrongCsv(csv);
    expect(result.ok && result.weightUnit).toBe('pounds');
    const backup = importStrong(csv);
    expect(setsOf(backup.workouts[0]!)).toEqual([
      [
        'Bench Press (Barbell)',
        [
          [8, 135],
          [12, 95],
        ],
      ],
    ]);
    const bench = backup.workouts[0]!.recordedExercises[0] as RecordedWeightedExercise;
    expect(bench.potentialSets.map((s) => s.weight.unit)).toEqual(['pounds', 'pounds']);
    expect(bench.blueprint.restBetweenSets.minRest.seconds()).toBe(180);
  });

  it('reads decimal commas in a semicolon-delimited export', () => {
    const csv = `"Workout #";"Date";"Workout Name";"Exercise Name";"Set Order";"Weight (kg)";"Reps";"Seconds"
"1";"2026-02-01 07:05:00";"Push";"Bench Press (Barbell)";"1";"62,5";"8";""
`;
    expect(setsOf(importStrong(csv).workouts[0]!)).toEqual([['Bench Press (Barbell)', [[8, 62.5]]]]);
  });

  it('rejects a FitNotes CSV', () => {
    const result = parseStrongCsv(readCsvImportFixture('fitnotes-android-export-kgs.csv'));
    expect(result).toEqual({ ok: false, error: 'Missing CSV columns: Exercise Name, Set Order' });
  });
});

describe('getImportForStrong', () => {
  it('builds one session per workout, named after the workout, in date order', () => {
    const backup = importStrong();
    expect(backup.workouts.map((s) => [s.blueprint.name, s.date.toString()])).toEqual([
      ['DAY 1', '2026-01-01'],
      ['DAY 2', '2026-01-03'],
      ['New template', '2026-01-04'],
      ['DAY 1', '2026-01-05'],
      ['DAY 2', '2026-01-07'],
    ]);
    expect(setsOf(backup.workouts[3]!)).toEqual([
      [
        'Bench Press (Barbell)',
        [
          [10, 62.5],
          [9, 62.5],
          [8, 62.5],
        ],
      ],
      [
        'Chest Fly',
        [
          [12, 27.5],
          [9, 27.5],
        ],
      ],
    ]);
    expect(setsOf(backup.workouts[1]!)).toEqual([
      [
        'Shoulder Press (Plate Loaded)',
        [
          [10, 40],
          [9, 40],
        ],
      ],
      [
        'Lateral Raise (Cable)',
        [
          [15, 7.5],
          [10, 5],
        ],
      ],
    ]);
    expect(backup.workouts[3]!.recordedExercises[0]!.notes).toBe('felt strong');
  });

  it('timestamps sets from the workout start, one second apart', () => {
    const session = importStrong().workouts[0]!;
    const times = session.recordedExercises.flatMap((e) =>
      (e as RecordedWeightedExercise).potentialSets.map((s) => s.set!.completionDateTime.toLocalDateTime().toString()),
    );
    expect(times).toEqual(['2026-01-01T13:21:15', '2026-01-01T13:21:16', '2026-01-01T13:21:17']);
  });

  it('gives every exercise the 1-20 rep target, so set count alone splits lineages', () => {
    const keys = importStrong().workouts.map((s) => s.recordedExercises.map((e) => e.progressionKey()));
    expect(keys).toEqual([
      ['Bench Press (Barbell)_WeightedExerciseBlueprint_2_1-20', 'Chest Fly_WeightedExerciseBlueprint_1_1-20'],
      [
        'Shoulder Press (Plate Loaded)_WeightedExerciseBlueprint_2_1-20',
        'Lateral Raise (Cable)_WeightedExerciseBlueprint_2_1-20',
      ],
      ['Squat (Barbell)_WeightedExerciseBlueprint_1_1-20'],
      ['Bench Press (Barbell)_WeightedExerciseBlueprint_3_1-20', 'Chest Fly_WeightedExerciseBlueprint_2_1-20'],
      [
        'Shoulder Press (Plate Loaded)_WeightedExerciseBlueprint_2_1-20',
        'Bicep Curl (Cable)_WeightedExerciseBlueprint_2_1-20',
      ],
    ]);
  });

  it('derives session ids from Workout # and start time, so importing again matches', () => {
    const first = importStrong().workouts.map((s) => s.id);
    expect(importStrong().workouts.map((s) => s.id)).toEqual(first);
    expect(first[0]).toBe(sessionIdFromCsvContent('strong|1|2026-01-01 13:21:15'));
    expect(new Set(first).size).toBe(5);
  });

  it('builds a "Strong" plan from the most recent workout of each repeated name', () => {
    const [id, plan] = onlyPlan(importStrong().programs);
    expect(id).toBe(programIdFromCsvImport('Strong'));
    expect(plan.name).toBe('Strong');
    expect(plan.lastEdited.toString()).toBe('2026-01-07');
    expect(planSummary(plan)).toEqual([
      [
        'DAY 1',
        [
          { name: 'Bench Press (Barbell)', sets: 3, reps: { min: 1, max: 20 }, restSeconds: [120, 120, 120] },
          { name: 'Chest Fly', sets: 2, reps: { min: 1, max: 20 }, restSeconds: [90, 90, 90] },
        ],
      ],
      [
        'DAY 2',
        [
          { name: 'Shoulder Press (Plate Loaded)', sets: 2, reps: { min: 1, max: 20 }, restSeconds: [90, 180, 300] },
          { name: 'Bicep Curl (Cable)', sets: 2, reps: { min: 1, max: 20 }, restSeconds: [60, 60, 60] },
        ],
      ],
    ]);
  });

  it('gives plan exercises the same progression keys as the history they were copied from', () => {
    const backup = importStrong();
    const [, plan] = onlyPlan(backup.programs);
    const latestOf = (name: string) => backup.workouts.findLast((s) => s.blueprint.name === name)!;
    for (const planSession of plan.sessions) {
      expect(planSession.exercises.map((e) => e.progressionKey())).toEqual(
        latestOf(planSession.name).recordedExercises.map((e) => e.progressionKey()),
      );
    }
  });

  it('orders plan workouts by natural name order', () => {
    const header = `Workout #,Date,Workout Name,Exercise Name,Set Order,Weight (kg),Reps`;
    const csv = [
      header,
      ...['DAY 10', 'DAY 2', 'DAY 10', 'DAY 2'].map(
        (name, i) => `${i + 1},2026-03-0${i + 1} 10:00:00,${name},Squat,1,100,5`,
      ),
    ].join('\n');
    const [, plan] = onlyPlan(importStrong(csv).programs);
    expect(plan.sessions.map((s) => s.name)).toEqual(['DAY 2', 'DAY 10']);
  });

  it('returns no plan when no workout name repeats', () => {
    const csv = `Workout #,Date,Workout Name,Exercise Name,Set Order,Weight (kg),Reps
1,2026-03-01 10:00:00,Legs,Squat,1,100,5
2,2026-03-03 10:00:00,Arms,Curl,1,20,10
`;
    const backup = importStrong(csv);
    expect(backup.workouts).toHaveLength(2);
    expect(backup.programs).toEqual({});
  });

  it('round-trips sessions and the plan through JSON', () => {
    const backup = importStrong();
    const session = backup.workouts[3]!;
    expect(Session.fromJSON(session.toJSON()).equals(session)).toBe(true);
    const [, plan] = onlyPlan(backup.programs);
    expect(ProgramBlueprint.fromJSON(plan.toJSON()).equals(plan)).toBe(true);
  });

  it('throws when nothing can be imported', () => {
    const csv = `Workout #,Date,Workout Name,Exercise Name,Set Order,Weight (kg),Reps,Seconds
1,2026-03-01 10:00:00,Core,Plank,1,,,60
`;
    expect(() => importStrong(csv)).toThrow('No workout rows found in CSV');
  });
});
