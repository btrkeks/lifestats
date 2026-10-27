import { describe, expect, it } from 'vitest';
import type { AchievementDay } from '@/types';
import { classifyDay } from './day';

const today = '2026-10-27';

function day(overrides: Partial<AchievementDay>): AchievementDay {
  return {
    date: '2026-10-25',
    has_goal: true,
    achieved: false,
    percentage_achieved: 61,
    override: 'none',
    value: 244,
    goal: { type: 'IntegerAmount', data: { target: 400, is_at_least: true } },
    ...overrides,
  };
}

describe('classifyDay', () => {
  it('flags suppressed days regardless of other fields', () => {
    expect(classifyDay(day({ override: 'suppressed', has_goal: false }), today)).toBe('suppressed');
  });

  it('flags days without a goal', () => {
    expect(classifyDay(day({ has_goal: false, goal: null }), today)).toBe('none');
  });

  it('flags met days, including with a custom goal', () => {
    expect(classifyDay(day({ achieved: true, override: 'goal' }), today)).toBe('met');
  });

  it('separates a missed past day from today still in progress', () => {
    expect(classifyDay(day({}), today)).toBe('missed');
    expect(classifyDay(day({ date: today }), today)).toBe('open');
  });

  it('marks days after today as future, even with a goal', () => {
    expect(classifyDay(day({ date: '2026-10-28', override: 'suppressed' }), today)).toBe('future');
  });
});
