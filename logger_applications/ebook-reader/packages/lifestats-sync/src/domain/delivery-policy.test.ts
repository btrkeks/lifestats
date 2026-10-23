/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { describe, expect, it } from 'vitest';

import {
  PARKED_RETRY_MS,
  isDue,
  nextDeliveryState,
  ready,
  type DeliveryContext,
  type DeliveryState
} from './delivery-policy';

const context: DeliveryContext = { now: 1_000_000, buildId: 'build-1', configRevision: 3 };
const unreachable = {
  kind: 'retryable',
  problem: { kind: 'unreachable', serverUrl: 'http://laptop:3000' },
  retryAfterMs: null
} as const;
const middle = () => 0.5;

describe('nextDeliveryState', () => {
  it('backs off 5 s, 15 s, 60 s, then 300 s for retryable failures', () => {
    let state: DeliveryState = ready;
    const delays: number[] = [];
    for (let i = 0; i < 6; i += 1) {
      state = nextDeliveryState(state, unreachable, context, middle);
      if (state.kind !== 'retrying') throw new Error('expected retrying');
      delays.push(state.nextAttemptAt - context.now);
    }
    expect(delays).toEqual([5_000, 15_000, 60_000, 300_000, 300_000, 300_000]);
  });

  it('keeps jitter within 20 percent', () => {
    const low = nextDeliveryState(ready, unreachable, context, () => 0);
    const high = nextDeliveryState(ready, unreachable, context, () => 1);
    expect(low).toMatchObject({ nextAttemptAt: context.now + 4_000 });
    expect(high).toMatchObject({ nextAttemptAt: context.now + 6_000 });
  });

  it('waits at least Retry-After', () => {
    const state = nextDeliveryState(
      ready,
      { ...unreachable, retryAfterMs: 120_000 },
      context,
      middle
    );
    expect(state).toMatchObject({ kind: 'retrying', nextAttemptAt: context.now + 120_000 });
  });

  it('parks a rejection with what re-arms it', () => {
    const problem = {
      kind: 'rejected',
      httpStatus: 422,
      code: 'invalid_field',
      field: 'spans',
      message: 'Invalid spans'
    } as const;
    expect(nextDeliveryState(ready, { kind: 'needs_fix', problem }, context, middle)).toEqual({
      kind: 'parked',
      problem,
      parkedAt: context.now,
      buildId: 'build-1',
      configRevision: 3
    });
  });
});

describe('isDue', () => {
  const parked = nextDeliveryState(
    ready,
    { kind: 'needs_fix', problem: { kind: 'server_too_old', serverUrl: 'http://x' } },
    context,
    middle
  );

  it('holds a parked record until a build, config or 6 h change', () => {
    expect(isDue(parked, context)).toBe(false);
    expect(isDue(parked, { ...context, now: context.now + PARKED_RETRY_MS - 1 })).toBe(false);
    expect(isDue(parked, { ...context, now: context.now + PARKED_RETRY_MS })).toBe(true);
    expect(isDue(parked, { ...context, buildId: 'build-2' })).toBe(true);
    expect(isDue(parked, { ...context, configRevision: 4 })).toBe(true);
  });

  it('holds a retrying record until its next attempt', () => {
    const retrying = nextDeliveryState(ready, unreachable, context, middle);
    expect(isDue(retrying, { ...context, now: context.now + 4_999 })).toBe(false);
    expect(isDue(retrying, { ...context, now: context.now + 5_000 })).toBe(true);
    expect(isDue(ready, context)).toBe(true);
  });
});
