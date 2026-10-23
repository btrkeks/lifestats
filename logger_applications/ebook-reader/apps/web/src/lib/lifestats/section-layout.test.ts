/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { ReaderBinding } from '@lifestats/ttu-sync';
import { describe, expect, it } from 'vitest';
import { withoutSectionLayout } from './section-layout';

function recorder() {
  const seen: number[] = [];
  const binding: ReaderBinding = {
    setBook: () => undefined,
    observe: (position) => seen.push(position),
    reachedEnd: () => undefined,
    close: async () => undefined
  };
  return { seen, binding };
}

describe('withoutSectionLayout', () => {
  it('drops the 0 a section load reports and passes on the measured position', () => {
    const { seen, binding } = recorder();
    const reader = withoutSectionLayout(binding);
    reader.binding.observe(2600);
    reader.startLayout();
    reader.binding.observe(0);
    reader.endLayout(2900);

    expect(seen).toEqual([2600, 2900]);
  });

  it('passes on the position a section load ends at, even when it was reported during the load', () => {
    const { seen, binding } = recorder();
    const reader = withoutSectionLayout(binding);
    reader.binding.observe(2282);
    reader.startLayout();
    reader.binding.observe(2638);
    reader.endLayout(2638);

    expect(seen).toEqual([2282, 2638]);
  });

  it('passes a position of 0 outside a section load', () => {
    const { seen, binding } = recorder();
    const reader = withoutSectionLayout(binding);
    reader.binding.observe(100_000);
    reader.binding.observe(0);
    reader.binding.observe(80);

    expect(seen).toEqual([100_000, 0, 80]);
  });
});
