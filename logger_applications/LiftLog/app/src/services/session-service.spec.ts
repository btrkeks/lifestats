import { beforeEach, describe, it, expect, vi } from 'vitest';
import { SessionService } from '@/services/session-service';
import { DatabaseMigrationService } from '@/services/database-migration-service';
import { setActiveSession, writeSession } from '@/db/sessions';
import { ProgressionRule, SessionBlueprint, WeightedExerciseBlueprint } from '@/models/blueprint-models';
import BigNumber from 'bignumber.js';
import {
  freeformSessionName,
  PotentialSet,
  RecordedExercise,
  RecordedSet,
  RecordedWeightedExercise,
  Session,
} from '@/models/session-models';
import {
  makeLegacyWeightedBlueprint,
  makeRecordedExercise,
  makeWeightedBlueprint,
  targets,
  tick,
} from '@/models/session-models/__test__/helpers';
import { Weight, WeightUnit } from '@/models/weight';
import { LocalDate } from '@js-joda/core';
import { drizzle, type ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { openDatabaseAsync } from 'expo-sqlite';
import { v4 as uuid } from 'uuid';
import type { RootState } from '@/store';

let db: ExpoSQLiteDatabase;

beforeEach(async () => {
  db = drizzle(await openDatabaseAsync(':memory:'));
  await new DatabaseMigrationService(db, { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn() } as never, {
    importOldData: async () => {},
  }).migrate();
});

function makeService({ useImperialUnits = false } = {}) {
  return new SessionService(db, () => ({ settings: { useImperialUnits } }) as unknown as RootState);
}

function store(...sessions: Session[]) {
  db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
}

function performed(
  name: string,
  exercises: RecordedExercise[] = [makeRecordedExercise(makeWeightedBlueprint(), [10])],
) {
  return new Session(
    uuid(),
    new SessionBlueprint(
      name,
      exercises.map((x) => x.blueprint),
      '',
    ),
    exercises,
    LocalDate.of(2025, 4, 5),
    undefined,
    undefined,
  );
}

async function collect(iter: AsyncIterableIterator<Session>, count: number): Promise<Session[]> {
  const out: Session[] = [];
  for await (const session of iter) {
    out.push(session);
    if (out.length >= count) break;
  }
  return out;
}

function bp(name: string, notes = '') {
  return new SessionBlueprint(name, [], notes);
}

describe('SessionService.getUpcomingSessions', () => {
  it('walks the plan in order and cycles when there is no history', async () => {
    const plan = [bp('Push'), bp('Pull'), bp('Legs')];
    const upcoming = await collect(makeService().getUpcomingSessions(plan), 6);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Push', 'Pull', 'Legs', 'Push', 'Pull', 'Legs']);
  });

  it('advances past duplicate-named workouts instead of stalling', async () => {
    // Regression: matching the next workout by name resolved duplicates to the
    // first occurrence, trapping progression on it forever.
    const plan = [bp('Upper'), bp('Lower'), bp('Lower', 'second'), bp('Cardio')];
    const upcoming = await collect(makeService().getUpcomingSessions(plan), 8);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual([
      'Upper',
      'Lower',
      'Lower',
      'Cardio',
      'Upper',
      'Lower',
      'Lower',
      'Cardio',
    ]);
    // The second "Lower" must be the distinct blueprint, not a repeat of the first.
    expect(upcoming[2]!.blueprint.notes).toBe('second');
  });

  it('continues from the last completed session', async () => {
    const plan = [bp('Push'), bp('Pull'), bp('Legs')];
    store(performed('Pull'));

    const upcoming = await collect(makeService().getUpcomingSessions(plan), 3);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Legs', 'Push', 'Pull']);
  });

  it('continues from the workout in progress', async () => {
    const plan = [bp('Push'), bp('Pull'), bp('Legs')];
    store(performed('Legs'));
    db.transaction((tx) => setActiveSession(tx, performed('Push')));

    const upcoming = await collect(makeService().getUpcomingSessions(plan), 3);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Pull', 'Legs', 'Push']);
  });

  it('does not count a freeform workout as a step through the plan', async () => {
    const plan = [bp('Push'), bp('Pull'), bp('Legs')];
    store(performed('Push'), performed(freeformSessionName));

    const upcoming = await collect(makeService().getUpcomingSessions(plan), 1);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Pull']);
  });

  it('restarts the plan when the last session is no longer in the plan', async () => {
    const plan = [bp('Push'), bp('Pull')];
    store(performed('Removed'));

    const upcoming = await collect(makeService().getUpcomingSessions(plan), 3);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Push', 'Pull', 'Push']);
  });

  it('carries the last bodyweight into every upcoming session', async () => {
    const plan = [bp('Push'), bp('Pull')];
    store(performed('Push').with({ bodyweight: new Weight(80, 'kilograms') }));

    const upcoming = await collect(makeService().getUpcomingSessions(plan), 2);

    expect(upcoming.map((s) => s.bodyweight?.value.toNumber())).toEqual([80, 80]);
  });

  it('repeats a single-workout plan', async () => {
    const plan = [bp('Full Body')];
    const upcoming = await collect(makeService().getUpcomingSessions(plan), 3);

    expect(upcoming.map((s) => s.blueprint.name)).toEqual(['Full Body', 'Full Body', 'Full Body']);
  });

  it('yields nothing for an empty plan', async () => {
    const upcoming = await collect(makeService().getUpcomingSessions([]), 3);
    expect(upcoming).toHaveLength(0);
  });

  it('gives every upcoming session a unique id', async () => {
    const plan = [bp('A'), bp('B')];
    const upcoming = await collect(makeService().getUpcomingSessions(plan), 6);
    expect(new Set(upcoming.map((s) => s.id)).size).toBe(upcoming.length);
  });
});

describe('SessionService weighted history', () => {
  /** `[weight, reps]` per set, `reps` undefined for a set left unchecked; sets complete in order. */
  function squat(sets: [number, number | undefined][], init: { name?: string; unit?: WeightUnit } = {}) {
    return new RecordedWeightedExercise(
      makeWeightedBlueprint({ name: init.name ?? 'Squat', sets: sets.length }),
      sets.map(([weight, reps]) =>
        PotentialSet.of({
          weight: new Weight(weight, init.unit ?? 'kilograms'),
          set: reps === undefined ? undefined : RecordedSet.of({ repsCompleted: reps, completionDateTime: tick() }),
        }),
      ),
      undefined,
    );
  }

  async function upcoming(blueprint: WeightedExerciseBlueprint, service = makeService()) {
    const [session] = await collect(service.getUpcomingSessions([new SessionBlueprint('Day', [blueprint], '')]), 1);
    return session!.recordedExercises[0] as RecordedWeightedExercise;
  }
  const weightsOf = (exercise: RecordedWeightedExercise) =>
    exercise.potentialSets.map((s) => `${s.weight.value.toString()} ${s.weight.unit}`);

  function storeSkippedIndexHistory() {
    store(
      performed('Day', [
        squat([
          [50, 8],
          [55, 7],
          [60, 6],
        ]),
      ]),
      performed('Day', [
        squat([
          [52, 9],
          [999, undefined],
          [62, 5],
          [65, 4],
        ]),
      ]),
    );
  }

  it('sizes the workout by the plan and starts each set at the weight last completed at its index', async () => {
    storeSkippedIndexHistory();

    const exercise = await upcoming(makeWeightedBlueprint({ sets: 3 }));

    expect(weightsOf(exercise)).toEqual(['52 kilograms', '55 kilograms', '62 kilograms']);
    expect(exercise.potentialSets.map((s) => s.set)).toEqual([undefined, undefined, undefined]);
  });

  it('starts a set never completed at the weight of the nearest earlier set that was', async () => {
    store(
      performed('Day', [
        squat([
          [52, 9],
          [55, 7],
        ]),
      ]),
    );
    expect(weightsOf(await upcoming(makeWeightedBlueprint({ sets: 4 })))).toEqual([
      '52 kilograms',
      '55 kilograms',
      '55 kilograms',
      '55 kilograms',
    ]);

    storeSkippedIndexHistory();
    expect(weightsOf(await upcoming(makeWeightedBlueprint({ sets: 5 })))).toEqual([
      '52 kilograms',
      '55 kilograms',
      '62 kilograms',
      '65 kilograms',
      '65 kilograms',
    ]);
  });

  it('never progresses a weight, even after an old plan with a load rule hit every target', async () => {
    const blueprint = makeLegacyWeightedBlueprint({
      targets: targets(2, 10),
      progression: [ProgressionRule.load(new BigNumber(2.5))],
    });
    store(
      performed('Day', [
        new RecordedWeightedExercise(
          blueprint,
          [10, 10].map((reps) =>
            PotentialSet.of({
              weight: new Weight(60, 'kilograms'),
              target: { min: 10, max: 10 },
              set: RecordedSet.of({ repsCompleted: reps, completionDateTime: tick() }),
            }),
          ),
          undefined,
        ),
      ]),
    );

    const exercise = await upcoming(blueprint);

    expect(weightsOf(exercise)).toEqual(['60 kilograms', '60 kilograms']);
    expect(exercise.potentialSets.map((s) => s.target)).toEqual([
      { min: 0, max: 0 },
      { min: 0, max: 0 },
    ]);
  });

  it('matches history by movement name, whatever the rep scheme or spelling', async () => {
    store(
      performed('Day', [
        squat(
          [
            [100, 5],
            [100, 5],
            [100, 5],
            [100, 5],
            [100, 5],
          ],
          { name: 'squats' },
        ),
      ]),
    );

    expect(weightsOf(await upcoming(makeWeightedBlueprint({ name: 'Squat', sets: 2 })))).toEqual([
      '100 kilograms',
      '100 kilograms',
    ]);
    expect(weightsOf(await upcoming(makeWeightedBlueprint({ name: 'Front Squat', sets: 1 })))).toEqual(['0 kilograms']);
  });

  it('keeps the unit a weight was lifted in', async () => {
    store(performed('Day', [squat([[135, 5]], { unit: 'pounds' })]));

    expect(weightsOf(await upcoming(makeWeightedBlueprint({ sets: 1 })))).toEqual(['135 pounds']);
  });

  it('learns nothing from the workout in progress', async () => {
    store(performed('Day', [squat([[60, 5]])]));
    db.transaction((tx) => setActiveSession(tx, performed('Day', [squat([[100, 5]])])));

    expect(weightsOf(await upcoming(makeWeightedBlueprint({ sets: 1 })))).toEqual(['60 kilograms']);
  });

  it('starts a fresh exercise at zero, in the unit the user prefers', async () => {
    expect(weightsOf(await upcoming(makeWeightedBlueprint({ sets: 2 })))).toEqual(['0 kilograms', '0 kilograms']);
    expect(
      weightsOf(await upcoming(makeWeightedBlueprint({ sets: 1 }), makeService({ useImperialUnits: true }))),
    ).toEqual(['0 pounds']);
  });
});
