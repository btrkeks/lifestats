import { cardioSetsSchema, recordedExercisesSchema, sessionsSchema, weightedSetsSchema } from '@/db/schema';
import {
  deleteSession,
  readActiveSession,
  readEarliestSessionDate,
  readExistingSessionIds,
  readLatestCardioExercises,
  readPersonalRecords,
  readPreviousComparableSession,
  readExerciseHistory,
  readPreviousCardioExercises,
  readSession,
  readSessions,
  readSessionsBetween,
  readSessionToContinueFrom,
  readStartedSessionVolumes,
  readWeightedHistory,
  setActiveSession,
  writeSession,
} from '@/db/sessions';
import {
  CardioExerciseBlueprint,
  CardioExerciseSetBlueprint,
  movementKeyFor,
  Rest,
  SessionBlueprint,
} from '@/models/blueprint-models';
import {
  freeformSessionName,
  RecordedCardioExercise,
  PotentialSet,
  RecordedExercise,
  RecordedSet,
  RecordedWeightedExercise,
  Session,
  WeightedHistory,
} from '@/models/session-models';
import {
  makeCardioBlueprint,
  makeLegacyWeightedBlueprint,
  makeRecordedExercise,
  makeSession,
  makeWeightedBlueprint,
  targets,
  tickAt,
} from '@/models/session-models/__test__/helpers';
import { SessionGenerator } from '@/models/storage/generators';
import { TemporalComparer } from '@/models/comparers';
import { sessionVolume } from '@/store/activity/volume';
import { findPersonalRecords } from '@/store/stats/personal-records';
import { getSessionReferenceTime } from '@/store/stored-sessions';
import { DatabaseMigrationService } from '@/services/database-migration-service';
import { Weight, WeightUnit } from '@/models/weight';
import { Duration, LocalDate, OffsetDateTime, ZoneId } from '@js-joda/core';
import BigNumber from 'bignumber.js';
import { eq, sql } from 'drizzle-orm';
import { drizzle, type ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { openDatabaseAsync } from 'expo-sqlite';
import fc from 'fast-check';
import { v4 as uuid } from 'uuid';
import { beforeEach, describe, expect, it, vi } from 'vitest';

async function createDb(): Promise<ExpoSQLiteDatabase> {
  const db = drizzle(await openDatabaseAsync(':memory:'));
  await new DatabaseMigrationService(db, { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn() } as never, {
    importOldData: async () => {},
  }).migrate();
  return db;
}

function canonical(session: Session): Session {
  const normalized = session.with({
    recordedExercises: session.recordedExercises.map((exercise) =>
      exercise instanceof RecordedCardioExercise
        ? exercise.with({
            blueprint: exercise.blueprint.with({ sets: exercise.sets.map((set) => set.blueprint) }),
            sets: exercise.sets.map((set) =>
              set.with({ duration: set.duration && Duration.ofMillis(set.duration.toMillis()) }),
            ),
          })
        : exercise,
    ),
  });
  return Session.fromJSON(normalized.toJSON());
}

const read = (db: ExpoSQLiteDatabase, id: string) => readSessions(db).sessions.find((x) => x.id === id);

const countRows = (db: ExpoSQLiteDatabase) => ({
  sessions: db.select().from(sessionsSchema).all().length,
  exercises: db.select().from(recordedExercisesSchema).all().length,
  weightedSets: db.select().from(weightedSetsSchema).all().length,
  cardioSets: db.select().from(cardioSetsSchema).all().length,
});

function workout() {
  const squat = makeWeightedBlueprint({ restBetweenSets: Rest.medium, supersetWithNext: true });
  return makeSession([squat, makeCardioBlueprint(2)]).with({
    bodyweight: new Weight(80.5, 'kilograms'),
    recordedExercises: [
      makeRecordedExercise(squat, [5, undefined, 3], new Weight(102.5, 'kilograms'), () => tickAt(9, 30)),
      RecordedCardioExercise.empty(makeCardioBlueprint(2)),
    ],
  });
}

describe('session tables', () => {
  let db: ExpoSQLiteDatabase;

  beforeEach(async () => {
    db = await createDb();
  });

  it('reads back every session it writes', () => {
    fc.assert(
      fc.property(SessionGenerator, (generated) => {
        const session = canonical(generated);

        db.transaction((tx) => writeSession(tx, session));

        expect(read(db, session.id)?.toJSON()).toEqual(session.toJSON());
        db.transaction((tx) => deleteSession(tx, session.id));
      }),
    );
  });

  it('replaces the exercises and sets of a session it already stored', () => {
    const session = workout();
    db.transaction((tx) => writeSession(tx, session));

    const trimmed = canonical(session.with({ recordedExercises: session.recordedExercises.slice(0, 1) }));
    db.transaction((tx) => writeSession(tx, trimmed));

    expect(read(db, session.id)?.toJSON()).toEqual(trimmed.toJSON());
    expect(countRows(db)).toEqual({ sessions: 1, exercises: 1, weightedSets: 3, cardioSets: 0 });
  });

  it('stores durations as milliseconds and completion times with their epoch millis', () => {
    const session = workout();

    db.transaction((tx) => writeSession(tx, session));

    const [exercise] = db.select().from(recordedExercisesSchema).where(eq(recordedExercisesSchema.ord, 0)).all();
    expect(exercise?.restMinMs).toBe(Rest.medium.minRest.toMillis());
    const [set] = db.select().from(weightedSetsSchema).where(eq(weightedSetsSchema.ord, 0)).all();
    expect(set?.completedAtMs).toBe(tickAt(9, 30).toInstant().toEpochMilli());
  });

  it('stores a new session inactive, and writing an active one leaves its flag alone', () => {
    const session = workout();

    db.transaction((tx) => writeSession(tx, session));
    expect(readSessions(db).activeSessionId).toBeUndefined();

    db.transaction((tx) => setActiveSession(tx, session));
    db.transaction((tx) => writeSession(tx, session.with({ date: session.date.plusDays(1) })));
    expect(readSessions(db).activeSessionId).toBe(session.id);
  });

  it('keeps at most one session active, and clears it', () => {
    const first = workout();
    const second = workout();

    db.transaction((tx) => setActiveSession(tx, first));
    db.transaction((tx) => setActiveSession(tx, second));
    expect(readSessions(db).activeSessionId).toBe(second.id);
    expect(read(db, first.id)).toBeDefined();

    db.transaction((tx) => setActiveSession(tx, undefined));
    expect(readSessions(db).activeSessionId).toBeUndefined();
  });

  it('deletes a session along with its exercises and sets', () => {
    const kept = workout();
    const deleted = workout();
    db.transaction((tx) => {
      writeSession(tx, kept);
      writeSession(tx, deleted);
    });

    db.transaction((tx) => deleteSession(tx, deleted.id));

    expect(readSessions(db).sessions.map((x) => x.id)).toEqual([kept.id]);
    expect(countRows(db)).toEqual({ sessions: 1, exercises: 2, weightedSets: 3, cardioSets: 2 });
  });

  it('enforces foreign keys once migrated, so deleting a session row cascades', () => {
    db.transaction((tx) => writeSession(tx, workout()));

    db.run(sql`DELETE FROM session`);

    expect(countRows(db)).toEqual({ sessions: 0, exercises: 0, weightedSets: 0, cardioSets: 0 });
  });
});

function sessionOf(name: string, exercises: RecordedExercise[], date = LocalDate.of(2025, 4, 5)) {
  return new Session(
    uuid(),
    new SessionBlueprint(
      name,
      exercises.map((x) => x.blueprint),
      '',
    ),
    exercises,
    date,
    undefined,
    undefined,
  );
}

const squatAt = (at: OffsetDateTime, weight = 100) =>
  makeRecordedExercise(makeWeightedBlueprint(), [10], new Weight(weight, 'kilograms'), () => at);

describe('readSessionToContinueFrom', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));

  beforeEach(async () => {
    db = await createDb();
  });

  it('finds nothing before the first session', () => {
    expect(readSessionToContinueFrom(db)).toBeUndefined();
  });

  it('continues from the session whose last set was completed most recently, whatever its date', () => {
    store(
      sessionOf('Push', [squatAt(tickAt(9, 0))], LocalDate.of(2025, 4, 9)),
      sessionOf('Pull', [squatAt(tickAt(10, 0))], LocalDate.of(2025, 4, 1)),
    );

    expect(readSessionToContinueFrom(db)?.name).toBe('Pull');
  });

  it('places a session with nothing completed at the start of its date', () => {
    store(
      sessionOf('Push', [squatAt(tickAt(9, 0))], LocalDate.of(2025, 4, 5)),
      sessionOf('Pull', [makeRecordedExercise(makeWeightedBlueprint(), [undefined])], LocalDate.of(2025, 4, 7)),
      sessionOf('Legs', [], LocalDate.of(2025, 4, 3)),
    );

    expect(readSessionToContinueFrom(db)?.name).toBe('Pull');
  });

  it('skips freeform workouts, which are not part of the plan', () => {
    store(sessionOf('Push', [squatAt(tickAt(9, 0))]), sessionOf(freeformSessionName, [squatAt(tickAt(10, 0))]));

    expect(readSessionToContinueFrom(db)?.name).toBe('Push');
  });

  it('continues from the workout in progress over any finished one', () => {
    const inProgress = sessionOf(freeformSessionName, [squatAt(tickAt(8, 0))]);
    store(sessionOf('Push', [squatAt(tickAt(9, 0))]));
    db.transaction((tx) => setActiveSession(tx, inProgress));

    expect(readSessionToContinueFrom(db)?.name).toBe(freeformSessionName);
  });

  it('carries the bodyweight', () => {
    store(sessionOf('Push', [squatAt(tickAt(9, 0))]).with({ bodyweight: new Weight(80.5, 'kilograms') }));

    expect(readSessionToContinueFrom(db)?.bodyweight?.toJSON()).toEqual(new Weight(80.5, 'kilograms').toJSON());
  });
});

describe('readLatestCardioExercises', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
  const row = makeCardioBlueprint(2);
  const rowedAt = (at: OffsetDateTime, incline: number) =>
    RecordedCardioExercise.empty(row).with({
      sets: RecordedCardioExercise.empty(row).sets.map((set) =>
        set.with({ completionDateTime: at, incline: BigNumber(incline) }),
      ),
    });
  const inclines = (exercise: RecordedCardioExercise | undefined) => exercise?.sets.map((x) => x.incline?.toNumber());

  beforeEach(async () => {
    db = await createDb();
  });

  it('reads the most recently completed performance of each lineage, including the workout in progress', () => {
    store(
      sessionOf('Later', [rowedAt(tickAt(10, 0), 4)], LocalDate.of(2025, 4, 1)),
      sessionOf('Earlier', [rowedAt(tickAt(9, 0), 3)], LocalDate.of(2025, 4, 9)),
    );
    expect(inclines(readLatestCardioExercises(db, [row.progressionKey()])[row.progressionKey()])).toEqual([4, 4]);

    db.transaction((tx) => setActiveSession(tx, sessionOf('Now', [rowedAt(tickAt(11, 0), 5)])));
    expect(inclines(readLatestCardioExercises(db, [row.progressionKey()])[row.progressionKey()])).toEqual([5, 5]);
  });

  it('reads only cardio lineages, and nothing for one never completed', () => {
    store(sessionOf('Day', [squatAt(tickAt(9, 0)), RecordedCardioExercise.empty(row)]));

    expect(readLatestCardioExercises(db, [makeWeightedBlueprint().progressionKey(), row.progressionKey()])).toEqual({});
    expect(readLatestCardioExercises(db, [])).toEqual({});
  });
});

describe('reading sessions by what is asked for', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));

  beforeEach(async () => {
    db = await createDb();
  });

  it('reads one session by id, with its exercises and sets', () => {
    const wanted = canonical(workout());
    store(wanted, workout());

    expect(readSession(db, wanted.id)?.toJSON()).toEqual(wanted.toJSON());
    expect(readSession(db, uuid())).toBeUndefined();
  });

  it('reads the workout in progress apart from the finished ones', () => {
    const finished = sessionOf('Push', [squatAt(tickAt(9, 0))]);
    const inProgress = sessionOf('Pull', [squatAt(tickAt(10, 0))]);
    store(finished);
    db.transaction((tx) => setActiveSession(tx, inProgress));

    expect(readActiveSession(db)?.id).toBe(inProgress.id);
    const thisYear = readSessionsBetween(db, LocalDate.of(2025, 1, 1), LocalDate.of(2025, 12, 31));
    expect(thisYear.map((x) => x.id)).toEqual([finished.id]);
  });

  it('reads finished sessions dated within a range, inclusive at both ends', () => {
    const before = sessionOf('Before', [], LocalDate.of(2025, 3, 31));
    const first = sessionOf('First', [], LocalDate.of(2025, 4, 1));
    const last = sessionOf('Last', [], LocalDate.of(2025, 4, 30));
    const after = sessionOf('After', [], LocalDate.of(2025, 5, 1));
    store(before, first, last, after);

    const names = readSessionsBetween(db, LocalDate.of(2025, 4, 1), LocalDate.of(2025, 4, 30)).map(
      (x) => x.blueprint.name,
    );

    expect(names.sort()).toEqual(['First', 'Last']);
  });

  it('reads the date of the earliest finished session', () => {
    expect(readEarliestSessionDate(db)).toBeUndefined();
    const inProgress = sessionOf('Now', [], LocalDate.of(2025, 1, 1));
    store(sessionOf('Later', [], LocalDate.of(2025, 4, 9)), sessionOf('Earlier', [], LocalDate.of(2025, 4, 2)));
    db.transaction((tx) => setActiveSession(tx, inProgress));

    expect(readEarliestSessionDate(db)?.toString()).toBe('2025-04-02');
  });

  it('reads which of the given ids are stored', () => {
    const stored = sessionOf('Push', []);
    store(stored);
    const missing = uuid();

    expect([...readExistingSessionIds(db, [stored.id, missing])]).toEqual([stored.id]);
    expect(readExistingSessionIds(db, []).size).toBe(0);
  });
});

describe('readExerciseHistory', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
  const squatKey = makeWeightedBlueprint().movementKey();

  beforeEach(async () => {
    db = await createDb();
  });

  it('reads every started performance of a movement, newest first', () => {
    store(
      sessionOf('Earlier', [squatAt(tickAt(9, 0), 90)], LocalDate.of(2025, 4, 9)),
      sessionOf('Later', [squatAt(tickAt(10, 0), 110)], LocalDate.of(2025, 4, 1)),
      sessionOf('Abandoned', [makeRecordedExercise(makeWeightedBlueprint(), [undefined])]),
    );

    const weights = (readExerciseHistory(db, squatKey) as RecordedWeightedExercise[]).map((x) =>
      x.potentialSets[0]?.weight.value.toNumber(),
    );

    expect(weights).toEqual([110, 90]);
  });

  it('counts the same movement under another rep scheme', () => {
    const threeByEight = makeLegacyWeightedBlueprint({ targets: targets(3, 8) });
    store(
      sessionOf('Day', [
        makeRecordedExercise(threeByEight, [8, 8, 8], new Weight(80, 'kilograms'), () => tickAt(9, 0)),
      ]),
    );

    expect(readExerciseHistory(db, squatKey)).toHaveLength(1);
  });

  it('leaves out the workout in progress', () => {
    store(sessionOf('Kept', [squatAt(tickAt(8, 0))]));
    db.transaction((tx) => setActiveSession(tx, sessionOf('Now', [squatAt(tickAt(10, 0))])));

    expect(readExerciseHistory(db, squatKey)).toHaveLength(1);
  });
});

describe('readPreviousCardioExercises', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
  const forget = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => deleteSession(tx, session.id)));
  const row = makeCardioBlueprint(1);
  const rowedAt = (at: OffsetDateTime, seconds: number) =>
    RecordedCardioExercise.empty(row).with({
      sets: [
        RecordedCardioExercise.empty(row).sets[0]!.with({
          completionDateTime: at,
          duration: Duration.ofSeconds(seconds),
        }),
      ],
    });
  const secondsOf = (exercise: RecordedCardioExercise | undefined) => exercise?.sets[0]?.duration?.seconds();

  beforeEach(async () => {
    db = await createDb();
  });

  it('leaves out the excluded session, the workout in progress, and performances with nothing completed', () => {
    const excluded = sessionOf('Excluded', [rowedAt(tickAt(9, 0), 120)]);
    store(
      excluded,
      sessionOf('Kept', [rowedAt(tickAt(8, 0), 100)]),
      sessionOf('Abandoned', [RecordedCardioExercise.empty(row)]),
    );
    db.transaction((tx) => setActiveSession(tx, sessionOf('Now', [rowedAt(tickAt(10, 0), 140)])));

    expect(secondsOf(readPreviousCardioExercises(db, [row.progressionKey()], excluded.id)[row.progressionKey()])).toBe(
      100,
    );
    expect(readPreviousCardioExercises(db, [], excluded.id)).toEqual({});
  });

  it('agrees with the newest performance of each lineage in its history', () => {
    fc.assert(
      fc.property(fc.uniqueArray(SessionGenerator, { selector: (x) => x.id, maxLength: 4 }), (generated) => {
        const sessions = generated.map(canonical);
        store(...sessions);
        try {
          const exercises = sessions
            .flatMap((x) => x.recordedExercises)
            .filter((x) => x instanceof RecordedCardioExercise);
          const previous = readPreviousCardioExercises(
            db,
            [...new Set(exercises.map((x) => x.progressionKey()))],
            uuid(),
          );

          for (const movementKey of new Set(exercises.map((x) => x.movementKey()))) {
            const history = readExerciseHistory(db, movementKey);
            for (const lineage of new Set(
              exercises.filter((x) => x.movementKey() === movementKey).map((x) => x.progressionKey()),
            )) {
              expect(previous[lineage]?.toJSON()).toEqual(
                history.find((x) => x.progressionKey() === lineage)?.toJSON(),
              );
            }
          }
        } finally {
          forget(...sessions);
        }
      }),
    );
  });
});

describe('readWeightedHistory', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
  const squatKey = movementKeyFor('Squat', 'WeightedExerciseBlueprint');

  /** One weighted exercise; `[weight, reps]` per set, `reps` undefined for a set left unchecked. */
  function performed(
    hour: number,
    sets: [number, number | undefined][],
    init: { name?: string; notes?: string; unit?: WeightUnit } = {},
  ) {
    return new RecordedWeightedExercise(
      makeWeightedBlueprint({ name: init.name ?? 'Squat', sets: sets.length }),
      sets.map(([weight, reps], index) =>
        PotentialSet.of({
          weight: new Weight(weight, init.unit ?? 'kilograms'),
          set:
            reps === undefined
              ? undefined
              : RecordedSet.of({ repsCompleted: reps, completionDateTime: tickAt(hour, index) }),
        }),
      ),
      init.notes,
    );
  }

  const describeSets = (history: WeightedHistory | undefined) =>
    [...(history?.sets ?? [])]
      .sort(([a], [b]) => a - b)
      .map(([index, set]) => `${index}: ${set.weight.value.toString()} ${set.weight.unit} x ${set.reps}`);

  beforeEach(async () => {
    db = await createDb();
  });

  it('takes each set index from the last time that index was completed', () => {
    store(
      sessionOf('Old', [
        performed(
          8,
          [
            [50, 8],
            [55, 7],
            [60, 6],
          ],
          { notes: 'old cue' },
        ),
      ]),
      sessionOf('New', [
        performed(
          9,
          [
            [52, 9],
            [999, undefined],
            [62, 5],
            [65, 4],
          ],
          { notes: 'new cue' },
        ),
      ]),
    );

    const history = readWeightedHistory(db, { movementKeys: [squatKey] }).get(squatKey);

    expect(describeSets(history)).toEqual([
      '0: 52 kilograms x 9',
      '1: 55 kilograms x 7',
      '2: 62 kilograms x 5',
      '3: 65 kilograms x 4',
    ]);
    expect(history?.lastNotes).toBe('new cue');
  });

  it('keeps the notes of the latest performance even when they are empty', () => {
    store(
      sessionOf('Old', [performed(8, [[50, 8]], { notes: 'old cue' })]),
      sessionOf('New', [performed(9, [[52, 9]], { notes: '' })]),
    );

    expect(readWeightedHistory(db, { movementKeys: [squatKey] }).get(squatKey)?.lastNotes).toBe('');
  });

  it('leaves out the excluded session and the workout in progress', () => {
    const excluded = sessionOf('Excluded', [performed(10, [[120, 5]])]);
    store(excluded, sessionOf('Kept', [performed(8, [[100, 5]])]));
    db.transaction((tx) => setActiveSession(tx, sessionOf('Now', [performed(11, [[140, 5]])])));

    expect(describeSets(readWeightedHistory(db, { movementKeys: [squatKey] }).get(squatKey))).toEqual([
      '0: 120 kilograms x 5',
    ]);
    expect(
      describeSets(readWeightedHistory(db, { movementKeys: [squatKey], excludeSessionId: excluded.id }).get(squatKey)),
    ).toEqual(['0: 100 kilograms x 5']);
  });

  it('counts a zero-rep set, keeps its unit, and ignores a weight that was never checked', () => {
    store(
      sessionOf('Old', [
        performed(
          8,
          [
            [100, 3],
            [135, 2],
          ],
          { unit: 'pounds' },
        ),
      ]),
      sessionOf('New', [
        performed(
          9,
          [
            [45, 0],
            [500, undefined],
          ],
          { unit: 'pounds' },
        ),
      ]),
    );

    expect(describeSets(readWeightedHistory(db, { movementKeys: [squatKey] }).get(squatKey))).toEqual([
      '0: 45 pounds x 0',
      '1: 135 pounds x 2',
    ]);
  });

  it('breaks a tie between two performances in one workout by their order', () => {
    const first = performed(9, [[60, 5]], { notes: 'first' });
    const second = performed(9, [[70, 5]], { notes: 'second' });
    store(sessionOf('Twice', [first, second]));

    const history = readWeightedHistory(db, { movementKeys: [squatKey] }).get(squatKey);

    expect(describeSets(history)).toEqual(['0: 60 kilograms x 5']);
    expect(history?.lastNotes).toBe('first');
  });

  it('matches by movement name whatever the set count or rep scheme, and keeps other names apart', () => {
    const fiveByFive = new RecordedWeightedExercise(
      makeLegacyWeightedBlueprint({ name: 'squats', targets: targets(5, 5) }),
      [
        PotentialSet.of({
          weight: new Weight(100, 'kilograms'),
          set: RecordedSet.of({ repsCompleted: 5, completionDateTime: tickAt(8, 0) }),
        }),
      ],
      undefined,
    );
    store(
      sessionOf('Old', [fiveByFive, performed(8, [[40, 10]], { name: 'Front Squat' })]),
      sessionOf('New', [
        performed(9, [
          [80, 8],
          [80, 8],
        ]),
      ]),
    );
    const frontSquatKey = movementKeyFor('Front Squat', 'WeightedExerciseBlueprint');

    const history = readWeightedHistory(db, { movementKeys: [squatKey, frontSquatKey] });

    expect(describeSets(history.get(squatKey))).toEqual(['0: 80 kilograms x 8', '1: 80 kilograms x 8']);
    expect(describeSets(history.get(frontSquatKey))).toEqual(['0: 40 kilograms x 10']);
  });

  it('picks, at each index, a set completed latest at that index in the movement history', () => {
    const forget = (...sessions: Session[]) =>
      db.transaction((tx) => sessions.forEach((session) => deleteSession(tx, session.id)));
    const describeSet = (weight: Weight, reps: number) => `${weight.value.toString()} ${weight.unit} x ${reps}`;
    fc.assert(
      fc.property(fc.uniqueArray(SessionGenerator, { selector: (x) => x.id, maxLength: 4 }), (generated) => {
        const sessions = generated.map(canonical);
        store(...sessions);
        try {
          const movementKeys = [
            ...new Set(
              sessions
                .flatMap((x) => x.recordedExercises)
                .filter((x) => x instanceof RecordedWeightedExercise)
                .map((x) => x.movementKey()),
            ),
          ];
          const history = readWeightedHistory(db, { movementKeys });

          for (const movementKey of movementKeys) {
            const completed = readExerciseHistory(db, movementKey).flatMap((exercise) =>
              (exercise as RecordedWeightedExercise).potentialSets.flatMap((ps, index) =>
                ps.set
                  ? [{ index, at: ps.set.completionDateTime, text: describeSet(ps.weight, ps.set.repsCompleted) }]
                  : [],
              ),
            );
            const indices = [...new Set(completed.map((x) => x.index))].sort((a, b) => a - b);
            const selected = history.get(movementKey)?.sets;
            expect([...(selected?.keys() ?? [])].sort((a, b) => a - b)).toEqual(indices);
            for (const index of indices) {
              const atIndex = completed.filter((x) => x.index === index);
              const latest = atIndex.reduce((a, b) => (b.at.isAfter(a.at) ? b : a));
              const set = selected!.get(index)!;
              expect(atIndex.filter((x) => x.at.isEqual(latest.at)).map((x) => x.text)).toContain(
                describeSet(set.weight, set.reps),
              );
            }
          }
        } finally {
          forget(...sessions);
        }
      }),
    );
  });

  it('reads nothing for no movements, or for one never completed', () => {
    store(sessionOf('Day', [performed(9, [[100, undefined]])]));

    expect([...readWeightedHistory(db, { movementKeys: [squatKey] })]).toEqual([]);
    expect([...readWeightedHistory(db, { movementKeys: [] })]).toEqual([]);
  });
});

describe('readPreviousComparableSession', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));

  beforeEach(async () => {
    db = await createDb();
  });

  it('finds the latest session of the same name performed before the given time', () => {
    const current = sessionOf('Push', [squatAt(tickAt(12, 0))]);
    const previous = sessionOf('Push', [squatAt(tickAt(10, 0))]);
    store(
      current,
      sessionOf('Push', [squatAt(tickAt(9, 0))]),
      previous,
      sessionOf('Pull', [squatAt(tickAt(11, 0))]),
      sessionOf('Push', [squatAt(tickAt(13, 0))]),
    );

    const found = readPreviousComparableSession(db, { id: current.id, name: 'Push', before: tickAt(12, 0) });

    expect(found?.id).toBe(previous.id);
  });

  it('places a session with nothing completed at the start of its date', () => {
    const untouched = sessionOf('Push', [], LocalDate.of(2025, 4, 4));
    store(untouched, sessionOf('Push', [], LocalDate.of(2025, 4, 6)));

    const found = readPreviousComparableSession(db, {
      id: uuid(),
      name: 'Push',
      before: LocalDate.of(2025, 4, 5).atStartOfDay().atZone(ZoneId.systemDefault()).toOffsetDateTime(),
    });

    expect(found?.id).toBe(untouched.id);
  });
});

describe('reading previous performances across exercise types', () => {
  it('keeps a weighted and a cardio exercise of the same name apart', async () => {
    const db = await createDb();
    const cardioBlueprint = new CardioExerciseBlueprint('New Exercise', [CardioExerciseSetBlueprint.empty()], '', '');
    const cardioExercise = RecordedCardioExercise.empty(cardioBlueprint).with({
      sets: RecordedCardioExercise.empty(cardioBlueprint).sets.map((set) =>
        set.with({ completionDateTime: tickAt(10, 0), duration: Duration.ofSeconds(45) }),
      ),
    });
    db.transaction((tx) => writeSession(tx, sessionOf('Freeform Workout', [cardioExercise])));
    const weightedKey = movementKeyFor('New Exercise', 'WeightedExerciseBlueprint');
    const cardioKey = movementKeyFor('New Exercise', 'CardioExerciseBlueprint');

    const cardioLineage = cardioExercise.progressionKey();

    expect(readExerciseHistory(db, weightedKey)).toEqual([]);
    expect(readExerciseHistory(db, cardioKey)).toHaveLength(1);
    expect([...readWeightedHistory(db, { movementKeys: [weightedKey, cardioKey] })]).toEqual([]);
    expect(readPreviousCardioExercises(db, [cardioLineage], uuid())[cardioLineage]?.sets[0]?.duration?.seconds()).toBe(
      45,
    );
  });
});

describe('aggregating effective weight in SQL', () => {
  let db: ExpoSQLiteDatabase;
  const store = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => writeSession(tx, session)));
  const forget = (...sessions: Session[]) =>
    db.transaction((tx) => sessions.forEach((session) => deleteSession(tx, session.id)));
  const closeTo = (actual: number | undefined, expected: number) =>
    expect(Math.abs((actual ?? NaN) - expected)).toBeLessThanOrEqual(1e-9 * Math.max(1, Math.abs(expected)));
  const generatedSessions = fc
    .uniqueArray(SessionGenerator, { selector: (x) => x.id, maxLength: 4 })
    .map((sessions) => sessions.map(canonical));

  beforeEach(async () => {
    db = await createDb();
  });

  it('moves the same kilograms as sessionVolume, for every started session', () => {
    fc.assert(
      fc.property(generatedSessions, (sessions) => {
        store(...sessions);
        try {
          const volumes = new Map(readStartedSessionVolumes(db).map((x) => [x.id, x.volumeKg]));
          const started = sessions.filter((x) => x.isStarted);

          expect([...volumes.keys()].sort()).toEqual(started.map((x) => x.id).sort());
          started.forEach((session) => closeTo(volumes.get(session.id), sessionVolume(session)));
        } finally {
          forget(...sessions);
        }
      }),
    );
  });

  it('finds the same records as findPersonalRecords', () => {
    fc.assert(
      fc.property(generatedSessions, (sessions) => {
        store(...sessions);
        try {
          const expected = findPersonalRecords(
            [...sessions].sort((a, b) => TemporalComparer(getSessionReferenceTime(a), getSessionReferenceTime(b))),
          );

          expect(
            readPersonalRecords(db)
              .map((x) => `${x.sessionId}:${x.exerciseName}`)
              .sort(),
          ).toEqual([...expected].flatMap(([id, records]) => records.map((x) => `${id}:${x.exerciseName}`)).sort());
        } finally {
          forget(...sessions);
        }
      }),
    );
  });

  it('counts only the added weight of a bodyweight exercise when no bodyweight was recorded', () => {
    const pullUp = makeWeightedBlueprint({ name: 'Pull up', resistance: 'bodyweight' });
    const weighed = sessionOf('Weighed', [
      makeRecordedExercise(pullUp, [10], new Weight(22.0462, 'pounds'), () => tickAt(9, 0)),
    ]).with({ bodyweight: new Weight(80, 'kilograms') });
    const unweighed = sessionOf('Unweighed', [
      makeRecordedExercise(pullUp, [10], new Weight(10, 'kilograms'), () => tickAt(10, 0)),
    ]);
    store(weighed, unweighed);

    const volumes = new Map(readStartedSessionVolumes(db).map((x) => [x.id, x.volumeKg]));

    closeTo(volumes.get(weighed.id), 900);
    closeTo(volumes.get(unweighed.id), 100);
  });

  it('leaves the workout in progress out of both', () => {
    const before = sessionOf('Push', [squatAt(tickAt(9, 0), 100)]);
    const inProgress = sessionOf('Push', [squatAt(tickAt(10, 0), 200)]);
    store(before);
    db.transaction((tx) => setActiveSession(tx, inProgress));

    expect(readStartedSessionVolumes(db).map((x) => x.id)).toEqual([before.id]);
    expect(readPersonalRecords(db)).toEqual([]);
  });

  it('follows a session that is written again', () => {
    const before = sessionOf('Push', [squatAt(tickAt(9, 0), 100)]);
    const after = sessionOf('Push', [squatAt(tickAt(10, 0), 110)]);
    store(before, after);
    expect(readPersonalRecords(db).map((x) => x.sessionId)).toEqual([after.id]);

    store(after.with({ recordedExercises: [squatAt(tickAt(10, 0), 90)] }));
    expect(readPersonalRecords(db)).toEqual([]);
    closeTo(new Map(readStartedSessionVolumes(db).map((x) => [x.id, x.volumeKg])).get(after.id), 900);

    store(after.with({ recordedExercises: [makeRecordedExercise(makeWeightedBlueprint(), [undefined])] }));
    expect(readStartedSessionVolumes(db).map((x) => x.id)).toEqual([before.id]);
  });
});
