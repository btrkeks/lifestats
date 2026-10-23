/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import fc from 'fast-check';
import { describe, expect, it } from 'vitest';

import { absorb, acknowledge, isPending, type DayObservation, type DayRecord } from './day-record';
import type { BookKey, ReadingDay, ServerId } from './ids';
import { SpanSet } from './span-set';

const book = `ttu-text-v1:${'b'.repeat(64)}` as BookKey;
const day = '2026-10-22' as ReadingDay;
const serverA = '11111111-1111-4111-8111-111111111111' as ServerId;
const serverB = '22222222-2222-4222-8222-222222222222' as ServerId;

function observation(start: number, end: number, atMs: number): DayObservation {
  return {
    book,
    day,
    title: '猫',
    length: 10000,
    spans: SpanSet.span(start, end),
    position: end,
    atMs
  };
}

function record(...observations: DayObservation[]): DayRecord {
  let current: DayRecord | undefined;
  for (const o of observations) current = absorb(current, o) ?? current;
  if (!current) throw new Error('no record');
  return current;
}

describe('absorb', () => {
  it('writes nothing when the same observation arrives twice', () => {
    const first = record(observation(0, 500, 1));
    expect(absorb(first, observation(0, 500, 1))).toBeUndefined();
  });

  it('writes nothing for an older observation inside what is stored', () => {
    const stored = record(observation(0, 500, 1), observation(500, 900, 2));
    expect(absorb(stored, observation(100, 300, 1))).toBeUndefined();
  });

  it('unions overlapping reading and bumps the revision once per change', () => {
    const stored = record(
      observation(0, 500, 1),
      observation(300, 800, 2),
      observation(300, 800, 2)
    );
    expect(stored.spans.toPairs()).toEqual([[0, 800]]);
    expect(stored.revision).toBe(2);
  });

  it('moves the position for a newer observation that adds no spans', () => {
    const stored = record(observation(0, 500, 1));
    const moved = absorb(stored, { ...observation(0, 0, 2), position: 200 });
    expect(moved).toMatchObject({ lastPosition: 200, observedAt: 2, revision: 2 });
    expect(moved?.spans.toPairs()).toEqual([[0, 500]]);
  });

  it('re-arms a parked record when new reading arrives', () => {
    const parked: DayRecord = {
      ...record(observation(0, 500, 1)),
      delivery: {
        kind: 'parked',
        problem: { kind: 'server_too_old', serverUrl: 'http://x' },
        parkedAt: 1,
        buildId: 'b',
        configRevision: 1
      }
    };
    expect(absorb(parked, observation(500, 600, 2))?.delivery).toEqual({ kind: 'ready' });
  });
});

describe('delivery acknowledgement', () => {
  it('keeps the highest acknowledged revision in any reply order', () => {
    fc.assert(
      fc.property(fc.array(fc.integer({ min: 1, max: 50 }), { minLength: 1 }), (replies) => {
        let current: DayRecord = { ...record(observation(0, 1, 1)), revision: 50 };
        for (const revision of replies) current = acknowledge(current, revision, serverA);
        expect(current.delivered).toEqual({ revision: Math.max(...replies), serverId: serverA });
      })
    );
  });

  it('stays pending until the current revision is acknowledged by the current server', () => {
    const stored = record(observation(0, 500, 1), observation(500, 600, 2));
    expect(isPending(stored, serverA)).toBe(true);
    expect(isPending(acknowledge(stored, 1, serverA), serverA)).toBe(true);
    const delivered = acknowledge(stored, 2, serverA);
    expect(isPending(delivered, serverA)).toBe(false);
    expect(isPending(delivered, serverB)).toBe(true);
    expect(isPending(delivered, null)).toBe(true);
  });
});
