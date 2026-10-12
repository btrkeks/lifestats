import { countSyncQueue, enqueueSyncDates } from '@/db/lifestats-sync';
import { readWorkoutDates } from '@/db/sessions';
import { pushQueuedDays, syncBaseUrl } from '@/services/lifestats-sync';
import { appForegroundChanged } from '@/store/app';
import {
  lifestatsDaysQueued,
  lifestatsSyncAllRequested,
  pendingDaysCounted,
  pushFinished,
  pushStarted,
} from '@/store/lifestats-sync';
import { setLifestatsSyncEnabled, setLifestatsSyncUrl } from '@/store/settings';
import { setIsHydrated as setStoredSessionsHydrated } from '@/store/stored-sessions';
import { AddEffectFn, AppDispatch, RootState } from '@/store/store';
import { Services } from '@/services';
import { Instant } from '@js-joda/core';

// Lets a burst of History edits land before one push carries them all.
const WAKE_DELAY_MS = 1_500;
// After a push that may succeed later (offline, timeout, server down), while the app is in the foreground.
const RETRY_DELAYS_MS = [15_000, 60_000, 300_000];

type SyncApi = { dispatch: AppDispatch; getState: () => RootState; extra: Services };

/**
 * Pushes the queued days to lifestats: shortly after anything is queued, on launch, on coming back to the
 * foreground and on a settings change, then with backoff while a retryable failure leaves days queued.
 * At most one push runs; a wake during it runs one more pass after it.
 */
export function applyLifestatsSyncEffects(addEffect: AddEffectFn) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pushing = false;
  let again = false;
  let failures = 0;
  let foreground = true;

  function wakeAfter(api: SyncApi, delayMs: number) {
    clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      void push(api);
    }, delayMs);
  }

  async function push(api: SyncApi) {
    if (pushing) {
      again = true;
      return;
    }
    pushing = true;
    try {
      do {
        again = false;
        const { db, logger } = api.extra;
        const { settings, storedSessions } = api.getState();
        const baseUrl = syncBaseUrl(settings.lifestatsSyncUrl);
        if (!settings.lifestatsSyncEnabled || !baseUrl || !storedSessions.isHydrated || countSyncQueue(db) === 0) {
          return;
        }
        api.dispatch(pushStarted());
        const outcome = await pushQueuedDays({ db, baseUrl, fetch: globalThis.fetch });
        const pendingDays = countSyncQueue(db);
        api.dispatch(pushFinished({ at: Instant.now().toString(), outcome, pendingDays }));
        if (outcome.kind === 'failed') {
          logger.warn('Lifestats sync failed', { ...outcome, pendingDays });
        }
        failures = outcome.kind === 'failed' && outcome.retryable && pendingDays > 0 ? failures + 1 : 0;
        if (failures && foreground && !again) {
          wakeAfter(api, RETRY_DELAYS_MS[Math.min(failures, RETRY_DELAYS_MS.length) - 1]!);
        }
      } while (again);
    } finally {
      pushing = false;
    }
  }

  function countAndWake(api: SyncApi) {
    api.dispatch(pendingDaysCounted(countSyncQueue(api.extra.db)));
    wakeAfter(api, WAKE_DELAY_MS);
  }

  addEffect([lifestatsDaysQueued, setStoredSessionsHydrated, setLifestatsSyncEnabled, setLifestatsSyncUrl], (_, api) =>
    countAndWake(api),
  );

  addEffect(lifestatsSyncAllRequested, (_, api) => {
    const { db } = api.extra;
    db.transaction((tx) => enqueueSyncDates(tx, readWorkoutDates(tx)));
    countAndWake(api);
  });

  addEffect(appForegroundChanged, (action, api) => {
    foreground = action.payload;
    if (foreground) {
      failures = 0;
      countAndWake(api);
    } else {
      clearTimeout(timer);
      timer = undefined;
    }
  });
}
