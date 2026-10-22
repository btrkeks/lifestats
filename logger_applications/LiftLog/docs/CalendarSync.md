# Calendar sync

On Android, LiftLog can move the Gym event in your phone's calendar to the time you actually trained.
Each workout you finish moves that day's Gym event to start at your first completed set and end at your
last. A timed cardio set ends when its timer stops. LiftLog writes through the phone's calendar
provider. Your Google account's sync handles upload separately. LiftLog never signs in to Google itself
and cannot verify cloud delivery.

## Set it up

1. Open **Settings → Calendar sync**. The page exists only on Android.
2. Tap **Allow calendar access**. If Android reports permanent denial, that same button press opens
   the system settings. Calendar access and the calendar list refresh when you return to this page from
   system settings, including when the route stays focused.
3. Pick the **Calendar** that holds your Gym events. The picker lists writable Google calendars, including
   secondary calendars. Each calendar shows its account next to its name.
4. Check the **Event title**. It defaults to `Gym` and must match the event's title exactly, apart from
   spaces at either end. Case matters.
5. Turn on **Move Gym events**.

Sync is off until you turn it on. Turning it on never touches past workouts. Only workouts you finish
from then on are moved automatically. To include older workouts, use the explicit preview below.

## What happens

- **Only one occurrence moves.** For a repeating Gym series, LiftLog moves that day's occurrence and
  leaves every other occurrence and the series itself alone.
- **LiftLog never creates or deletes an event.** A day without a Gym event stays without one.
- **A workout keeps its event.** Editing the workout in History moves the same event again, as long as
  the workout stays on that day.
- **Resuming, deleting or emptying a workout leaves its event where it is.** The event stays that
  workout's, so another workout on the same day cannot take it.
- **Moving a workout to another day writes nothing.** Its event stays on the old day, and the workout is
  listed under **Needs attention**. Move the workout back and LiftLog follows it again.
- **Changes made in the calendar win.** If the event's time, title, organizer, or source series timing
  changed since LiftLog last inspected it, LiftLog leaves it alone and lists the workout. Restore the
  previous values and tap **Try again** to let LiftLog move it again.
- **Settings changes apply to the next workout you finish.** A finished workout keeps the calendar and
  title it was finished with.
- **Workouts wait while LiftLog cannot write.** Without calendar access, or while sync is off, finished
  workouts stay waiting across restarts. LiftLog moves them when the app starts, comes back to the
  foreground, or a workout is queued.
- **The calendar write is local.** It needs no network. Your account's sync uploads it later, and
  LiftLog cannot see whether that upload happened.

## Backfill past workouts

1. Configure the calendar and exact event title, grant calendar access, and turn calendar sync on.
2. Open **Backfill workout history** from Calendar sync settings.
3. Review the **From** and **Through** dates. They default to the first and last finished workout dates
   with completed sets. Both dates are inclusive. Edit them as `YYYY-MM-DD`.
4. Tap **Preview history**. This reads workouts and calendar occurrences without changing either.
5. Review the counts, workout names and dates, and each matching event's current time versus the workout
   time. Choose individual **Ready** workouts, or tap **Select all ready**. Nothing starts selected.
6. Tap **Apply N selected workouts** to queue only those rows. The results show queued and skipped
   workouts. Calendar sync's normal waiting and **Needs attention** status track delivery afterward.

Changing the calendar, title, enabled state, or date range invalidates the preview and selection.
Already-linked workouts are left alone, including ones waiting or needing attention from an earlier
run. Active, empty, and zero-length workouts are excluded. Missing, ambiguous, unsupported, or claimed
events cannot be selected. When multiple workouts would take the same occurrence, all of that group is
shown as conflicting and skipped. No event is created, unlinked, deleted, or restored.

Apply checks permission, the current configuration and dates, and each workout again. Changed or
removed workouts are skipped. It saves the exact previewed occurrence and its expected current time;
it never searches for a replacement at delivery time. If that event changes outside LiftLog before
delivery, normal calendar sync leaves the external edit alone and reports it. Repeating Apply does not
change an already-enrolled workout or take its event twice.

### Needs attention

The page lists each workout LiftLog could not finish with, by its day, and why. **Try again** looks at
all of them again.

| Reason shown                          | What to do                                                       |
| ------------------------------------- | ---------------------------------------------------------------- |
| No event with this title on this day  | Add the event or fix its title, then tap Try again.             |
| More than one event with this title   | Leave one event with this title on that day, then tap Try again. |
| Already shows another workout         | Nothing. Each event belongs to the first workout that moved it. |
| All day, read-only or someone else's  | LiftLog does not move these events.                              |
| Moved to another day                  | Move the workout back, or leave the event as it is.             |
| Time was changed in the calendar      | Put the old time back and tap Try again, or leave it.            |
| Deleted or its series changed         | Nothing. LiftLog does not recreate events.                       |
| Two copies of this event              | Delete one copy, then tap Try again.                             |
| Calendar is gone or read-only         | Check the calendar's account on the phone.                       |
| Waiting for the series to sync        | Let the phone sync the account. LiftLog retries by itself.       |
| Could not be updated                  | LiftLog retries by itself while the app is open.                 |

### Google reminder inheritance

On the Google sync adapter tested, creating a moved occurrence preserved the current alert time but
converted a calendar-default reminder into an explicit reminder on that occurrence. LiftLog's device
calendar integration does not expose Google's reminder-inheritance flag, so inherited reminders are not
guaranteed to follow later calendar-default changes. Existing explicit reminders retain their current
settings. A time-only historical operation can preserve the original Google reminder representation by
reconciling the saved before-state through Google Calendar after upload.

## Contributor notes

```text
setActiveSessionId effect (finishing a workout)
  → queueSessionSync(tx, ids, write, { enrollFinishedIn })   db/session-sync.ts, same transaction as the write
  → calendarSessionsQueued                                   wakes the calendar effect
  → applyCalendarSyncEffects                                 store/calendar-sync/effects.ts: debounce, one run at a time, backoff
  → syncCalendarSessions                                     services/calendar-sync.ts: per workout, settle, match, move
  → PhoneCalendar                                            services/phone-calendar.android.ts → LiftLogCalendarModule.kt
```

### Two tables

`db/calendar-sync.ts` is the only code that reads or writes them. Backups leave both out
(`getBackupBytes` clears `calendar\_%`), because their ids name rows in this phone's calendar provider.

- **`calendar_sync_session`** has one row per enrolled workout, with no foreign key, so it outlives a
  deleted workout. It holds the calendar id, account and title from when the workout was finished. `seq`
  moves on every later write of the workout. `doneSeq` is the `seq` the last settled attempt read, so a
  workout is pending while `seq > doneSeq`, and an edit landing during a calendar write stays pending.
  `outcome` and `outcomeDate` hold what the last attempt found (`calendarOutcomes` in
  `models/calendar-sync.ts`).
- **`calendar_claim`** is the occurrence a workout owns. A unique index on `(calendarId, eventId,
  instanceTime)` stops two workouts from owning one occurrence. For a series, `eventId` is the master and
  `instanceTime` is `ORIGINAL_INSTANCE_TIME`. A single event stores `-1` there. `syncId`, `rrule`, title,
  organizer, source start, duration, and timezone
  record the source when claimed. A mismatch blocks a later write. Source start and duration are pinned
  only for series, since single-event times move with the workout. `observed` is the time LiftLog last saw
  or wrote. `attempt` is the span of a write
  that may or may not have reached the provider.

### Enrollment

For automatic enrollment, only `setActiveSessionId` passes `enrollFinishedIn`, and only when `selectCalendarSyncTarget` returns a
target: sync on, a calendar chosen, a non-empty title, settings loaded, and a phone calendar present.
`queueSessionSync` then enrolls the workout in progress if the write finished it and it has a completed
set. Every session writer re-queues sessions that are already enrolled. Restores, imports, History
edits and Lifestats **Sync all history** never automatically enroll a workout.

### Explicit history enrollment

`models/calendar-backfill.ts` defines `BackfillPreview`, with a captured target and inclusive range,
workout IDs/dates/names, exact completed spans, widened desired spans, and matched occurrence identities
and current spans. `previewCalendarBackfill` in `services/calendar-backfill.ts` uses the existing day and
title matching rules. It counts matches before checking suitability or ownership and caches one lookup
per day. It performs no database or provider writes.

`applyCalendarBackfill` rereads permission before touching SQLite, then rereads the live request and
finished workout inside each transaction. It inserts a session row only when none exists and pins the
preview claim, original/observed times, and desired attempt in the same transaction. A failed claim
rolls back enrollment. The existing unique claim index arbitrates competing writers. No provider write
runs inside a transaction. Only newly queued rows cause the page to dispatch `calendarSessionsQueued`.
The existing runner reconciles the saved attempt and checks the pinned occurrence before delivery.
No new table or migration is involved.

### One run

For each pending workout, `syncCalendarSessions`:

1. Settles a saved `attempt` first. It reads the occurrence. If it shows the attempt, that becomes
   `observed`. If it still shows `observed`, the attempt is dropped. Anything else is `changed`.
2. Records `idle` for a deleted, resumed or empty workout, and writes nothing.
3. For a claimed workout, records `moved-date` if the workout's date is not the claim's date. Otherwise
   it inspects the occurrence, even when the requested span equals `observed`. External changes remain
   visible. If a move is needed, it saves the new span as `attempt` and moves the occurrence only if it
   still shows `observed`.
4. For an unclaimed workout, lists the target calendar's occurrences on the workout's local day. It
   counts every occurrence titled exactly like the target that starts that day, before checking
   whether it can move or who owns it. Zero is `no-match`, two or more is `ambiguous`, and one owned by
   another workout is `claimed`. A single unclaimed match is claimed with the span as `attempt`, in one
   transaction, before the write.

The drain checks foreground, settings and session hydration, and enabled state before each job and write.
After a lookup or inspection, it rereads the session and its revision. A changed revision stays pending
without claiming or sending the obsolete span. Saved attempts are reconciled before reading the newest
workout, so recovery never replays a stale span.

A workout that is blocked or waiting does not stop the run. Missing calendar access stops the run and
leaves every workout pending.

### The native module

`LiftLogCalendarModule.kt` in `modules/native-lib` has four functions: `listCalendars`,
`findInstances`, `inspectOccurrence` and `moveOccurrence`. It never writes as a sync adapter, so the
provider can mark changes dirty for the account sync adapter. Dirty state does not prove cloud delivery.
`phone-calendar.android.ts` is its only caller and
parses every answer. iOS and web use `phone-calendar.ts`, which reports no support.

`moveOccurrence` re-checks the calendar, its account and access, the master's `_sync_id` and
`RRULE`, title, organizer, source timing, timezone, current edit access, and the occurrence's time before
it writes, and runs one write at a time. RDATE, EXRULE, and EXDATE recurrence variants are not written.
All-day matches use their UTC calendar date and still count toward ambiguity. Queries cover an extra
day at both ends so offsets in either direction cannot hide those matches.

- **First move of a series occurrence.** The module inserts an exception through
  `Events.CONTENT_EXCEPTION_URI/<master>` with `ORIGINAL_INSTANCE_TIME`, `DTSTART`, a whole-second
  `DURATION` and the master's `STATUS`, or confirmed when it is null. The provider rejects `DTEND`
  on this insert.
- **Later moves, or a single event.** The module updates the row's `DTSTART` and `DTEND` with a
  selection on `Events.CONTENT_URI` that requires the row ID, calendar, original master and instance
  identity, null or empty recurrence fields, current title, organizer, and editability, and the times it just
  read. It cannot match a series master. Nullable snapshot fields use `IS NULL` without a bind argument;
  non-null fields bind strings. Nullable status and timezone do not use equality guards.
- **Before inserting**, it looks for an existing exception for the same master and
  `ORIGINAL_INSTANCE_TIME`. One is updated instead. Two or more is `duplicate`. The provider does not
  merge a second insert, so this lookup is what prevents duplicates after a lost answer.
- **A series without `_sync_id`** is `unsynced`. On the provider tested, moving an occurrence of such a
  series dropped its other occurrences from `Instances`.
- After writing, it reads the occurrence back and answers `moved` only if it shows the new time.

The permissions are declared in `modules/native-lib/android/src/main/AndroidManifest.xml` and requested
only from the settings page. An old binary without the optional module reports no support and can
still start.
