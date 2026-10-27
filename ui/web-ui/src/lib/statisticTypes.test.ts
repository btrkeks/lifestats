import { describe, expect, it } from 'vitest';
import { canEditGoals, DERIVED_TYPES, LOGGED_TYPES, STATISTIC_TYPES } from './statisticTypes';

describe('source-owned statistics', () => {
  it('keeps TimedAmounts out of both creation groups while allowing goal editing', () => {
    expect(STATISTIC_TYPES.TimedAmounts.origin).toBe('source');
    expect(LOGGED_TYPES.map((info) => info.type)).toEqual(['IntegerAmount', 'Habit', 'PointInTime', 'TimeSpans']);
    expect(DERIVED_TYPES.map((info) => info.type)).toEqual(['Summing', 'TimeDifference']);
    expect(canEditGoals('TimedAmounts')).toBe(true);
  });
});
