# Android Screen Time Logger — Plan

Status: implementation and USB verification complete (2026-08-05); criteria
1-4 verified on the Xiaomi after setting **Battery saver → No restrictions**.
For criterion 4, a fresh namespaced WorkManager job overwrote the API value
from 75 to 83 while the device still reported deep `IDLE`, before idle was
released.

Follow-up implemented 2026-08-09: the app now preserves this raw total while
optionally logging a second goal-oriented total that excludes user-selected
packages. See `logger_applications/screen_time_logger/INTEGRATION.md` for the
current contract and setup.

## Contract

**Outcome**: An Android app that automatically logs the phone's daily screen
time (minutes, from `UsageStatsManager`) into lifestats once per day, with no
manual interaction after initial setup.

**Scope**:
- New Android app (Kotlin) under `logger_applications/screen_time_logger/`,
  following the external-client pattern of `logger_applications/AnkiLogger`
  (thin client → existing web API; no changes to lifestats core).
- Lifestats side: create one `IntegerAmount` statistic (e.g. name
  "Screen Time", category "Wellness") via `POST /statistics` — done once via
  curl, not code. Log with `POST /statistics/{id}/log/{date}` and
  `update_policy: "overwrite"` (usage stats return a running daily total;
  summing would double-count).
- An `INTEGRATION.md` in the app directory mirroring AnkiLogger's.

**Non-goals** (possible follow-ups, out of scope now):
- Per-app breakdowns or per-session `TimeSpans` logging.
- Any authentication layer on the API.
- Play Store packaging/signing beyond a debug/sideload build.

**Verifiable completion criteria**:
1. `./gradlew assembleDebug` builds; `adb install` succeeds on the connected
   device.
2. After triggering a sync via the debug broadcast (see harness below),
   `GET http://localhost:3000/statistics/{id}/logged/{today}` returns
   `has_value: true` with a plausible minutes value (cross-check against
   Digital Wellbeing manually once).
3. Re-triggering sync overwrites (value stays consistent, does not double).
4. The daily WorkManager job runs under forced Doze
   (`dumpsys deviceidle force-idle` + `cmd jobscheduler run`).

## Design outline

- **Data**: `UsageStatsManager.queryEvents(previousLocalMidnight, now)`, union
  exact resumed-activity intervals clipped to `startOfDayLocal`, then convert
  to minutes. The device proved that `queryAndAggregateUsageStats` can return
  an OEM daily bucket beginning before local midnight (266 minutes versus 56
  in Digital Wellbeing), so bucket summation is not safe. Requires the
  `PACKAGE_USAGE_STATS` special permission (grantable via adb, see harness;
  the app also deep-links to the Usage Access settings screen for non-adb
  setup).
- **Scheduling**: `WorkManager` periodic worker (~every 2–4 h, logging the
  running total with `overwrite`) so the end-of-day value converges even if
  the phone sleeps through midnight. Constraint: any network; retry with
  backoff.
- **Networking**: plain HTTP POST (OkHttp or `HttpURLConnection`). Base URL
  and statistic ID configurable in a minimal settings screen, persisted in
  `SharedPreferences`. Must allow cleartext HTTP to the configured host
  (`networkSecurityConfig`), since the API is plain `http://` on the LAN.
- **Dates**: log against the device-local date (`YYYY-MM-DD`), matching API
  conventions.
- **Debug seam**: a debug-build-only exported `BroadcastReceiver` that runs
  the sync worker immediately — this is what makes the adb loop deterministic.

## Autonomous iteration harness (adb)

Verified available on this machine: adb at `/usr/bin/adb`, SDK at
`~/Android/Sdk`. No `gradle` binary — generate/commit the Gradle wrapper so
builds are `./gradlew`-only. Set `sdk.dir` via `local.properties` or
`ANDROID_HOME=~/Android/Sdk`.

**Target device**: the USB-connected Xiaomi, serial `GQHE9HZHLVRGQOAM`
(model 23078PND5G), already authorized. Two devices are attached, so **every
adb command MUST use `adb -s GQHE9HZHLVRGQOAM`**. Do NOT touch the wireless
device `192.168.69.26:5555` — it is in use for something else.

The full build → install → grant → trigger → verify loop, no human in it:

```bash
export ADB="adb -s GQHE9HZHLVRGQOAM"
./gradlew assembleDebug
$ADB install -r app/build/outputs/apk/debug/app-debug.apk

# Grant the Usage Access special permission without touching the UI:
$ADB shell appops set <pkg> android:get_usage_stats allow

# During dev, route the phone's localhost:3000 to this machine
# (avoids hardcoding LAN IPs):
$ADB reverse tcp:3000 tcp:3000
# → app base URL for dev: http://localhost:3000

# Trigger a sync deterministically (debug receiver):
$ADB shell am broadcast -a <pkg>.SYNC_NOW -n <pkg>/.DebugSyncReceiver

# Observe:
$ADB logcat -d -s ScreenTimeLogger
curl http://localhost:3000/statistics/<id>/logged/$(date +%F)

# Doze/background robustness check:
$ADB shell dumpsys deviceidle force-idle
$ADB shell am broadcast -a <pkg>.SYNC_NOW -n <pkg>/.DebugSyncReceiver
$ADB shell cmd jobscheduler run -f -u 0 \
  -n androidx.work.systemjobscheduler <pkg> <new-job-id>
$ADB shell dumpsys deviceidle unforce
```

UI checks if ever needed: `$ADB exec-out screencap -p > /tmp/screen.png`.

Caveats for the implementing agent:
- If the USB device shows `unauthorized` or disappears, stop and ask Daniel
  to re-plug/confirm the on-device prompt; do not fall back to the wireless
  device.
- The device runs MIUI/HyperOS (Xiaomi). If `adb install` is rejected with a
  MIUI security error, "Install via USB" must be enabled by Daniel in
  Developer Options — escalate rather than working around it. MIUI's
  aggressive battery optimizer can also kill WorkManager jobs; if the Doze
  check fails, report it and suggest exempting the app from battery
  restrictions on-device rather than looping on code changes.
- `adb reverse` does not survive reboots; production config should use the
  machine's LAN/Tailscale address instead — keep base URL configurable.
- The lifestats web API server must be running locally
  (`http://localhost:3000`, no auth).

## Phases

1. **Scaffold**: Gradle project (wrapper committed), empty app with settings
   screen (base URL, statistic ID), builds and installs via adb.
2. **Usage stats reader**: pure Kotlin function computing today's screen-time
   minutes; unit-test the aggregation/date-boundary logic with fakes;
   permission grant via appops verified on device.
3. **API client + sync worker**: POST with `overwrite`, WorkManager periodic
   scheduling, debug `SYNC_NOW` receiver; end-to-end verification against the
   live API per completion criteria 2–3.
4. **Robustness + docs**: retry/backoff, Doze verification (criterion 4),
   `INTEGRATION.md`, register the statistic, update
   `core/infrastructure/web_api/README.md` only if any API observation
   contradicts it (no API changes expected).
