import { describe, expect, it } from 'vitest';
import {
  createGoalRequest,
  formGoal,
  goalFormDefaultsFor,
  goalFormSchemaFor,
  parseDailyGoalState,
  parseGoalData,
  parseStatisticDefaultGoal,
  serializeGoalData,
  statisticListSchema,
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

describe('formGoal', () => {
  it('reads valid form values as a goal', () => {
    expect(formGoal('PointInTime', { deadline: '22:05' })).toEqual({
      type: 'PointInTime',
      data: { deadline: '22:05' },
    });
    expect(formGoal('IntegerAmount', { target: '400', is_at_least: 'true' })).toEqual({
      type: 'IntegerAmount',
      data: { target: 400, is_at_least: true },
    });
  });

  it('returns null while the form is invalid', () => {
    expect(formGoal('PointInTime', { deadline: '' })).toBeNull();
    expect(formGoal('TimeSpans', { target: [], efficiency: 80 })).toBeNull();
  });
});

describe('TimedAmounts goals', () => {
  it('maps stored totals to editable goals and serializes the request fields', () => {
    const goal = parseGoalData('TimedAmounts', { target_amount: 400, is_at_least: false });
    expect(goal).toEqual({ type: 'TimedAmounts', data: { target: 400, is_at_least: false } });
    expect(serializeGoalData({ ...goal.data })).toEqual({ target: '400', is_at_least: 'false' });
    expect(goalFormDefaultsFor('TimedAmounts')).toEqual({ target: 0, is_at_least: false });
  });

  it.each([0, 400, Number.MAX_SAFE_INTEGER])('accepts a safe nonnegative target %s', (target) => {
    expect(goalFormSchemaFor('TimedAmounts').parse({ target: String(target), is_at_least: 'false' }))
      .toEqual({ target, is_at_least: false });
    expect(formGoal('TimedAmounts', { target, is_at_least: true }))
      .toEqual({ type: 'TimedAmounts', data: { target, is_at_least: true } });
    expect(parseGoalData('TimedAmounts', { target_amount: target, is_at_least: false }).data)
      .toEqual({ target, is_at_least: false });
  });

  it.each([-1, 0.5, Number.MAX_SAFE_INTEGER + 1, Infinity, NaN])('rejects invalid target %s', (target) => {
    expect(goalFormSchemaFor('TimedAmounts').safeParse({ target, is_at_least: false }).success).toBe(false);
    expect(formGoal('TimedAmounts', { target, is_at_least: false })).toBeNull();
    expect(() => parseGoalData('TimedAmounts', { target_amount: target, is_at_least: false })).toThrow();
  });

  it('rejects form-shaped or string targets in stored goals', () => {
    expect(() => parseGoalData('TimedAmounts', { target: 400, is_at_least: false })).toThrow();
    expect(() => parseGoalData('TimedAmounts', { target_amount: '400', is_at_least: false })).toThrow();
  });
});

describe('statistic listings', () => {
  const intake = {
    id: 42, name: '[OpenNutriTracker] Caffeine (mg)', category: 'OpenNutriTracker',
    statistic_type: 'TimedAmounts', default_goal: { target_amount: 400, is_at_least: false },
  };
  const habit = { id: 1, name: 'Read', category: '', statistic_type: 'Habit', default_goal: null };

  it('keeps known entries and skips future types wherever they appear', () => {
    expect(statisticListSchema.parse([
      { statistic_type: 'FutureType' }, habit, intake, { statistic_type: 'AnotherFutureType' },
    ])).toEqual([habit, intake]);
    expect(statisticListSchema.parse([{ statistic_type: 'toString' }])).toEqual([]);
    expect(statisticListSchema.parse([])).toEqual([]);
  });

  it('retains validation for known types and malformed list envelopes', () => {
    expect(() => statisticListSchema.parse([{ ...intake, id: '42' }])).toThrow();
    expect(() => statisticListSchema.parse([{ ...intake, default_goal: [] }])).toThrow();
    expect(() => statisticListSchema.parse([{ statistic_type: 42 }])).toThrow();
    expect(() => statisticListSchema.parse({})).toThrow();
  });
});
