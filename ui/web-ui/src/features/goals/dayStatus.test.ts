import { describe, expect, it } from 'vitest';
import type { DailyAchievement } from '@/types';
import { classifyDay } from './dayStatus';

const today = '2026-09-03';

function day(overrides: Partial<DailyAchievement>): DailyAchievement {
  return {
    date: '2026-09-01',
    has_goal: true,
    achieved: false,
    percentage_achieved: 40,
    override: 'none',
    ...overrides,
  };
}

describe('classifyDay', () => {
  it('flags suppressed days regardless of other fields', () => {
    expect(classifyDay(day({ override: 'suppressed', has_goal: false }), today)).toBe('suppressed');
  });

  it('flags days without a goal', () => {
    expect(classifyDay(day({ has_goal: false, percentage_achieved: null }), today)).toBe('no_goal');
  });

  it('flags achieved days, including with a custom goal', () => {
    expect(classifyDay(day({ achieved: true, override: 'goal' }), today)).toBe('achieved');
  });

  it('treats an unachieved goal as pending today and missed in the past', () => {
    expect(classifyDay(day({ date: today }), today)).toBe('pending');
    expect(classifyDay(day({ date: '2026-09-02' }), today)).toBe('missed');
  });
});
