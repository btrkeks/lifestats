/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { describe, expect, it } from 'vitest';

import { isBookKey, isReadingDay, newProducerId, isProducerId, readingDayOf } from './ids';

describe('readingDayOf (TZ=Europe/Berlin)', () => {
  it.each([
    ['2026-10-22T03:59:59', '2026-10-21'],
    ['2026-10-22T04:00:00', '2026-10-22'],
    ['2026-10-22T23:59:59', '2026-10-22'],
    ['2026-01-01T02:00:00', '2025-12-31'],
    ['2026-03-01T03:00:00', '2026-02-28'],
    ['2026-03-29T03:30:00', '2026-03-28'],
    ['2026-03-29T04:00:00', '2026-03-29'],
    ['2026-10-25T03:30:00', '2026-10-24'],
    ['2026-10-25T04:00:00', '2026-10-25']
  ])('maps local %s to %s', (local, expected) => {
    expect(readingDayOf(new Date(local))).toBe(expected);
  });

  it('runs in the zone the DST cases assume', () => {
    expect(new Date('2026-03-29T03:30:00').getTimezoneOffset()).toBe(-120);
    expect(new Date('2026-03-28T03:30:00').getTimezoneOffset()).toBe(-60);
  });
});

describe('identifier parsing', () => {
  it('accepts only real calendar days', () => {
    expect(isReadingDay('2026-02-28')).toBe(true);
    expect(isReadingDay('2026-02-29')).toBe(false);
    expect(isReadingDay('2026-2-28')).toBe(false);
  });

  it('accepts only the server key format', () => {
    expect(isBookKey(`ttu-text-v1:${'a'.repeat(64)}`)).toBe(true);
    expect(isBookKey(`ttu-text-v1:${'A'.repeat(64)}`)).toBe(false);
    expect(isBookKey(`ttu-book-v1:${'a'.repeat(64)}`)).toBe(false);
    expect(isBookKey('ttu-text-v1:abc')).toBe(false);
  });

  it('creates canonical lowercase producer ids', () => {
    const id = newProducerId();
    expect(isProducerId(id)).toBe(true);
    expect(isProducerId(id.toUpperCase())).toBe(false);
  });
});
