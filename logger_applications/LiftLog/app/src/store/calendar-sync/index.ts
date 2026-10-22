import type { CalendarAttention } from '@/db/calendar-sync';
import type { CalendarTarget } from '@/models/calendar-sync';
import type { RootState } from '@/store/store';
import { createAction, createSlice, PayloadAction } from '@reduxjs/toolkit';

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
 * Where a workout finished now would look for its event. Undefined while sync is off, not set up, or the
 * settings have not loaded, so a workout finished then is never enrolled.
 */
export function selectCalendarSyncTarget(state: RootState): CalendarTarget | undefined {
  const { isHydrated, calendarSyncEnabled, calendarSyncCalendar, calendarSyncEventTitle } = state.settings;
  const title = calendarSyncEventTitle?.trim();
  return isHydrated && calendarSyncEnabled && calendarSyncCalendar && title
    ? { calendarId: calendarSyncCalendar.id, accountName: calendarSyncCalendar.accountName, title }
    : undefined;
}

export const calendarSyncReducer = calendarSyncSlice.reducer;
