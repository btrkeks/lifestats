/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import fc from 'fast-check';
import { describe, expect, it } from 'vitest';

import cases from '../../contract/span-cases.json';
import { SpanSet } from './span-set';

function set(pairs: unknown): SpanSet {
  const parsed = SpanSet.fromPairs(pairs);
  if (!parsed) throw new Error(`fixture pairs rejected: ${JSON.stringify(pairs)}`);
  return parsed;
}

const spans = fc
  .array(fc.tuple(fc.nat(200), fc.nat(40)), { maxLength: 8 })
  .map((pairs) => set(pairs.map(([start, length]) => [start, start + length])));

function members(s: SpanSet): Set<number> {
  return new Set(
    s.toPairs().flatMap(([start, end]) => [...Array(end - start).keys()].map((i) => start + i))
  );
}

describe('SpanSet against the shared server fixture', () => {
  it('uses fixture version 1', () => {
    expect(cases.version).toBe(1);
  });

  it.each(cases.normalization)('normalizes $input', ({ input, normalized, size }) => {
    const parsed = set(input);
    expect(parsed.toPairs()).toEqual(normalized);
    expect(parsed.size).toBe(size);
  });

  it.each(cases.union)('unions $a and $b', ({ a, b, union, size }) => {
    expect(set(a).union(set(b)).toPairs()).toEqual(union);
    expect(set(b).union(set(a)).size).toBe(size);
  });
});

describe('SpanSet algebra', () => {
  it('is commutative, associative and idempotent', () => {
    fc.assert(
      fc.property(spans, spans, spans, (a, b, c) => {
        expect(a.union(b).equals(b.union(a))).toBe(true);
        expect(
          a
            .union(b)
            .union(c)
            .equals(a.union(b.union(c)))
        ).toBe(true);
        expect(a.union(a).equals(a)).toBe(true);
      })
    );
  });

  it('contains exactly the members of both operands', () => {
    fc.assert(
      fc.property(spans, spans, (a, b) => {
        const expected = new Set([...members(a), ...members(b)]);
        expect(members(a.union(b))).toEqual(expected);
        expect(a.union(b).size).toBe(expected.size);
      })
    );
  });

  it('reports coverage like member inclusion', () => {
    fc.assert(
      fc.property(spans, spans, (a, b) => {
        const inside = [...members(b)].every((n) => members(a).has(n));
        expect(a.covers(b)).toBe(inside);
        expect(a.union(b).covers(a) && a.union(b).covers(b)).toBe(true);
      })
    );
  });

  it('produces only sorted, disjoint, non-adjacent spans, as the server requires', () => {
    fc.assert(
      fc.property(spans, spans, (a, b) => {
        const pairs = a.union(b).toPairs();
        pairs.forEach(([start, end], i) => {
          expect(start).toBeLessThan(end);
          if (i > 0) expect(start).toBeGreaterThan(pairs[i - 1][1]);
        });
      })
    );
    expect(
      set([
        [0, 5],
        [5, 10]
      ]).toPairs()
    ).toEqual([[0, 10]]);
  });

  it('round-trips through pairs', () => {
    fc.assert(
      fc.property(spans, (a) => {
        expect(set(a.toPairs()).equals(a)).toBe(true);
      })
    );
  });

  it.each([[[[-1, 3]]], [[[3, 1]]], [[[0.5, 2]]], [[[0, 2 ** 53 + 2]]], [[[0, 1, 2]]], ['0,1']])(
    'rejects %j',
    (pairs) => {
      expect(SpanSet.fromPairs(pairs)).toBeUndefined();
    }
  );
});
