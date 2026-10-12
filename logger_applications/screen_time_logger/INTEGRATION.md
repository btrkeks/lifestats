# Android Screen Time Logger Integration

This app reads timestamped foreground-activity events from Android's
`UsageStatsManager` and overwrites up to two Lifestats `IntegerAmount` values
every four hours:

- **Total screen time** includes every foreground app and preserves the raw
  device-usage observation.
- **Goal screen time** excludes user-selected packages, such as AnkiDroid,
  while retaining them in the total. This statistic is optional so existing
  installations continue syncing until it is configured.

Overwrite is intentional: Android reports running totals for the day, so
summing repeated syncs would double-count screen time.

## Architecture

The app is a thin external client; it does not add a Lifestats backend route.

```text
UsageStatsManager
        |
        v
Usage events + goal-exempt packages
                 |
                 v
         ScreenTimeReader
          |             |
          v             v
    total minutes   goal minutes
          \             /
           ScreenTimeSyncWorker --> POST /statistics/{id}/log/{date}
                    ^                    update_policy=overwrite
                    |
          WorkManager every 4 hours
```

- `MainActivity` stores the base URL, total statistic ID, optional goal
  statistic ID, and goal-exempt package set in private `SharedPreferences`.
  It opens Android's Usage Access settings and offers an app picker populated
  from packages observed in the last 30 days. This avoids requesting broad
  package-visibility access.
- `ScreenTimeReader` computes local midnight and unions resumed-activity
  intervals from `UsageEvents`, clipped to the current local day. It seeds
  cross-midnight state from the previous day and clears state at screen-off
  and reboot boundaries before flooring the total to whole minutes. Exact
  events are required because OEM "daily" aggregate buckets can begin before
  local midnight. An interval contributes to goal screen time when at least
  one active foreground package is not exempt, preserving split-screen and
  overlay behavior.
- `ScreenTimeSyncWorker` always sends the raw total and sends the goal total
  when a goal statistic ID is configured. Network, HTTP 408/429, and server
  failures retry with exponential backoff.
- `SyncScheduler` maintains one unique periodic job with a connected-network
  constraint. Saving settings also queues an immediate sync.
- `DebugSyncReceiver` exists only in debug builds, requires Android's
  shell/system-only `DUMP` permission, and queues an immediate sync for
  deterministic ADB testing.

## One-time Lifestats setup

Start the API and check whether the statistic already exists:

```bash
cargo run -p lifestats-web-api
curl -sS http://localhost:3000/statistics
```

Keep the existing `Screen Time` `IntegerAmount` as the raw total, or create it
if it does not exist:

```bash
curl -sS -X POST http://localhost:3000/statistics \
  -H 'Content-Type: application/json' \
  -d '{"name":"Screen Time","category":"Wellness","statistic_type":"IntegerAmount"}'
```

Keep the returned `id`. In the app, enter the API address reachable from the
phone as the **Total screen-time statistic ID**.

Create a second statistic for the goal-oriented value:

```bash
curl -sS -X POST http://localhost:3000/statistics \
  -H 'Content-Type: application/json' \
  -d '{"name":"Distracting Screen Time","category":"Wellness","statistic_type":"IntegerAmount"}'
```

Enter its returned `id` as the **Goal screen-time statistic ID**. Open Usage
Access settings, grant access to Lifestats Screen Time, refresh the recent-app
list, and select apps that should not count toward the goal. For example,
AnkiDroid normally appears as `com.ichi2.anki`. Tap **Save and sync now**. No
interaction is needed after this setup unless the exemptions change.

Upgrading an existing installation preserves the original statistic ID and
continues logging total screen time. The goal statistic remains disabled until
its ID is entered; saved goal-exempt apps take effect on the next sync.

For normal use, configure the machine's LAN or Tailscale address. Android's
`localhost` means the phone itself and only works during development while an
ADB reverse is active.

## Build and unit tests

The committed Gradle wrapper is the only Gradle installation required. The app
uses compile/target SDK 36 and min SDK 23.

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
./gradlew testDebugUnitTest assembleDebug
```

The APK is written to `app/build/outputs/apk/debug/app-debug.apk`.

## Deterministic USB verification

The wireless device is reserved for other work. Every command below goes
through a helper that always supplies the approved USB serial.

```bash
SERIAL=GQHE9HZHLVRGQOAM
PKG=dev.lifestats.screentimelogger
adb_usb() { /usr/bin/adb -s "$SERIAL" "$@"; }

./gradlew assembleDebug
adb_usb install -r app/build/outputs/apk/debug/app-debug.apk
adb_usb shell appops set "$PKG" android:get_usage_stats allow
adb_usb reverse tcp:3000 tcp:3000
adb_usb shell am start -n "$PKG/.MainActivity"

# Optional extras configure a fresh debug install without UI interaction.
# Use the IDs returned by the one-time statistic creation requests.
TOTAL_STATISTIC_ID=42
GOAL_STATISTIC_ID=43
adb_usb shell am broadcast \
  -a "$PKG.SYNC_NOW" \
  -n "$PKG/.DebugSyncReceiver" \
  --es base_url http://localhost:3000 \
  --es statistic_id "$TOTAL_STATISTIC_ID" \
  --es goal_statistic_id "$GOAL_STATISTIC_ID" \
  --es goal_exempt_packages com.ichi2.anki

adb_usb shell am wait-for-broadcast-idle
adb_usb logcat -d -s ScreenTimeLogger
for STATISTIC_ID in "$TOTAL_STATISTIC_ID" "$GOAL_STATISTIC_ID"; do
  for _ in $(seq 1 20); do
    RESPONSE=$(curl -sS \
      "http://localhost:3000/statistics/$STATISTIC_ID/logged/$(date +%F)")
    printf '%s\n' "$RESPONSE"
    printf '%s' "$RESPONSE" | grep -q '"has_value":true' && break
    sleep 1
  done
done
```

Run the broadcast and GET both statistics a second time. Each value should
remain close to its first value rather than doubling, and the goal value must
not exceed the total. Compare the raw total with Digital Wellbeing once; small
differences are expected because the app floors partial minutes and Android
updates usage events on its own cadence.

## Doze and WorkManager verification

Queue a fresh debug sync in forced idle, locate its namespaced WorkManager
JobScheduler ID, then force that exact job. Forcing the periodic job itself
before its four-hour window only makes WorkManager reschedule it, by design.
Always leave forced idle, including after a failed check.

```bash
leave_doze() { adb_usb shell cmd deviceidle unforce >/dev/null; }
trap leave_doze EXIT

adb_usb shell cmd deviceidle force-idle
adb_usb shell cmd deviceidle get deep  # IDLE
adb_usb shell am broadcast \
  -a "$PKG.SYNC_NOW" \
  -n "$PKG/.DebugSyncReceiver"

JOB_ID=$(adb_usb shell dumpsys jobscheduler | sed -n \
  "s|.*JOB androidx\\.work\\.systemjobscheduler:[^/]*/\\([0-9][0-9]*\\):.*@androidx\\.work\\.systemjobscheduler@$PKG/.*|\\1|p" \
  | sort -n | tail -n 1)
test -n "$JOB_ID"
adb_usb shell cmd jobscheduler run -f -u 0 \
  -n androidx.work.systemjobscheduler "$PKG" "$JOB_ID"
adb_usb logcat -d -s ScreenTimeLogger

leave_doze
trap - EXIT
```

If MIUI/HyperOS prevents installation, enable **Install via USB** in Developer
Options. If **Remember my choice** was combined with **Deny**, search Android
Settings for **Install via USB (Security)** and change the denied entry to
Allow. If the install log says `Unknown authority guard`, confirm the system
Guard Provider is inactive before restoring it for user 0:

```bash
adb_usb shell dumpsys package providers  # should contain [guard]
adb_usb shell cmd package install-existing --user 0 com.miui.guardprovider
```

This restores Xiaomi's security provider; it does not bypass package checks.
If HyperOS accepts the JobScheduler command but defers worker execution until
idle is released, set **Battery saver → No restrictions** for Lifestats Screen
Time and repeat the check. Do not substitute the wireless device. `adb reverse`
must be restored after each phone reboot.

## Troubleshooting

- `Usage Access is required`: grant access in Android settings or run the
  documented `appops` command for a debug install.
- HTTP 404: confirm that the configured statistic ID exists.
- HTTP 400: confirm that the statistic is an `IntegerAmount`.
- Goal screen time is not logged: enter a goal statistic ID and save again.
- An app is missing from the exemption list: use it once, return to the logger,
  and tap **Refresh recent apps**. Only packages observed in the last 30 days
  are listed, plus previously saved selections.
- Goal time unexpectedly matches total time: confirm the intended packages are
  checked and that their package names match the usage-event entries.
- Connection refused with `http://localhost:3000`: start the API and recreate
  the ADB reverse, or configure a reachable LAN/Tailscale URL.
- A value much larger than Digital Wellbeing can indicate an OEM aggregate
  bucket crossing midnight. The logger intentionally uses exact `UsageEvents`,
  not `queryAndAggregateUsageStats`, to avoid that overcount.
- A transient network or server error is retried automatically. Corrected
  settings take effect immediately after tapping **Save and sync now**.
