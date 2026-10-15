import type { RestWindow } from '@/models/session-models';

export type RestPhase = 'resting' | 'ready' | 'over';

export interface RestTimerState {
  phase: RestPhase;
  windowStart: number;
  /** Undefined for a fixed rest (min === max): a target rather than a window, so there is no second segment. */
  windowEnd: number | undefined;
  /** Left until the end of the phase, or, once rest is over, how far past it the timer has run. */
  timeMs: number;
  /** The fraction of the rest segment still to run. */
  restLeft: number;
  /** The fraction of the min-to-max window still to run. */
  windowLeft: number;
}

export function getRestTimerState(rest: RestWindow, elapsedMs: number): RestTimerState {
  const windowStart = rest.minRest.toMillis();
  const windowEnd = rest.minRest.equals(rest.maxRest) ? undefined : rest.maxRest.toMillis();

  if (elapsedMs < windowStart) {
    return {
      phase: 'resting',
      windowStart,
      windowEnd,
      timeMs: windowStart - elapsedMs,
      restLeft: 1 - elapsedMs / windowStart,
      windowLeft: 1,
    };
  }
  if (windowEnd !== undefined && elapsedMs < windowEnd) {
    return {
      phase: 'ready',
      windowStart,
      windowEnd,
      timeMs: windowEnd - elapsedMs,
      restLeft: 0,
      windowLeft: 1 - (elapsedMs - windowStart) / (windowEnd - windowStart),
    };
  }
  return {
    phase: 'over',
    windowStart,
    windowEnd,
    timeMs: elapsedMs - (windowEnd ?? windowStart),
    restLeft: 0,
    windowLeft: 0,
  };
}
