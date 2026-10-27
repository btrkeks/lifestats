import { describe, expect, it } from 'vitest';
import { datesBetween, formatMonth, monthGrid, shiftMonth, startOfWeekIso } from './grid';

describe('monthGrid', () => {
  it('covers whole Monday-to-Sunday weeks around the month', () => {
    const grid = monthGrid('2026-10');
    expect(grid.start).toBe('2026-09-28');
    expect(grid.end).toBe('2026-11-01');
    expect(grid.dates).toHaveLength(35);
  });

  it('grows to six rows when the month needs them', () => {
    expect(monthGrid('2026-08').dates).toHaveLength(42);
  });
});

describe('month helpers', () => {
  it('shifts across years and formats', () => {
    expect(shiftMonth('2026-12', 1)).toBe('2027-01');
    expect(shiftMonth('2026-01', -1)).toBe('2025-12');
    expect(formatMonth('2026-10')).toBe('October 2026');
  });
});

describe('datesBetween', () => {
  it('lists the inclusive range in either direction', () => {
    expect(datesBetween('2026-10-21', '2026-10-19')).toEqual([
      '2026-10-19',
      '2026-10-20',
      '2026-10-21',
    ]);
    expect(datesBetween('2026-10-12', '2026-10-21')).toHaveLength(10);
  });

  it('finds the Monday of a week', () => {
    expect(startOfWeekIso('2026-10-27')).toBe('2026-10-26');
    expect(startOfWeekIso('2026-10-25')).toBe('2026-10-19');
  });
});
