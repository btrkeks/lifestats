import { createSelector } from '@reduxjs/toolkit';
import { LocalDate, YearMonth } from '@js-joda/core';
import { RootState } from '@/store/store';
import { ActivityCell, ActivityRow } from '@/store/activity/activity-types';
import { levelFor } from '@/store/activity/volume';
import { DayActivity, OwnHistory } from '@/store/activity/own-history';

export * from '@/store/activity/activity-types';
export * from '@/store/activity/volume';
export * from '@/store/activity/streak';
export * from '@/store/activity/week-start';
export * from '@/store/activity/own-history';

const selectFirstDayOfWeek = (state: RootState) => state.settings.firstDayOfWeek;

const noActivity: DayActivity = { sessionCount: 0, volume: 0 };

function buildCell(date: LocalDate, own: OwnHistory, today: LocalDate, isOutsideFocus: boolean): ActivityCell {
  const day = own.days.get(date.toString()) ?? noActivity;
  const scale = own.volumeScale;

  return {
    date,
    level: day.sessionCount === 0 || !scale ? 0 : levelFor(day.volume, scale),
    sessionCount: day.sessionCount,
    isToday: date.isEqual(today),
    isFuture: date.isAfter(today),
    isOutsideFocus,
  };
}

export interface ActivityMonthParams {
  today: LocalDate;
  own: OwnHistory;
  yearMonth: YearMonth;
}

/** `today` is an argument rather than read from the clock, so memoization can't go stale across midnight. */
export const selectActivityMonth = createSelector(
  [
    selectFirstDayOfWeek,
    (_: RootState, params: ActivityMonthParams) => params.yearMonth,
    (_: RootState, params: ActivityMonthParams) => params.today,
    (_: RootState, params: ActivityMonthParams) => params.own,
  ],
  (firstDayOfWeek, yearMonth, today, own): ActivityRow[] => {
    const firstOfMonth = LocalDate.of(yearMonth.year(), yearMonth.monthValue(), 1);

    const leadingDays = (firstOfMonth.dayOfWeek().value() - firstDayOfWeek.value() + 7) % 7;
    const trailingDays = (7 - ((leadingDays + yearMonth.lengthOfMonth()) % 7)) % 7;
    const totalDays = leadingDays + yearMonth.lengthOfMonth() + trailingDays;

    const cells = Array.from({ length: totalDays }, (_, index) =>
      buildCell(
        firstOfMonth.plusDays(index - leadingDays),
        own,
        today,
        index < leadingDays || index >= leadingDays + yearMonth.lengthOfMonth(),
      ),
    );

    const rows: ActivityRow[] = [];
    for (let start = 0; start < cells.length; start += 7) {
      rows.push({ key: `week-${start / 7}`, cells: cells.slice(start, start + 7) });
    }
    return rows;
  },
);
