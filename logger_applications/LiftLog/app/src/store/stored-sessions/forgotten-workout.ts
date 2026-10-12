import { Session } from '@/models/session-models';
import { Duration, OffsetDateTime } from '@js-joda/core';

/** Longer than any real workout, so a workout idle this long was left open by mistake. */
export const FORGOTTEN_WORKOUT_IDLE_LIMIT = Duration.ofHours(3);

export type ForgottenWorkoutAction = 'finish' | 'discard' | 'keep';

/**
 * What to do with the workout in progress when the app opens or returns: finish one that has been idle past
 * the limit, discard one from an earlier day that never recorded a set, and leave anything else alone.
 */
export function forgottenWorkoutAction(session: Session, now: OffsetDateTime): ForgottenWorkoutAction {
  const lastSet = session.lastExercise?.latestTime;
  if (!lastSet) {
    return session.date.isBefore(now.toLocalDate()) ? 'discard' : 'keep';
  }
  // A running cardio clock is re-anchored while it is on screen, so it shows the user was still there.
  const timerStart = session.runningCardioSet?.set.currentBlockStartTime;
  const lastActivity = timerStart?.isAfter(lastSet) ? timerStart : lastSet;
  return Duration.between(lastActivity, now).compareTo(FORGOTTEN_WORKOUT_IDLE_LIMIT) > 0 ? 'finish' : 'keep';
}
