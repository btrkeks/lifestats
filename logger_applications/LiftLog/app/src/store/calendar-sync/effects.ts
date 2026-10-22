import { countPendingCalendarSessions, readCalendarAttention, requeueCalendarAttention } from '@/db/calendar-sync';
import { Services } from '@/services';
import { syncCalendarSessions } from '@/services/calendar-sync';
import { appForegroundChanged } from '@/store/app';
import {
  calendarRetryRequested,
  calendarRunFinished,
  calendarRunStarted,
  calendarSessionsQueued,
  calendarStatusRead,
  CalendarSyncStatus,
} from '@/store/calendar-sync';
import { setCalendarSyncEnabled } from '@/store/settings';
import { setIsHydrated as setStoredSessionsHydrated } from '@/store/stored-sessions';
import { AddEffectFn, AppDispatch, RootState } from '@/store/store';
import { Instant } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

// Lets a burst of History edits land before one run handles them all.
const WAKE_DELAY_MS = 1_500;
// While a workout waits on something that may clear by itself (a calendar sync, a provider error).
const RETRY_DELAYS_MS = [15_000, 60_000, 300_000];

type SyncApi = { dispatch: AppDispatch; getState: () => RootState; extra: Services };

const readStatus = (db: ExpoSQLiteDatabase): CalendarSyncStatus => ({
  pending: countPendingCalendarSessions(db),
  attention: readCalendarAttention(db),
});

/**
 * Moves the events of pending workouts: shortly after a workout is queued, on launch, on coming back to the
 * foreground (which is also when a permission granted in system settings shows up), when sync is turned on
 * and on Try again, then with backoff while a workout waits. At most one run at a time; a wake during it
 * runs one more pass after it. Turning sync off pauses runs and keeps every pending workout.
 */
export function applyCalendarSyncEffects(addEffect: AddEffectFn) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running = false;
  let again = false;
  let failures = 0;
  let foreground = true;

  function wakeAfter(api: SyncApi, delayMs: number) {
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      void run(api);
    }, delayMs);
  }

  async function run(api: SyncApi) {
    if (running) {
      again = true;
      return;
    }
    running = true;
    try {
      do {
        again = false;
        const { db, phoneCalendar, logger } = api.extra;
        const { settings, storedSessions } = api.getState();
        if (
          !phoneCalendar.supported ||
          !settings.calendarSyncEnabled ||
          !storedSessions.isHydrated ||
          countPendingCalendarSessions(db) === 0
        ) {
          return;
        }
        api.dispatch(calendarRunStarted());
        const result = await syncCalendarSessions({ db, calendar: phoneCalendar, logger });
        api.dispatch(
          calendarRunFinished({ at: Instant.now().toString(), noPermission: result.noPermission, ...readStatus(db) }),
        );
        failures = result.retry ? failures + 1 : 0;
        if (failures && foreground && !again) {
          wakeAfter(api, RETRY_DELAYS_MS[Math.min(failures, RETRY_DELAYS_MS.length) - 1]!);
        }
      } while (again);
    } finally {
      running = false;
    }
  }

  function readAndWake(api: SyncApi) {
    api.dispatch(calendarStatusRead(readStatus(api.extra.db)));
    wakeAfter(api, WAKE_DELAY_MS);
  }

  addEffect([calendarSessionsQueued, setStoredSessionsHydrated, setCalendarSyncEnabled], (_, api) => readAndWake(api));

  addEffect(calendarRetryRequested, (_, api) => {
    api.extra.db.transaction((tx) => requeueCalendarAttention(tx));
    failures = 0;
    readAndWake(api);
  });

  addEffect(appForegroundChanged, (action, api) => {
    foreground = action.payload;
    if (foreground) {
      failures = 0;
      readAndWake(api);
    } else {
      clearTimeout(timer);
      timer = undefined;
    }
  });
}
