import { describe, expect, it } from 'vitest';
import {
  createGoalRequest,
  parseDailyGoalState,
  parseGoalData,
  parseStatisticDefaultGoal,
  serializeGoalData,
} from './schemas';

describe('parseGoalData (backend response shapes)', () => {
  it('maps AmountGoal target_amount to target', () => {
    expect(parseGoalData('IntegerAmount', { target_amount: 27000, is_at_least: true })).toEqual({
      type: 'IntegerAmount',
      data: { target: 27000, is_at_least: true },
    });
  });

  it('maps HabitGoal target to occurred', () => {
    expect(parseGoalData('Habit', { target: true })).toEqual({
      type: 'Habit',
      data: { occurred: true },
    });
  });

  it('maps FinishedInTimeGoal target to deadline', () => {
    expect(parseGoalData('PointInTime', { target: '22:00:00' })).toEqual({
      type: 'PointInTime',
      data: { deadline: '22:00:00' },
    });
  });

  it('keeps TimeSpans frames and accepts a JSON-encoded frame list', () => {
    const frames = [{ start: '07:45:00', end: '08:45:00' }];
    expect(parseGoalData('TimeSpans', { target: frames, efficiency: 90 })).toEqual({
      type: 'TimeSpans',
      data: { target: frames, efficiency: 90 },
    });
    expect(
      parseGoalData('TimeSpans', { target: JSON.stringify(frames), efficiency: 90 }).data
    ).toEqual({ target: frames, efficiency: 90 });
  });

  it('rejects unknown shapes', () => {
    expect(() => parseGoalData('IntegerAmount', { target: '5' })).toThrow();
  });
});

describe('parseStatisticDefaultGoal', () => {
  it('returns null without a goal and on unreadable goals', () => {
    const base = { id: 1, name: 'n', category: 'c', statistic_type: 'Habit' as const };
    expect(parseStatisticDefaultGoal({ ...base, default_goal: null })).toBeNull();
    expect(parseStatisticDefaultGoal({ ...base, default_goal: { nonsense: 1 } })).toBeNull();
    expect(parseStatisticDefaultGoal({ ...base, default_goal: { target: false } })).toEqual({
      type: 'Habit',
      data: { occurred: false },
    });
  });
});

describe('parseDailyGoalState', () => {
  it('passes through not_set and suppressed', () => {
    expect(parseDailyGoalState('Habit', { kind: 'not_set' })).toEqual({ kind: 'not_set' });
    expect(parseDailyGoalState('Habit', { kind: 'suppressed' })).toEqual({ kind: 'suppressed' });
  });

  it('parses the goal variant', () => {
    expect(
      parseDailyGoalState('IntegerAmount', {
        kind: 'goal',
        goal_data: { target_amount: 5, is_at_least: false },
      })
    ).toEqual({
      kind: 'goal',
      goal: {
        statistic_type: 'IntegerAmount',
        goal_data: { type: 'IntegerAmount', data: { target: 5, is_at_least: false } },
      },
    });
  });
});

describe('request serialization', () => {
  it('stringifies primitives and JSON-encodes structures', () => {
    expect(
      serializeGoalData({ target: [{ start: '09:00', end: '10:00' }], efficiency: 80, skip: null })
    ).toEqual({ target: '[{"start":"09:00","end":"10:00"}]', efficiency: '80' });
  });

  it('wraps a daily goal request', () => {
    expect(createGoalRequest({ target: 5, is_at_least: true })).toEqual({
      kind: 'goal',
      goal_data: { target: '5', is_at_least: 'true' },
    });
  });
});
