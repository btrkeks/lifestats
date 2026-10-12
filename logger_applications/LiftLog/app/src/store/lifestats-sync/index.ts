import type { PushOutcome } from '@/services/lifestats-sync';
import { createAction, createSlice, PayloadAction } from '@reduxjs/toolkit';

/** What the settings screen shows. The queue itself lives in SQLite; nothing here is persisted. */
type LifestatsSyncState = {
  pendingDays: number;
  pushing: boolean;
  lastPush: { at: string; outcome: PushOutcome } | undefined;
};

const initialState: LifestatsSyncState = { pendingDays: 0, pushing: false, lastPush: undefined };

const lifestatsSyncSlice = createSlice({
  name: 'lifestatsSync',
  initialState,
  reducers: {
    pendingDaysCounted(state, action: PayloadAction<number>) {
      state.pendingDays = action.payload;
    },
    pushStarted(state) {
      state.pushing = true;
    },
    pushFinished(state, action: PayloadAction<{ at: string; outcome: PushOutcome; pendingDays: number }>) {
      state.pushing = false;
      state.pendingDays = action.payload.pendingDays;
      state.lastPush = { at: action.payload.at, outcome: action.payload.outcome };
    },
  },
});

export const { pendingDaysCounted, pushStarted, pushFinished } = lifestatsSyncSlice.actions;

/** A session write queued days for lifestats. */
export const lifestatsDaysQueued = createAction('lifestatsSync/daysQueued');
/** Queue every day that has a finished workout, e.g. on first setup or after restoring a backup. */
export const lifestatsSyncAllRequested = createAction('lifestatsSync/syncAllRequested');

export const lifestatsSyncReducer = lifestatsSyncSlice.reducer;
