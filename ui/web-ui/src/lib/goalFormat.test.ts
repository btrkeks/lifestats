import { describe, expect, it } from 'vitest';
import type { GoalData } from '@/types';
import { describeGoal, formatClock, formatGoal, formatGoalTarget, goalsEqual } from './goalFormat';

const cards = { kind: 'count', unit: 'cards' } as const;
const plain = { kind: 'count', unit: null } as const;
const seconds = { kind: 'duration' } as const;
const clock = { kind: 'clock' } as const;

const atLeast400: GoalData = { type: 'IntegerAmount', data: { target: 400, is_at_least: true } };
const bedtime: GoalData = { type: 'PointInTime', data: { deadline: '22:05:00' } };
const reading: GoalData = {
  type: 'TimeSpans',
  data: { target: [{ start: '21:15:00', end: '21:45:00' }], efficiency: 80 },
};

describe('formatClock', () => {
  it('drops zero seconds', () => {
    expect(formatClock('07:45:00')).toBe('07:45');
  });
  it('keeps non-zero seconds and HH:MM input', () => {
    expect(formatClock('07:45:30')).toBe('07:45:30');
    expect(formatClock('07:45')).toBe('07:45');
  });
});

describe('formatGoal', () => {
  it('formats amount goals with a sign and unit', () => {
    expect(formatGoal(atLeast400, cards)).toBe('≥ 400 cards');
    expect(
      formatGoal({ type: 'Summing', data: { target: 27000, is_at_least: false } }, plain)
    ).toBe('≤ 27,000');
  });
  it('formats second-based amounts as durations', () => {
    expect(
      formatGoal({ type: 'IntegerAmount', data: { target: 25200, is_at_least: true } }, seconds)
    ).toBe('≥ 7 h');
  });
  it('formats habits, deadlines and windows', () => {
    expect(formatGoal({ type: 'Habit', data: { occurred: true } }, { kind: 'habit' })).toBe('Done');
    expect(formatGoal(bedtime, clock)).toBe('by 22:05');
    expect(formatGoal(reading, { kind: 'spans' })).toBe('21:15–21:45 · 80%');
  });
});

describe('formatGoalTarget and describeGoal', () => {
  it('reads the bare target', () => {
    expect(formatGoalTarget(atLeast400, cards)).toBe('400');
    expect(formatGoalTarget(bedtime, clock)).toBe('22:05');
  });
  it('describes the goal in a sentence', () => {
    expect(describeGoal(atLeast400, cards)).toBe('At least 400 a day.');
    expect(describeGoal(bedtime, clock)).toBe('Logged by 22:05 each day.');
  });
});

describe('goalsEqual', () => {
  it('ignores missing seconds on clock times', () => {
    expect(goalsEqual(bedtime, { type: 'PointInTime', data: { deadline: '22:05' } })).toBe(true);
    expect(goalsEqual(bedtime, { type: 'PointInTime', data: { deadline: '22:10' } })).toBe(false);
  });
  it('compares amounts and windows field by field', () => {
    expect(
      goalsEqual(atLeast400, { ...atLeast400, data: { target: 400, is_at_least: false } })
    ).toBe(false);
    expect(
      goalsEqual(reading, {
        type: 'TimeSpans',
        data: { target: [{ start: '21:15', end: '21:45' }], efficiency: 80 },
      })
    ).toBe(true);
  });
});

describe('TimedAmounts formatting', () => {
  const mg = { kind: 'count', unit: 'mg' } as const;
  const cap: GoalData = { type: 'TimedAmounts', data: { target: 400, is_at_least: false } };

  it('uses total targets, direction and mg', () => {
    expect(formatGoalTarget(cap, mg)).toBe('400');
    expect(formatGoal(cap, mg)).toBe('≤ 400 mg');
    expect(describeGoal(cap, mg)).toBe('At most 400 mg a day.');
    const minimum: GoalData = { type: 'TimedAmounts', data: { target: 100, is_at_least: true } };
    expect(formatGoal(minimum, mg)).toBe('≥ 100 mg');
    expect(describeGoal(minimum, mg)).toBe('At least 100 mg a day.');
  });

  it('compares amount and direction without equating other statistic types', () => {
    expect(goalsEqual(cap, { ...cap })).toBe(true);
    expect(goalsEqual(cap, { type: 'TimedAmounts', data: { target: 200, is_at_least: false } })).toBe(false);
    expect(goalsEqual(cap, { type: 'TimedAmounts', data: { target: 400, is_at_least: true } })).toBe(false);
    expect(goalsEqual(cap, { type: 'IntegerAmount', data: cap.data })).toBe(false);
  });
});
