import { describe, expect, it } from 'vitest';
import type { GoalData } from '@/types';
import type { Day, DayStatus } from './day';
import { anchoredMinutes, formatMinutes, goalLevel, metCount, trendPoints, trendReadout } from './insights';

const bedtime: GoalData = { type: 'PointInTime', data: { deadline: '22:05:00' } };

function day(date: string, status: DayStatus, value: Day['value'] = null): Day {
  return {
    date,
    status,
    value,
    has_goal: status !== 'none' && status !== 'suppressed',
    achieved: status === 'met',
    percentage_achieved: status === 'met' ? 100 : 50,
    override: status === 'suppressed' ? 'suppressed' : 'none',
    goal: null,
  };
}

/** `count` consecutive October days starting on the 1st */
function days(count: number, make: (i: number) => [DayStatus, Day['value']]): Day[] {
  return Array.from({ length: count }, (_, i) => {
    const [status, value] = make(i);
    return day(`2026-10-${String(i + 1).padStart(2, '0')}`, status, value);
  });
}

describe('anchoredMinutes', () => {
  it('keeps late-night times next to an evening goal', () => {
    expect(anchoredMinutes('00:30', 22 * 60 + 5)).toBe(24 * 60 + 30);
    expect(anchoredMinutes('22:37:00', 22 * 60 + 5)).toBe(22 * 60 + 37);
    expect(anchoredMinutes('05:50', 6 * 60)).toBe(5 * 60 + 50);
    expect(formatMinutes(24 * 60 + 30)).toBe('24:30');
  });
});

describe('metCount', () => {
  it('counts today but not suppressed or goal-less days', () => {
    const list = [day('a', 'met'), day('b', 'missed'), day('c', 'suppressed'), day('d', 'open')];
    expect(metCount(list)).toEqual({ met: 1, withGoal: 3 });
  });
});

describe('trendReadout', () => {
  it('compares the met share of the last 14 finished days with the 14 before', () => {
    // 1-14 all missed, 15-28 all met, 29 = today (open)
    const list = days(29, (i) => [i === 28 ? 'open' : i < 14 ? 'missed' : 'met', 1]);
    expect(trendReadout(list, { kind: 'count', unit: null }, null)).toEqual({
      label: 'Last 14 days',
      value: '100% met',
      delta: { text: 'from 0% before', direction: 'up', better: true },
    });
  });

  it('averages times and reads earlier as better for deadlines', () => {
    const list = days(29, (i) => ['missed', i < 14 ? '23:00:00' : '22:30:00']);
    expect(trendReadout(list, { kind: 'clock' }, bedtime)).toEqual({
      label: 'Last 14 days',
      value: '22:30 avg',
      delta: { text: '30 min earlier than the 14 before', direction: 'down', better: true },
    });
  });

  it('says so when no day had a goal', () => {
    const list = days(5, () => ['none', null]);
    expect(trendReadout(list, { kind: 'count', unit: null }, null).value).toBe('No goal days');
  });
});

describe('trendPoints', () => {
  it('plots amounts, anchored times and percentages', () => {
    expect(trendPoints([day('a', 'met', 412)], { kind: 'count', unit: null }, 400)[0].y).toBe(412);
    expect(trendPoints([day('a', 'missed', '00:10:00')], { kind: 'clock' }, 1325)[0].y).toBe(1450);
    expect(trendPoints([day('a', 'met', 3600)], { kind: 'spans' }, 100)[0].y).toBe(100);
  });
});

describe('TimedAmounts insights', () => {
  it('plots scalar totals, including zero, and leaves missing values unplotted', () => {
    const goal: GoalData = { type: 'TimedAmounts', data: { target: 400, is_at_least: false } };
    expect(goalLevel(goal)).toBe(400);
    expect(trendPoints([
      day('2026-10-25', 'missed', null), day('2026-10-26', 'met', 0), day('2026-10-27', 'met', 180),
    ], { kind: 'count', unit: 'mg' }, goalLevel(goal)).map((point) => point.y)).toEqual([null, 0, 180]);
  });
});
