import { describe, expect, it } from 'vitest';
import type { GoalData } from '@/types';
import type { Day, DayStatus } from './day';
import { todayCounts, todayFigure, todayProgress } from './today';

function today(status: DayStatus, value: Day['value'], goal: GoalData | null): Day {
  return {
    date: '2026-10-27',
    status,
    value,
    goal,
    has_goal: goal !== null,
    achieved: status === 'met',
    percentage_achieved: 81,
    override: status === 'suppressed' ? 'suppressed' : 'none',
  };
}

const cards = { kind: 'count', unit: 'cards' } as const;
const atLeast400: GoalData = { type: 'IntegerAmount', data: { target: 400, is_at_least: true } };
const atMostHour: GoalData = { type: 'IntegerAmount', data: { target: 3600, is_at_least: false } };
const bedtime: GoalData = { type: 'PointInTime', data: { deadline: '22:05:00' } };

describe('todayProgress', () => {
  it('reads amounts against the target', () => {
    expect(todayProgress(today('open', 327, atLeast400), cards)).toEqual({
      text: '327 of 400',
      tone: 'open',
    });
    expect(todayProgress(today('open', null, atLeast400), cards)).toEqual({
      text: '0 of 400',
      tone: 'open',
    });
  });

  it('reads an upper limit as time used', () => {
    expect(todayProgress(today('met', 20, atMostHour), { kind: 'duration' })).toEqual({
      text: '20 s used',
      tone: 'met',
    });
  });

  it('reads an open deadline as due', () => {
    expect(todayProgress(today('open', null, bedtime), { kind: 'clock' })).toEqual({
      text: 'Due 22:05',
      tone: 'open',
    });
  });

  it('reads suppressed days as such', () => {
    expect(todayProgress(today('suppressed', null, null), cards)).toEqual({
      text: 'Suppressed',
      tone: 'muted',
    });
  });
});

describe('todayFigure', () => {
  it('formats the header figure', () => {
    expect(todayFigure(today('open', 327, atLeast400), cards)).toBe('327 / 400');
    expect(todayFigure(today('open', null, bedtime), { kind: 'clock' })).toBe('due 22:05');
  });
});

describe('todayCounts', () => {
  it('counts statuses', () => {
    const days = [
      today('met', 1, atLeast400),
      today('open', 1, atLeast400),
      today('suppressed', null, null),
    ];
    expect(todayCounts(days)).toEqual({ met: 1, open: 1, missed: 0, suppressed: 1 });
  });
});

describe('TimedAmounts today', () => {
  const mg = { kind: 'count', unit: 'mg' } as const;
  const cap: GoalData = { type: 'TimedAmounts', data: { target: 400, is_at_least: false } };
  const minimum: GoalData = { type: 'TimedAmounts', data: { target: 400, is_at_least: true } };

  it('preserves missing data for both goal directions', () => {
    for (const goal of [cap, minimum]) {
      const day = today('open', null, goal);
      expect(todayProgress(day, mg)).toEqual({ text: 'No data', tone: 'muted' });
      expect(todayFigure(day, mg)).toBe('No data');
    }
  });

  it('shows confirmed zero intake as 0 mg', () => {
    const day = today('met', 0, cap);
    expect(todayProgress(day, mg)).toEqual({ text: '0 mg used', tone: 'met' });
    expect(todayFigure(day, mg)).toBe('0 / 400 mg');
  });

  it('formats upper limits and minimums with their units and status', () => {
    expect(todayProgress(today('met', 180, cap), mg)).toEqual({ text: '180 mg used', tone: 'met' });
    expect(todayProgress(today('missed', 500, cap), mg)).toEqual({ text: '500 mg used', tone: 'missed' });
    expect(todayProgress(today('open', 180, minimum), mg)).toEqual({ text: '180 of 400 mg', tone: 'open' });
    expect(todayProgress(today('met', 400, minimum), mg)).toEqual({ text: '400 of 400 mg', tone: 'met' });
    expect(todayFigure(today('met', 180, cap), mg)).toBe('180 / 400 mg');
  });
});
