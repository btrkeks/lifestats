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
