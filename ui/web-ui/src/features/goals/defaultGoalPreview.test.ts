import { describe, expect, it } from 'vitest';
import type { GoalData } from '@/types';
import { datesBetween } from '@/features/calendar/grid';
import type { Day, DayStatus } from './day';
import { describeEffects, previewMark } from './defaultGoalPreview';

const bedtime: GoalData = { type: 'PointInTime', data: { deadline: '22:05:00' } };
const clock = { kind: 'clock' } as const;
const today = '2026-10-27';

function day(date: string, status: DayStatus, override: Day['override'] = 'none'): Day {
  const hasGoal = status === 'met' || status === 'missed' || status === 'open';
  return {
    date,
    status,
    override,
    has_goal: hasGoal,
    achieved: status === 'met',
    percentage_achieved: hasGoal ? 70 : null,
    value: null,
    goal: hasGoal ? bedtime : null,
  };
}

/** Bedtime before the hand backfill: no goal 12-21, goal from 22, suppressed 24-26 */
const october: Day[] = datesBetween('2026-10-12', today).map((date) => {
  if (date < '2026-10-22') return day(date, 'none');
  if (date >= '2026-10-24' && date <= '2026-10-26') return day(date, 'suppressed', 'suppressed');
  return day(date, date === today ? 'open' : 'missed');
});

describe('previewMark', () => {
  it('marks days by what saving does to them', () => {
    const marks = october.map((d) => previewMark(d, '2026-10-12', bedtime));
    expect(marks.slice(0, 10).every((mark) => mark === 'gains')).toBe(true);
    expect(marks[10]).toBe('same');
    expect(marks[12]).toBe('override');
    expect(previewMark(day('2026-10-11', 'none'), '2026-10-12', bedtime)).toBe('before');
    expect(previewMark(day('2026-10-28', 'future'), '2026-10-12', bedtime)).toBe('future');
  });
});

describe('describeEffects', () => {
  it('summarises backdating the bedtime goal to 12 Oct', () => {
    const effects = describeEffects({
      days: october,
      appliesFrom: '2026-10-12',
      newGoal: bedtime,
      current: { goal: bedtime, validSince: '2026-10-22' },
      measure: clock,
      today,
    });
    expect(effects).toEqual([
      { kind: 'start', text: 'Starts Mon 12 Oct instead of Thu 22 Oct' },
      { kind: 'gains', text: '10 more days get this goal, no backfilling' },
      { kind: 'suppressed', text: '24–26 Oct stay suppressed' },
    ]);
  });

  it('keeps the old goal on earlier days when the new one starts later', () => {
    const later: GoalData = { type: 'PointInTime', data: { deadline: '22:30' } };
    const effects = describeEffects({
      days: october.filter((d) => d.date === today),
      appliesFrom: today,
      newGoal: later,
      current: { goal: bedtime, validSince: '2026-10-22' },
      measure: clock,
      today,
    });
    expect(effects).toEqual([
      { kind: 'start', text: 'Starts Tue 27 Oct instead of Thu 22 Oct' },
      { kind: 'gains', text: '1 day switches to this goal' },
      { kind: 'earlier', text: 'Days before Tue 27 Oct keep by 22:05' },
    ]);
  });

  it('describes a first goal', () => {
    const effects = describeEffects({
      days: [day(today, 'none')],
      appliesFrom: today,
      newGoal: bedtime,
      current: null,
      measure: clock,
      today,
    });
    expect(effects).toEqual([
      { kind: 'start', text: 'Starts Tue 27 Oct' },
      { kind: 'gains', text: '1 more day gets this goal, no backfilling' },
    ]);
  });

  it('uses the singular for one suppressed day', () => {
    const effects = describeEffects({
      days: [day('2026-10-25', 'suppressed', 'suppressed')],
      appliesFrom: '2026-10-25',
      newGoal: bedtime,
      current: { goal: bedtime, validSince: '2026-10-22' },
      measure: clock,
      today,
    });
    expect(effects).toContainEqual({ kind: 'suppressed', text: '25 Oct stays suppressed' });
  });
});
