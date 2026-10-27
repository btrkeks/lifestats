import { describe, expect, it } from 'vitest';
import type { Day, DayStatus } from './day';
import { describeSelection, formatSelectionSpan } from './selection';

function day(status: DayStatus, override: Day['override'] = 'none'): Day {
  return {
    date: '2026-10-12',
    status,
    override,
    has_goal: status !== 'none',
    achieved: status === 'met',
    percentage_achieved: null,
    value: null,
    goal: null,
  };
}

describe('describeSelection', () => {
  it('says "all" when every day is alike', () => {
    expect(describeSelection([day('none'), day('none')])).toBe('all without a goal');
  });

  it('counts each day once, custom goals first', () => {
    expect(
      describeSelection([day('missed', 'goal'), day('missed'), day('suppressed', 'suppressed')])
    ).toBe('1 with a custom goal, 1 suppressed, 1 missed');
  });
});

describe('formatSelectionSpan', () => {
  it('names the month once when both ends share it', () => {
    expect(formatSelectionSpan('2026-10-12', '2026-10-21')).toBe('Mon 12 – Wed 21 Oct');
    expect(formatSelectionSpan('2026-09-30', '2026-10-02')).toBe('Wed 30 Sep – Fri 2 Oct');
  });
});
