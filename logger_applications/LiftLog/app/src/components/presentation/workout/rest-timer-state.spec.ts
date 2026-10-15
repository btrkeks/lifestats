import { describe, expect, it } from 'vitest';
import { Duration } from '@js-joda/core';
import { getRestTimerState } from '@/components/presentation/workout/rest-timer-state';

const window = { minRest: Duration.ofSeconds(60), maxRest: Duration.ofSeconds(90) };
const fixed = { minRest: Duration.ofSeconds(60), maxRest: Duration.ofSeconds(60) };

const at = (rest: typeof window, seconds: number) => {
  const state = getRestTimerState(rest, seconds * 1000);
  return {
    phase: state.phase,
    timeMs: state.phase === 'over' ? state.overtimeMs : state.leftMs,
    left: [state.restFractionLeft, state.windowFractionLeft],
  };
};

describe('getRestTimerState', () => {
  it('starts with both segments full', () => {
    expect(at(window, 0)).toEqual({ phase: 'resting', timeMs: 60_000, left: [1, 1] });
  });

  it('drains the rest segment first', () => {
    expect(at(window, 30)).toEqual({ phase: 'resting', timeMs: 30_000, left: [0.5, 1] });
  });

  it('drains the window once the minimum rest is up', () => {
    expect(at(window, 60)).toEqual({ phase: 'ready', timeMs: 30_000, left: [0, 1] });
    expect(at(window, 75)).toEqual({ phase: 'ready', timeMs: 15_000, left: [0, 0.5] });
  });

  it('is empty past the maximum rest and counts the overtime up', () => {
    expect(at(window, 100)).toEqual({ phase: 'over', timeMs: 10_000, left: [0, 0] });
  });

  it('goes straight from resting to over for a fixed rest', () => {
    expect(at(fixed, 45)).toEqual({ phase: 'resting', timeMs: 15_000, left: [0.25, 1] });
    expect(at(fixed, 70)).toEqual({ phase: 'over', timeMs: 10_000, left: [0, 0] });
  });
});
