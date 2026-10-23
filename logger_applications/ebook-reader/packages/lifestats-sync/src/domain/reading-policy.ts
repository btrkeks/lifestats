/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { SpanSet } from './span-set';

/** About 2.5 times fast native Japanese reading. Holding a page key cannot outrun it. */
export const READING_SPEED_CAP = 40;

export interface ReadingPolicy {
  /** ttu's `trackerForwardSkipThreshold`. A single larger step is a skip and never counts. */
  readonly forwardSkipThreshold: number;
}

/** Per-session position trail. Lives in memory only. */
export interface Trail {
  readonly anchor: { readonly position: number; readonly atMs: number } | null;
  /** Characters that may still be credited, refilled by elapsed time up to the skip threshold. */
  readonly budget: number;
}

export const emptyTrail: Trail = { anchor: null, budget: 0 };

export type Step =
  | { readonly kind: 'read'; readonly spans: SpanSet; readonly trail: Trail }
  | {
      readonly kind: 'moved';
      readonly reason: 'anchored' | 'backward' | 'skip' | 'too_fast';
      readonly trail: Trail;
    };

function refilled(trail: Trail, atMs: number, policy: ReadingPolicy): number {
  if (!trail.anchor) return Math.min(trail.budget, policy.forwardSkipThreshold);
  const elapsedSeconds = Math.max(0, atMs - trail.anchor.atMs) / 1000;
  return Math.min(policy.forwardSkipThreshold, trail.budget + elapsedSeconds * READING_SPEED_CAP);
}

/** User-driven movement: credits a forward step that is neither a skip nor faster than the cap. */
export function observe(trail: Trail, position: number, atMs: number, policy: ReadingPolicy): Step {
  if (!trail.anchor) return relocate(trail, position, atMs, policy);
  const budget = refilled(trail, atMs, policy);
  const from = trail.anchor.position;
  const moved = (reason: 'backward' | 'skip' | 'too_fast'): Step => ({
    kind: 'moved',
    reason,
    trail: { anchor: { position, atMs }, budget }
  });
  const delta = position - from;
  if (delta <= 0) return moved('backward');
  if (delta > policy.forwardSkipThreshold) return moved('skip');
  if (delta > budget) return moved('too_fast');
  return {
    kind: 'read',
    spans: SpanSet.span(from, position),
    trail: { anchor: { position, atMs }, budget: budget - delta }
  };
}

/** Programmatic movement (open, restore, resize, navigation): moves the anchor, never credits. */
export function relocate(
  trail: Trail,
  position: number,
  atMs: number,
  policy: ReadingPolicy
): Step {
  return {
    kind: 'moved',
    reason: 'anchored',
    trail: { anchor: { position, atMs }, budget: refilled(trail, atMs, policy) }
  };
}

/** Drops the anchor and keeps the budget, until `relocate` sets a new anchor. */
export function release(trail: Trail): Trail {
  return { anchor: null, budget: trail.budget };
}
