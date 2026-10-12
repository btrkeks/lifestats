import { Session } from '@/models/session-models';
import { toDurationJSON, toInstantJson } from '@/models/storage/versions/latest';
import { CardioTimerInfo, CurrentExerciseDetails, RestTimerInfo } from '@/models/workout-worker-messages';
import { Duration } from '@js-joda/core';

export function workoutUpdatedEvent(session: Session, restTimersEnabled: boolean) {
  return {
    type: 'WorkoutUpdatedEvent',
    workout: session.toJSON(),
    restTimerInfo: restTimersEnabled ? getTimerInfo(session) : undefined,
    cardioTimerInfo: getCardioTimerInfo(session),
    currentExerciseDetails: getCurrentExerciseDetails(session),
    totalWeightLifted: session.totalWeightLifted.toJSON(),
    workoutDuration: toDurationJSON(session.duration ?? Duration.ZERO),
  } as const;
}

export function getCardioTimerInfo(session: Session): CardioTimerInfo | undefined {
  const running = session.runningCardioSet;
  if (!running) {
    return undefined;
  }

  const { set, exerciseIndex, setIndex } = running;
  return {
    currentBlockStartTime: toInstantJson(set.currentBlockStartTime?.toInstant()),
    // The notification anchors its clock at `currentBlockStartTime - currentDuration`, so this must
    // be the set's own banked time and not the exercise's running total.
    currentDuration: toDurationJSON(set.duration ?? Duration.ZERO),
    exerciseIndex,
    setIndex,
  };
}

export function getCurrentExerciseDetails(session: Session): CurrentExerciseDetails | undefined {
  return session.nextExercise
    ? {
        exercise: session.nextExercise.toJSON(),
        setIndex: session.nextExercise.currentSetIndex,
      }
    : undefined;
}

export function getTimerInfo(session: Session): RestTimerInfo | undefined {
  const lastExercise = session.lastExercise;
  const nextExercise = session.nextExercise;
  if (!session.restTimer || session.restTimer.isPaused || !lastExercise || !nextExercise) {
    return undefined;
  }

  const rest = session.restWindow;
  if (!rest || rest.minRest.equals(Duration.ZERO)) {
    return;
  }
  return {
    startedAt: toInstantJson(session.restTimer.startedAt.toInstant()),
    partiallyEndAt: toInstantJson(session.restTimer.startedAt.plus(rest.minRest).toInstant()),
    endAt: toInstantJson(session.restTimer.startedAt.plus(rest.maxRest).toInstant()),
  };
}
