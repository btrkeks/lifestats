import type { RestWindow } from '@/models/session-models';

export type RestPhase = 'resting' | 'ready' | 'over';

export interface RestBounds {
  windowStart: number;
  /** Undefined for a fixed rest (min === max): a target rather than a window, so there is no second segment. */
  windowEnd: number | undefined;
}

export interface RestTimerState {
  phase: RestPhase;
  /** Left until the end of the phase, or, once rest is over, how far past it the timer has run. */
  timeMs: number;
  /** The fraction of the rest segment still to run. */
  restLeft: number;
  /** The fraction of the min-to-max window still to run. */
  windowLeft: number;
}

export function restBounds(rest: RestWindow): RestBounds {
  return {
    windowStart: rest.minRest.toMillis(),
    windowEnd: rest.minRest.equals(rest.maxRest) ? undefined : rest.maxRest.toMillis(),
  };
}

export function getRestTimerState(rest: RestWindow, elapsedMs: number): RestTimerState {
  const { windowStart, windowEnd } = restBounds(rest);

  if (elapsedMs < windowStart) {
    return { phase: 'resting', timeMs: windowStart - elapsedMs, restLeft: 1 - elapsedMs / windowStart, windowLeft: 1 };
  }
  if (windowEnd !== undefined && elapsedMs < windowEnd) {
    return {
      phase: 'ready',
      timeMs: windowEnd - elapsedMs,
      restLeft: 0,
      windowLeft: 1 - (elapsedMs - windowStart) / (windowEnd - windowStart),
    };
  }
  return { phase: 'over', timeMs: elapsedMs - (windowEnd ?? windowStart), restLeft: 0, windowLeft: 0 };
}
