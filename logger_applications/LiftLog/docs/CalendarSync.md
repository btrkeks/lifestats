# Calendar sync

On Android, LiftLog can move the Gym event in your phone's calendar to the time you actually trained.
Each workout you finish moves that day's Gym event to start at your first completed set and end at your
last. A timed cardio set ends when its timer stops. LiftLog writes through the phone's calendar
provider, and your account's own sync (usually Google) uploads the change. LiftLog never signs in to
Google itself.

## Set it up

1. Open **Settings → Calendar sync**. The page exists only on Android.
2. Tap **Allow calendar access**. If you denied it before for good, the same row opens the system
   settings instead.
3. Pick the **Calendar** that holds your Gym events. Each calendar shows its account next to its name.
4. Check the **Event title**. It defaults to `Gym` and must match the event's title exactly, apart from
   spaces at either end. Case matters.
5. Turn on **Move Gym events**.

Sync is off until you turn it on. Turning it on never touches past workouts. Only workouts you finish
from then on are moved.

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
- **Changes made in the calendar win.** If the event's time changed in the calendar since LiftLog last
  moved it, LiftLog leaves it alone and lists the workout. Put the time back and tap **Try again** to
  let LiftLog move it again.
- **Settings changes apply to the next workout you finish.** A finished workout keeps the calendar and
  title it was finished with.
- **Workouts wait while LiftLog cannot write.** Without calendar access, or while sync is off, finished
  workouts stay waiting across restarts. LiftLog moves them when the app starts, comes back to the
  foreground, or a workout is queued.
- **The calendar write is local.** It needs no network. Your account's sync uploads it later, and
  LiftLog cannot see whether that upload happened.

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
  `instanceTime` is `ORIGINAL_INSTANCE_TIME`. A single event stores `-1` there. `syncId` and `rrule`
  record the master as it was when claimed. The native module treats a different value later as a
  different event. `observed` is the time LiftLog last saw or wrote. `attempt` is the span of a write
  that may or may not have reached the provider.

### Enrollment

Only `setActiveSessionId` passes `enrollFinishedIn`, and only when `selectCalendarSyncTarget` returns a
target: sync on, a calendar chosen, a non-empty title, settings loaded, and a phone calendar present.
`queueSessionSync` then enrolls the workout in progress if the write finished it and it has a completed
set. Every session writer re-queues sessions that are already enrolled. Restores, imports, History
edits and **Sync all history** never enroll a workout.

### One run

For each pending workout, `syncCalendarSessions`:

1. Settles a saved `attempt` first. It reads the occurrence. If it shows the attempt, that becomes
   `observed`. If it still shows `observed`, the attempt is dropped. Anything else is `changed`.
2. Records `idle` for a deleted, resumed or empty workout, and writes nothing.
3. For a claimed workout, records `moved-date` if the workout's date is not the claim's date. Otherwise
   it saves the new span as `attempt` and moves the occurrence only if it still shows `observed`.
4. For an unclaimed workout, lists the target calendar's occurrences on the workout's local day. It
   counts every occurrence titled exactly like the target that starts that day, before checking
   whether it can move or who owns it. Zero is `no-match`, two or more is `ambiguous`, and one owned by
   another workout is `claimed`. A single unclaimed match is claimed with the span as `attempt`, in one
   transaction, before the write.

A workout that is blocked or waiting does not stop the run. Missing calendar access stops the run and
leaves every workout pending.

### The native module

`LiftLogCalendarModule.kt` in `modules/native-lib` has four functions: `listCalendars`,
`findInstances`, `inspectOccurrence` and `moveOccurrence`. It never writes as a sync adapter, so the
provider marks every change dirty for upload. `phone-calendar.android.ts` is its only caller and
parses every answer. iOS and web use `phone-calendar.ts`, which reports no support.

`moveOccurrence` re-checks the calendar, its account and access, the master's `_sync_id` and
`RRULE`, and the occurrence's current time before it writes, and runs one write at a time.

- **First move of a series occurrence.** The module inserts an exception through
  `Events.CONTENT_EXCEPTION_URI/<master>` with `ORIGINAL_INSTANCE_TIME`, `DTSTART`, a whole-second
  `DURATION` and the master's `STATUS`. The provider rejects `DTEND` on this insert and marks an
  exception without `STATUS` tentative.
- **Later moves, or a single event.** The module updates the row's `DTSTART` and `DTEND` with a
  selection that requires `RRULE IS NULL`, `RDATE IS NULL` and the times it just read, so it can never
  match a series master.
- **Before inserting**, it looks for an existing exception for the same master and
  `ORIGINAL_INSTANCE_TIME`. One is updated instead. Two or more is `duplicate`. The provider does not
  merge a second insert, so this lookup is what prevents duplicates after a lost answer.
- **A series without `_sync_id`** is `unsynced`. On the provider tested, moving an occurrence of such a
  series dropped its other occurrences from `Instances`.
- After writing, it reads the occurrence back and answers `moved` only if it shows the new time.

The permissions are declared in `modules/native-lib/android/src/main/AndroidManifest.xml` and requested
only from the settings page.
