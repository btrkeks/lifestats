import type { RestWindow } from '@/models/session-models';

export type RestPhase = 'resting' | 'ready' | 'over';

export interface RestBounds {
  windowStart: number;
  windowEnd: number | undefined;
}

export type RestTimerState = {
  restFractionLeft: number;
  windowFractionLeft: number;
} & ({ phase: 'resting' | 'ready'; leftMs: number } | { phase: 'over'; overtimeMs: number });

export function restBounds(rest: RestWindow): RestBounds {
  return {
    windowStart: rest.minRest.toMillis(),
    windowEnd: rest.minRest.equals(rest.maxRest) ? undefined : rest.maxRest.toMillis(),
  };
}

export function getRestTimerState(rest: RestWindow, elapsedMs: number): RestTimerState {
  const { windowStart, windowEnd } = restBounds(rest);

  if (elapsedMs < windowStart) {
    return {
      phase: 'resting',
      leftMs: windowStart - elapsedMs,
      restFractionLeft: 1 - elapsedMs / windowStart,
      windowFractionLeft: 1,
    };
  }
  if (windowEnd !== undefined && elapsedMs < windowEnd) {
    return {
      phase: 'ready',
      leftMs: windowEnd - elapsedMs,
      restFractionLeft: 0,
      windowFractionLeft: 1 - (elapsedMs - windowStart) / (windowEnd - windowStart),
    };
  }
  return {
    phase: 'over',
    overtimeMs: elapsedMs - (windowEnd ?? windowStart),
    restFractionLeft: 0,
    windowFractionLeft: 0,
  };
}
