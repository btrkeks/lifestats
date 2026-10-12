import { describe, it, expect } from 'vitest';
import { Duration, LocalDate, OffsetDateTime } from '@js-joda/core';
import { SessionBlueprint } from '@/models/blueprint-models';
import { Session } from '@/models/session-models/session';
import { RestTimer } from '@/models/session-models/rest-timer';
import { RecordedCardioExercise } from '@/models/session-models/recorded-cardio-exercise';
import { RecordedWeightedExercise } from '@/models/session-models/recorded-weighted-exercise';
import {
  emptyPotentialSet,
  filledPotentialSet,
  makeCardioBlueprint,
  makeSession,
  makeLegacyWeightedBlueprint,
  makeRecordedExercise,
  makeWeightedBlueprint,
  tick,
} from '@/models/session-models/__test__/helpers';
import { getCardioTimerInfo, getCurrentExerciseDetails, getTimerInfo } from '@/store/workout-worker/helpers';
import { uuid } from '@/utils/uuid';

describe('getCardioTimerInfo', () => {
  it('returns undefined when no cardio set has a running timer', () => {
    const session = makeSession([makeCardioBlueprint(1)]);
    expect(getCardioTimerInfo(session)).toBeUndefined();
  });

  it('reports the exercise and set index of the running timer', () => {
    const cardio = RecordedCardioExercise.empty(makeCardioBlueprint(2)).withSet(1, (s) =>
      s.with({ currentBlockStartTime: tick() }),
    );
    const session = new Session(
      uuid(),
      new SessionBlueprint('Test', [cardio.blueprint], ''),
      [cardio],
      LocalDate.of(2025, 4, 5),
      undefined,
      undefined,
    );

    const info = getCardioTimerInfo(session)!;

    expect(info.exerciseIndex).toBe(0);
    expect(info.setIndex).toBe(1);
    expect(info.currentBlockStartTime).toBeDefined();
  });
});

describe('getCurrentExerciseDetails', () => {
  it('returns undefined when there is no next exercise', () => {
    const session = makeSession([]);
    expect(getCurrentExerciseDetails(session)).toBeUndefined();
  });

  it('returns the serialized next exercise and its current set index', () => {
    const session = makeSession([makeWeightedBlueprint({ name: 'Squat' })]);
    const details = getCurrentExerciseDetails(session)!;
    expect(details.setIndex).toBe(0);
    expect(details.exercise).toBeDefined();
  });
});

describe('getTimerInfo', () => {
  function sessionWithRestTimer(reps: number, start = tick()) {
    const bp = makeWeightedBlueprint();
    const exercise = new RecordedWeightedExercise(
      bp,
      [filledPotentialSet(reps, tick()), emptyPotentialSet(100)],
      undefined,
    );
    return new Session(
      uuid(),
      new SessionBlueprint('Test', [bp], ''),
      [exercise],
      LocalDate.of(2025, 4, 5),
      undefined,
      new RestTimer(start),
    );
  }

  it('returns undefined without a running rest timer', () => {
    const bp = makeWeightedBlueprint();
    const exercise = new RecordedWeightedExercise(bp, [filledPotentialSet(10, tick())], undefined);
    const session = new Session(
      uuid(),
      new SessionBlueprint('Test', [bp], ''),
      [exercise],
      LocalDate.of(2025, 4, 5),
      undefined,
      undefined,
    );
    expect(getTimerInfo(session)).toBeUndefined();
  });

  it('returns partial and full rest times after a successful set', () => {
    const info = getTimerInfo(sessionWithRestTimer(10))!;
    expect(info.startedAt).toBeDefined();
    expect(info.partiallyEndAt).toBeDefined();
    expect(info.endAt).toBeDefined();
  });

  it('rests from the minimum to the maximum after any set, however short of an old target it fell', () => {
    const bp = makeLegacyWeightedBlueprint({
      targets: [
        { min: 10, max: 10 },
        { min: 10, max: 10 },
      ],
    }).with({
      restBetweenSets: {
        minRest: Duration.ofSeconds(60),
        maxRest: Duration.ofSeconds(90),
        failureRest: Duration.ofSeconds(180),
      },
    });
    const exercise = makeRecordedExercise(bp, [2, undefined]);
    const session = new Session(
      uuid(),
      new SessionBlueprint('Test', [bp], ''),
      [exercise],
      LocalDate.of(2025, 4, 5),
      undefined,
      new RestTimer(OffsetDateTime.parse('2025-04-05T12:00:00Z')),
    );

    expect(getTimerInfo(session)).toEqual({
      startedAt: '2025-04-05T12:00:00Z',
      partiallyEndAt: '2025-04-05T12:01:00Z',
      endAt: '2025-04-05T12:01:30Z',
    });
  });
});
