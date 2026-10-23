/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

/** The server's largest accepted offset. */
export const MAX_OFFSET = 2 ** 53;

function isOffset(value: unknown): value is number {
  return Number.isInteger(value) && (value as number) >= 0 && (value as number) <= MAX_OFFSET;
}

/**
 * Character ranges [start, end) of one book. Normalized by construction: sorted, disjoint,
 * and non-adjacent, so equal sets have one representation. Union is commutative,
 * associative and idempotent, which is why retries, overlapping commits and two tabs
 * cannot count a character twice.
 */
export class SpanSet {
  /** Flat [s0, e0, s1, e1, ...] with s0 < e0 < s1 < e1 < ... */
  private constructor(private readonly bounds: readonly number[]) {}

  static readonly empty = new SpanSet([]);

  static span(start: number, end: number): SpanSet {
    return end > start ? new SpanSet([start, end]) : SpanSet.empty;
  }

  /** Storage and fixture boundary. Returns undefined unless every pair is [start <= end] in range. */
  static fromPairs(pairs: unknown): SpanSet | undefined {
    if (!Array.isArray(pairs)) return undefined;
    const spans: [number, number][] = [];
    for (const pair of pairs) {
      if (!Array.isArray(pair) || pair.length !== 2) return undefined;
      const [start, end] = pair;
      if (!isOffset(start) || !isOffset(end) || end < start) return undefined;
      if (end > start) spans.push([start, end]);
    }
    spans.sort((a, b) => a[0] - b[0]);
    const bounds: number[] = [];
    for (const [start, end] of spans) {
      const last = bounds.length - 1;
      if (last > 0 && start <= bounds[last]) {
        bounds[last] = Math.max(bounds[last], end);
      } else {
        bounds.push(start, end);
      }
    }
    return new SpanSet(bounds);
  }

  union(other: SpanSet): SpanSet {
    if (other.isEmpty || this.covers(other)) return this;
    if (this.isEmpty || other.covers(this)) return other;
    return SpanSet.fromPairs([...this.toPairs(), ...other.toPairs()]) ?? SpanSet.empty;
  }

  /** Whether every character of `other` is already in this set. */
  covers(other: SpanSet): boolean {
    let i = 0;
    for (let j = 0; j < other.bounds.length; j += 2) {
      while (i < this.bounds.length && this.bounds[i + 1] < other.bounds[j + 1]) i += 2;
      if (i >= this.bounds.length || this.bounds[i] > other.bounds[j]) return false;
    }
    return true;
  }

  /** The part of this set before `end`. */
  below(end: number): SpanSet {
    if (this.end <= end) return this;
    const pairs = this.toPairs()
      .filter(([start]) => start < end)
      .map(([start, stop]) => [start, Math.min(stop, end)]);
    return SpanSet.fromPairs(pairs) ?? SpanSet.empty;
  }

  get size(): number {
    let total = 0;
    for (let i = 0; i < this.bounds.length; i += 2) total += this.bounds[i + 1] - this.bounds[i];
    return total;
  }

  get isEmpty(): boolean {
    return this.bounds.length === 0;
  }

  /** Largest end, or 0 for the empty set. */
  get end(): number {
    return this.bounds.length ? this.bounds[this.bounds.length - 1] : 0;
  }

  equals(other: SpanSet): boolean {
    return (
      this.bounds.length === other.bounds.length &&
      this.bounds.every((bound, i) => bound === other.bounds[i])
    );
  }

  toPairs(): [number, number][] {
    const pairs: [number, number][] = [];
    for (let i = 0; i < this.bounds.length; i += 2)
      pairs.push([this.bounds[i], this.bounds[i + 1]]);
    return pairs;
  }
}
