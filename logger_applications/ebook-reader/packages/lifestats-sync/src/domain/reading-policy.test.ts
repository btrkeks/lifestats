/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { describe, expect, it } from 'vitest';

import { emptyTrail, observe, release, relocate, type Step, type Trail } from './reading-policy';
import { SpanSet } from './span-set';

const policy = { forwardSkipThreshold: 2700 };
const second = 1000;

/** Replays [kind, position, seconds] and returns every credited span merged. */
function replay(moves: ['observe' | 'relocate' | 'release', number, number][], start = emptyTrail) {
  let trail: Trail = start;
  let credited = SpanSet.empty;
  const steps: Step[] = [];
  for (const [kind, position, seconds] of moves) {
    if (kind === 'release') {
      trail = release(trail);
      continue;
    }
    const step = (kind === 'observe' ? observe : relocate)(
      trail,
      position,
      seconds * second,
      policy
    );
    if (step.kind === 'read') credited = credited.union(step.spans);
    steps.push(step);
    trail = step.trail;
  }
  return { credited, steps, trail };
}

describe('reading policy', () => {
  it('credits a page turn after enough reading time', () => {
    const { credited } = replay([
      ['observe', 1000, 0],
      ['observe', 1600, 60]
    ]);
    expect(credited.toPairs()).toEqual([[1000, 1600]]);
  });

  it('anchors at the first position without crediting the bookmark', () => {
    const { credited, steps } = replay([['observe', 85900, 0]]);
    expect(credited.size).toBe(0);
    expect(steps[0]).toMatchObject({ kind: 'moved', reason: 'anchored' });
  });

  it('never credits a skip, even after dwelling at the target', () => {
    const { credited } = replay([
      ['observe', 0, 0],
      ['observe', 100000, 60],
      ['observe', 100000, 75],
      ['observe', 100000, 600]
    ]);
    expect(credited.size).toBe(0);
  });

  it('credits a step at the skip threshold and skips one character more', () => {
    expect(
      replay([
        ['observe', 0, 0],
        ['observe', 2700, 600]
      ]).credited.size
    ).toBe(2700);
    expect(
      replay([
        ['observe', 0, 0],
        ['observe', 2701, 600]
      ]).credited.size
    ).toBe(0);
  });

  it('limits held page keys to the banked budget plus the speed cap', () => {
    const moves: ['observe', number, number][] = [['observe', 0, 0]];
    for (let turn = 1; turn <= 20; turn += 1) moves.push(['observe', 600 * turn, 60 + turn / 2]);
    const { credited } = replay(moves);
    expect(credited.size).toBeLessThanOrEqual(2700 + 10 * 40);
    expect(credited.size).toBeGreaterThan(0);
  });

  it('credits nothing for backward movement, then reading forward from there', () => {
    const { credited } = replay([
      ['observe', 5000, 0],
      ['observe', 4000, 30],
      ['observe', 4500, 60]
    ]);
    expect(credited.toPairs()).toEqual([[4000, 4500]]);
  });

  it('never credits a relocation, and reading continues from the new anchor', () => {
    const { credited } = replay([
      ['observe', 0, 0],
      ['relocate', 2000, 60],
      ['observe', 2500, 90]
    ]);
    expect(credited.toPairs()).toEqual([[2000, 2500]]);
  });

  it('re-anchors without credit after the page was hidden', () => {
    const { credited } = replay([
      ['observe', 0, 0],
      ['observe', 500, 30],
      ['release', 0, 0],
      ['observe', 1500, 300],
      ['observe', 1800, 330]
    ]);
    expect(credited.toPairs()).toEqual([
      [0, 500],
      [1500, 1800]
    ]);
  });

  it('does not refill the budget from time spent hidden', () => {
    const { credited } = replay([
      ['observe', 0, 0],
      ['release', 0, 0],
      ['observe', 0, 3600],
      ['observe', 600, 3601]
    ]);
    expect(credited.size).toBe(0);
  });

  it('treats a clock set back as no elapsed time', () => {
    const { trail } = replay([
      ['observe', 0, 3600],
      ['observe', 100, 0]
    ]);
    expect(trail.budget).toBe(0);
  });
});
