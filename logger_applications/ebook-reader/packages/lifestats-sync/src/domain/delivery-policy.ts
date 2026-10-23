/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { DeliveryProblem } from './problem';

/** What one request achieved, with HTTP details already classified away. */
export type AttemptOutcome =
  | { readonly kind: 'delivered' }
  | {
      readonly kind: 'retryable';
      readonly problem: DeliveryProblem;
      readonly retryAfterMs: number | null;
    }
  | { readonly kind: 'needs_fix'; readonly problem: DeliveryProblem };

export type FailedAttempt = Exclude<AttemptOutcome, { kind: 'delivered' }>;

export type DeliveryState =
  | { readonly kind: 'ready' }
  | {
      readonly kind: 'retrying';
      readonly attempts: number;
      readonly nextAttemptAt: number;
      readonly problem: DeliveryProblem;
    }
  | {
      /** Needs a fix. Retried after a new build, a config change, Retry now, or 6 hours. */
      readonly kind: 'parked';
      readonly problem: DeliveryProblem;
      readonly parkedAt: number;
      readonly buildId: string;
      readonly configRevision: number;
    };

/** What re-arms parked work and what time it is. */
export interface DeliveryContext {
  readonly now: number;
  readonly buildId: string;
  readonly configRevision: number;
}

export const BACKOFF_MS = [5_000, 15_000, 60_000, 300_000] as const;
export const PARKED_RETRY_MS = 6 * 3_600_000;

export const ready: DeliveryState = { kind: 'ready' };

export function nextDeliveryState(
  previous: DeliveryState,
  outcome: FailedAttempt,
  context: DeliveryContext,
  random: () => number
): DeliveryState {
  if (outcome.kind === 'needs_fix') {
    return {
      kind: 'parked',
      problem: outcome.problem,
      parkedAt: context.now,
      buildId: context.buildId,
      configRevision: context.configRevision
    };
  }
  const attempts = previous.kind === 'retrying' ? previous.attempts + 1 : 1;
  const backoff = BACKOFF_MS[Math.min(attempts, BACKOFF_MS.length) - 1];
  const jittered = backoff * (0.8 + 0.4 * random());
  return {
    kind: 'retrying',
    attempts,
    nextAttemptAt: context.now + Math.max(outcome.retryAfterMs ?? 0, jittered),
    problem: outcome.problem
  };
}

export function isDue(state: DeliveryState, context: DeliveryContext): boolean {
  switch (state.kind) {
    case 'ready':
      return true;
    case 'retrying':
      return state.nextAttemptAt <= context.now;
    case 'parked':
      return (
        state.buildId !== context.buildId ||
        state.configRevision !== context.configRevision ||
        context.now - state.parkedAt >= PARKED_RETRY_MS
      );
    default: {
      const _exhaustive: never = state;
      return _exhaustive;
    }
  }
}
