import { describe, expect, it } from 'vitest';
import { LocalDate, OffsetDateTime } from '@js-joda/core';
import { SessionBlueprint } from '@/models/blueprint-models';
import { RecordedCardioExercise, Session } from '@/models/session-models';
import {
  makeCardioBlueprint,
  makeRecordedExercise,
  makeWeightedBlueprint,
} from '@/models/session-models/__test__/helpers';
import { forgottenWorkoutAction } from '@/store/stored-sessions/forgotten-workout';

const at = (iso: string) => OffsetDateTime.parse(iso);

function workout(date: string, setTimes: string[]): Session {
  const blueprint = makeWeightedBlueprint();
  const sets = setTimes.length ? setTimes.map(() => 5) : [undefined];
  return new Session(
    'workout',
    new SessionBlueprint('DAY 1', [blueprint], ''),
    [makeRecordedExercise(blueprint, sets, undefined, (i) => at(setTimes[i]!))],
    LocalDate.parse(date),
    undefined,
    undefined,
  );
}

describe('forgottenWorkoutAction', () => {
  it('keeps a workout whose last set was 2 h 59 min ago', () => {
    const session = workout('2026-10-10', ['2026-10-10T17:30:00+02:00', '2026-10-10T18:00:00+02:00']);
    expect(forgottenWorkoutAction(session, at('2026-10-10T20:59:00+02:00'))).toBe('keep');
  });

  it('finishes a workout whose last set was 3 h 1 min ago', () => {
    const session = workout('2026-10-10', ['2026-10-10T17:30:00+02:00', '2026-10-10T18:00:00+02:00']);
    expect(forgottenWorkoutAction(session, at('2026-10-10T21:01:00+02:00'))).toBe('finish');
  });

  it('keeps a late workout checked just after midnight', () => {
    const session = workout('2026-10-10', ['2026-10-10T23:30:00+02:00']);
    expect(forgottenWorkoutAction(session, at('2026-10-11T01:00:00+02:00'))).toBe('keep');
  });

  it('discards a workout from yesterday with no completed set', () => {
    expect(forgottenWorkoutAction(workout('2026-10-10', []), at('2026-10-11T08:00:00+02:00'))).toBe('discard');
  });

  it('keeps a workout from today with no completed set', () => {
    expect(forgottenWorkoutAction(workout('2026-10-11', []), at('2026-10-11T23:00:00+02:00'))).toBe('keep');
  });

  it('keeps a workout whose cardio clock was still running 2 h ago, however old its last set', () => {
    const blueprint = makeCardioBlueprint();
    const cardio = RecordedCardioExercise.empty(blueprint).withSet(0, (set) =>
      set.with({
        completionDateTime: at('2026-10-10T17:00:00+02:00'),
        currentBlockStartTime: at('2026-10-10T19:00:00+02:00'),
      }),
    );
    const session = new Session(
      'cardio',
      new SessionBlueprint('Cardio', [blueprint], ''),
      [cardio],
      LocalDate.parse('2026-10-10'),
      undefined,
      undefined,
    );

    expect(forgottenWorkoutAction(session, at('2026-10-10T21:00:00+02:00'))).toBe('keep');
    expect(forgottenWorkoutAction(session, at('2026-10-10T22:01:00+02:00'))).toBe('finish');
  });
});
