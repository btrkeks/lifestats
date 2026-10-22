import type { CalendarAttention } from '@/db/calendar-sync';
import type { CalendarTarget } from '@/models/calendar-sync';
import type { RootState } from '@/store/store';
import { createAction, createSelector, createSlice, PayloadAction } from '@reduxjs/toolkit';

export type CalendarSyncStatus = { pending: number; attention: CalendarAttention[] };

/** What the settings screen shows. The workouts and their events live in SQLite; nothing here is persisted. */
type CalendarSyncState = CalendarSyncStatus & {
  running: boolean;
  /** The last run stopped because LiftLog has no calendar access. */
  noPermission: boolean;
  lastRunAt: string | undefined;
};

const initialState: CalendarSyncState = {
  pending: 0,
  attention: [],
  running: false,
  noPermission: false,
  lastRunAt: undefined,
};

const calendarSyncSlice = createSlice({
  name: 'calendarSync',
  initialState,
  reducers: {
    calendarStatusRead(state, action: PayloadAction<CalendarSyncStatus>) {
      state.pending = action.payload.pending;
      state.attention = action.payload.attention;
    },
    calendarRunStarted(state) {
      state.running = true;
    },
    calendarRunFinished(state, action: PayloadAction<CalendarSyncStatus & { at: string; noPermission: boolean }>) {
      state.running = false;
      state.pending = action.payload.pending;
      state.attention = action.payload.attention;
      state.noPermission = action.payload.noPermission;
      state.lastRunAt = action.payload.at;
    },
  },
});

export const { calendarStatusRead, calendarRunStarted, calendarRunFinished } = calendarSyncSlice.actions;

/** A session write queued workouts for the calendar. */
export const calendarSessionsQueued = createAction('calendarSync/sessionsQueued');
/** Look again at every workout that needs attention, e.g. after fixing the calendar or granting access. */
export const calendarRetryRequested = createAction('calendarSync/retryRequested');

/**
 * The enabled target for automatic or explicit enrollment. Undefined until settings are loaded and
 * calendar sync is configured and enabled.
 */
export const selectCalendarSyncTarget = createSelector(
  (state: RootState) => state.settings,
  ({ isHydrated, calendarSyncEnabled, calendarSyncCalendar, calendarSyncEventTitle }): CalendarTarget | undefined => {
    const title = calendarSyncEventTitle?.trim();
    return isHydrated && calendarSyncEnabled && calendarSyncCalendar && title
      ? { calendarId: calendarSyncCalendar.id, accountName: calendarSyncCalendar.accountName, title }
      : undefined;
  },
);

export const calendarSyncReducer = calendarSyncSlice.reducer;
