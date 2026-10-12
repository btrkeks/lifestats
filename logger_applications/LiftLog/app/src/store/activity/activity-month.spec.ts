import { describe, expect, it } from 'vitest';
import { DayOfWeek, LocalDate, YearMonth } from '@js-joda/core';
import { ownHistoryOf, selectActivityMonth } from '@/store/activity';
import { RootState } from '@/store';

const today = LocalDate.of(2026, 2, 12);
const state = { settings: { firstDayOfWeek: DayOfWeek.MONDAY } } as RootState;
const own = ownHistoryOf([{ id: 'a', date: LocalDate.of(2026, 2, 10), volumeKg: 1000 }], [], DayOfWeek.MONDAY, today);

describe('selectActivityMonth', () => {
  const rows = selectActivityMonth(state, { yearMonth: YearMonth.of(2026, 2), today, own });
  const cells = rows.flatMap((row) => row.cells);

  it('pads February 2026 out to whole Monday-first weeks', () => {
    expect(rows).toHaveLength(5);
    expect(cells[0]!.date).toEqual(LocalDate.of(2026, 1, 26));
    expect(cells.at(-1)!.date).toEqual(LocalDate.of(2026, 3, 1));
    expect(cells.filter((cell) => cell.isOutsideFocus).map((cell) => cell.date.toString())).toEqual([
      '2026-01-26',
      '2026-01-27',
      '2026-01-28',
      '2026-01-29',
      '2026-01-30',
      '2026-01-31',
      '2026-03-01',
    ]);
  });

  it('grades the days you trained and marks today and the future', () => {
    const trainedDay = cells.find((cell) => cell.date.equals(LocalDate.of(2026, 2, 10)))!;
    expect(trainedDay.sessionCount).toBe(1);
    expect(trainedDay.level).toBeGreaterThan(0);

    expect(cells.filter((cell) => cell.level > 0)).toHaveLength(1);
    expect(cells.find((cell) => cell.isToday)!.date).toEqual(today);
    expect(cells.find((cell) => cell.isFuture)!.date).toEqual(today.plusDays(1));
  });
});
