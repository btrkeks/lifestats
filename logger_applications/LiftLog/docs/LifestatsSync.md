# Lifestats sync

LiftLog can keep the `[LiftLog] Workout Time` statistic in lifestats up to date. Each workout you
finish, edit, move, delete or import updates the workout time of its day. The phone pushes nothing
else: no sets, weights or totals.

## Set it up

1. Open **Settings → Lifestats sync**.
2. Check the **Lifestats URL**. It defaults to `https://laptop-1.tail87da75.ts.net` and must start
   with `https://`.
3. Turn on **Sync workout time**.
4. To send workouts that were finished before you turned sync on, tap **Sync all history**.

The page shows how many days are waiting and the result of the last sync.

### Correct workouts imported from Strong

Workouts imported from Strong before LiftLog read `Duration (sec)` last only seconds. To give them
their real length in LiftLog and in lifestats, turn sync on, then import the same Strong export again
(**Settings → Export, backup, and restore → Import from other apps**). LiftLog corrects their set times and
sends their days. See [CsvImport.md](./CsvImport.md).

## What happens

- A day is waiting from the moment a change to it is saved on the phone. It stays waiting until
  lifestats accepts it, across restarts and with the app closed.
- LiftLog sends the waiting days about 1.5 seconds after a change, when the app starts, when it comes
  back to the foreground, and when you change these settings.
- If lifestats cannot be reached, LiftLog tries again after 15 seconds, 1 minute and then every
  5 minutes while the app stays open. Nothing is sent in the background.
- Each send replaces the workout time of exactly the days it names. A day lifestats already has but
  this phone never changed is left alone.
- A day's workout time is one span per finished workout, from its first completed set to its last.
  A timed cardio set ends when its timer stops. Lifestats merges overlapping spans and counts a span
  shorter than a minute as a minute.
- A day whose last workout you delete, or move to another day, is cleared in lifestats.
- A workout in progress is not sent. Resuming a finished workout removes it from its day until you
  finish it again.
- A workout you forget to finish is finished for you. When the app starts or comes back to the
  foreground, a workout in progress whose last completed set is more than 3 hours old is finished
  as if you had tapped Finish, and its day is sent. The plan is not changed. A workout from an
  earlier day with no completed set is discarded instead.
- While sync is off, changed days keep waiting and are sent once you turn it on.

## Contributor notes

The contract is `PUT {url}/integrations/liftlog/days` with
`{"days":[{"date":"2026-10-11","workouts":[{"id":"…","start":"2026-10-11T18:02:11+02:00","end":"…"}]}]}`.
`workouts: []` clears that day. Lifestats answers `200` with `statistic_id`, `days_updated`,
`days_cleared` and `days_unchanged`, and `400 {"error": …}` for a malformed request. The server side
is documented in the lifestats repository (`docs/liftlog-integration.md`).

```text
session write (stored-sessions effects)
  → queueSessionSync(tx, ids, write)        db/session-sync.ts, same transaction as the write
  → lifestatsDaysQueued                     wakes the sync effect
  → applyLifestatsSyncEffects               store/lifestats-sync/effects.ts: debounce, one push at a time, backoff
  → pushQueuedDays                          services/lifestats-sync.ts: 100 days per request, ack on success
```

- **The queue** is the `lifestats_sync_queue` table: one row per day, with a `seq` that every enqueue
  bumps. A push reads `(date, seq)` pairs and deletes only rows whose `seq` still matches, so an edit
  that lands during a push stays queued. The payload is never stored. Each push rebuilds a day from
  the `session` rows (`syncDay`, `sessionSpan`).
- **What queues a day.** A session counts on its day when it is finished and has a completed set
  (`readWorkoutDates`). `queueSessionSync` reads those dates for the sessions a write touches
  before and after the write, and queues both sets. That covers History edits and date moves
  (`putStoredSession` / `updateStoredSession`), deletes, imports and restores
  (`upsertStoredSessions`), and finishing and resuming (`setActiveSessionId`, in the transaction that
  flips `active`). Starting a workout and closing History unchanged queue nothing.
- **Automatic finishing** dispatches `sessionFinished`, so it queues the day through the same
  `setActiveSessionId` transaction. The decision is `forgottenWorkoutAction`
  (`store/stored-sessions/forgotten-workout.ts`), and the limit is `FORGOTTEN_WORKOUT_IDLE_LIMIT`.
- **A new session write** that can change a finished session must go through `queueSessionSync`
  in its transaction, or lifestats will not hear about it. The same call queues calendar sync
  ([CalendarSync.md](./CalendarSync.md)).
- **Delivery order.** The queue protects acknowledgements, not the order in which lifestats applies
  requests. A request delayed past its 15-second timeout could still land after a newer one for the
  same day. This is accepted for one phone on a tailnet.
- **Only lifestats' answer acknowledges.** A `2xx` without a numeric `statistic_id` (a captive portal,
  a wrong URL) counts as a failure, so no day is lost to it.
- The preferences are `lifestatsSyncEnabled` and `lifestatsSyncUrl`. Status (`pendingDays`,
  `pushing`, `lastPush`) is in the `lifestatsSync` slice and is not persisted; the pending count is
  re-read from the table on launch. The queue is left out of backups (`getBackupBytes`).
