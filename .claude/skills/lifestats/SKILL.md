---
name: lifestats
description: Interact with Daniel's personal Lifestats habit/wellness tracker via its local web API (localhost:3000). Use when asked to log habits, exercise, reading, sleep, or study time; check daily progress, goals, streaks, or stats; manage progress items (books); or anything mentioning "lifestats". No repo checkout needed — everything works over HTTP.
---

# Lifestats Web API

Personal observability platform tracking daily habits and wellness metrics. The API server runs locally at `http://localhost:3000` (SQLite behind it, source at `~/.local/share/src/lifestats` — only needed for development, not for using the API).

Quick liveness check: `curl -s http://localhost:3000/statistics` — if it fails, the server isn't running (`cargo run -p lifestats-web-api` from the repo).

## Conventions

- Dates: `YYYY-MM-DD`. Times: `HH:MM` (24h; `HH:MM:SS` also accepted). Get today with `date +%F`.
- All date ranges are **inclusive on both ends**.
- Errors return `{"error": "message"}` with standard status codes (400 validation/domain, 404 missing, 500 internal).
- Statistic names are mostly **Japanese** (e.g. 読書時間 = reading time, 睡眠時間 = sleep). Resolve names to IDs first — never guess IDs.

## Domain rules (read before writing data)

- **Statistic types** — logable: `IntegerAmount`, `Habit`, `PointInTime`, `TimeSpans`; derived: `Summing`, `TimeDifference`; source-owned: `TimedAmounts` (a per-day list of `{time, amount}` entries, e.g. caffeine in mg, written only by the OpenNutriTracker sync). Logging to a derived or `TimedAmounts` statistic returns 400. `GET /statistics/loggable` lists only the manually logable ones. Expect new types to appear; treat an unknown `statistic_type` as read-only rather than failing.
- **Goals**: a statistic has an optional default goal with a history: each change applies from a "valid since" date onwards, and earlier days keep the goal they had. A daily goal for a specific date overrides the default for that date only.
- **Progress items** are separate from statistics: bounded, absolute-position things (books read in pages/characters) logged via checkpoints, not daily values.

## Finding statistics

```bash
curl -s http://localhost:3000/statistics           # all: [{id, name, category, statistic_type, default_goal}] (default_goal is null when unset)
curl -s http://localhost:3000/statistics/loggable  # excludes derived and TimedAmounts
curl -s http://localhost:3000/statistics/42        # one
```

## Logging values

`POST /statistics/{id}/log/{date}` with `{"value": "<string>", "update_policy": "<optional>"}`. Returns 204. The `value` is always a **string**; its format depends on the statistic type:

| Type | `value` format | Policies (case-insensitive) | Default |
|------|---------------|------------------------------|---------|
| IntegerAmount | `"50"` | `sum`, `overwrite`, `ignore` | `sum` |
| Habit | `"true"` / `"false"` | `overwrite`, `ignore` | `overwrite` |
| PointInTime | `"23:15"` | `overwrite`, `ignore` | `overwrite` |
| TimeSpans | don't use this route — see below | | |
| TimedAmounts | not loggable here (400) — synced by OpenNutriTracker | | |

```bash
curl -s -X POST http://localhost:3000/statistics/1/log/2026-08-01 \
  -H "Content-Type: application/json" -d '{"value": "480"}'
```

Note the default for IntegerAmount is `sum` — logging twice adds up. Use `overwrite` to correct a value.

### Logging time frames (TimeSpans)

`POST /statistics/{id}/log_time_frame/{date}` with `{"start_time": "09:30", "end_time": "11:45", "update_policy": "updateendvalue"}`. Returns 204.

Pass `"update_policy": "updateendvalue"` to update an interval, or `"update_policy": "overwrite"` to replace every interval for the selected date with the submitted interval. Repeating an identical overwrite leaves the value unchanged.

The default (`append`) hits an unimplemented code path (`todo!()`) when the date already has time frames and crashes the request. Only omit the policy if you've verified the date is empty via `GET /statistics/{id}/logged/{date}`.

## Reading data

```bash
# Has a value been logged? → {statistic_id, date, has_value, value?}
# TimedAmounts value is the list: [{"time":"08:12:00","amount":120},{"time":null,"amount":40}]
# (time null = time unknown); [] = confirmed zero; has_value false = never synced
curl -s http://localhost:3000/statistics/1/logged/2026-08-01

# Totals over a range (note param names differ per endpoint!)
curl -s "http://localhost:3000/statistics/23/time_spans/total?start_date=2026-07-01&end_date=2026-07-31"
curl -s "http://localhost:3000/statistics/1/integer_amount/total?start_date=2026-07-01&end_date=2026-07-31"  # also TimedAmounts: sum of daily totals

# Goal achievement score (mean % of goal, params are start/end here!)
# Optional daily_cap_percent=100: caps each day at 100% before averaging, so
# over-achieving can't offset missed days (partial days still count; response
# also includes days_achieved). This is the habit-dashboard default.
curl -s "http://localhost:3000/statistics/1/score?start=2026-07-01&end=2026-07-31&daily_cap_percent=100"

# Per-day achievement strip (params start/end). Each day: {date, has_goal, achieved,
# percentage_achieved (null without goal), override: none|suppressed|goal,
# value (logged value or null: number for counts, seconds for TimeSpans, true/false for habits,
# "HH:MM:SS" for PointInTime, daily total for TimedAmounts), goal (the goal that applied, like goals/default goal_data, or null)}
# A TimedAmounts day that was never synced has value null and misses its goal; [] counts as 0.
curl -s "http://localhost:3000/statistics/1/achievements?start=2026-07-25&end=2026-07-31"

# Performance + trend (params start_date/end_date; trend needs ≥14 data points)
curl -s "http://localhost:3000/statistics/1/performance?start_date=2026-07-01&end_date=2026-07-31"
# → {overall_score, trend: {direction: improving|declining|stable|insufficient_data, strength?}}

# Consistency (% of last N days goal achieved; days defaults to 14, max 366)
curl -s "http://localhost:3000/statistics/1/consistency?days=30"

# Current goal streak in days (no params)
curl -s http://localhost:3000/statistics/1/streak

# Daily summary: per-category goal % + timeline of the day's activities
curl -s http://localhost:3000/daily_summary/2026-08-01
```

**Dashboard** (multi-statistic overview — today's value, 7d avg, performance):

```bash
curl -s -X POST http://localhost:3000/statistics/dashboard \
  -H "Content-Type: application/json" \
  -d '{"statistic_ids": [1, 23], "today": "2026-08-01", "performance_window_days": 14}'
```

`today_value` units: IntegerAmount = raw value; Habit = 0/1; TimeSpans = total seconds; PointInTime = seconds since midnight; TimedAmounts = daily total (mg). Derived statistics are rejected with 400.

## Goals

```bash
curl -s http://localhost:3000/statistics/1/goals/default     # GET → {"goal_data": {...}, "valid_since": "YYYY-MM-DD"} | null; PUT / DELETE
curl -s http://localhost:3000/statistics/1/goals/2026-08-01  # GET → {"kind": "not_set"} | {"kind": "suppressed"} | {"kind": "goal", "goal_data": {...}}
# Set default (PUT, 204). goal_data values are all strings; fields per type:
#   IntegerAmount/Summing/TimeDifference/TimedAmounts: target (+ optional is_at_least, default "true");
#   for TimedAmounts the target is the daily total, e.g. {"target": "400", "is_at_least": "false"} = at most 400 mg
#   Habit: occurred | PointInTime: deadline "HH:MM" | TimeSpans: target (TimeFrameList JSON string), efficiency
curl -s -X PUT http://localhost:3000/statistics/1/goals/default \
  -H "Content-Type: application/json" -d '{"goal_data": {"target": "480", "is_at_least": "true"}}'
# Optional "valid_since" (default today) backdates or schedules the change; days before it keep their
# old default goal, daily overrides stay. DELETE ends the default goal from today (past days keep it).
curl -s -X PUT http://localhost:3000/statistics/20/goals/default \
  -H "Content-Type: application/json" -d '{"goal_data": {"deadline": "22:05"}, "valid_since": "2026-10-12"}'
# Daily override (PUT, 204) — overrides the default for that date only; DELETE removes the override
curl -s -X PUT http://localhost:3000/statistics/1/goals/2026-08-01 \
  -H "Content-Type: application/json" -d '{"kind": "goal", "goal_data": {"target": "200"}}'
# Suppress = "no goal applies that day" (e.g. travelling). Shows as override: suppressed in /achievements
curl -s -X PUT http://localhost:3000/statistics/1/goals/2026-08-01 \
  -H "Content-Type: application/json" -d '{"kind": "suppressed"}'
# Same override for every day of a range (PUT, 204, all or nothing, at most 366 days)
curl -s -X PUT http://localhost:3000/statistics/1/goals/range \
  -H "Content-Type: application/json" -d '{"start": "2026-08-01", "end": "2026-08-07", "kind": "suppressed"}'
```

## Progress items (books etc.)

Checkpoints record **absolute position** (e.g. "at page 73"), not deltas.

```bash
curl -s http://localhost:3000/progress-items                       # list
curl -s http://localhost:3000/progress-items/1/summary
# → {item, latest_checkpoint, latest_position, furthest_position, percent_latest, percent_furthest, is_complete}
curl -s "http://localhost:3000/progress-items/1/daily-deltas?start=2026-07-01&end=2026-07-31"

# Create: POST /progress-items {name, category, target_value, unit: {type: "Pages"|"Characters"}, tags: {...}}
# Log checkpoint (recorded_at optional, defaults to server time):
curl -s -X POST http://localhost:3000/progress-items/1/checkpoints \
  -H "Content-Type: application/json" \
  -d '{"effective_date": "2026-08-01", "position": 73, "note": "chapter 5"}'
# Update target: PATCH /progress-items/{id}/target
```

Ebook reading syncs automatically via `POST /integrations/ebook/events` (idempotent by `event_id`) — don't log ebook progress manually unless asked.

Beurer scale readings sync automatically via `PUT /integrations/beurer/days/{day}`; do not log them manually. The strict `YYYY-MM-DD` day accepts required integer `weight_grams` (1–500000) and optional integer `body_fat_permille`, `muscle_permille`, `body_water_permille` (0–1000). Missing/null optional metrics stay untouched. The PUT atomically provisions four tagged `Beurer` statistics and overwrites daily levels, returning `204`. Plot data uses `kilograms` and `percent_one_decimal` human units, and PNG labels show one decimal (`67.5 kg`, `15.6 %`); stored readings remain grams/permille.

Yomikata study syncs automatically through `PUT /integrations/yomikata/days/{day}`. Send a strict valid `YYYY-MM-DD` date and `{"cards_reviewed":42,"study_time_seconds":932}`. Both fields are required nonnegative JSON integers up to `9223372036854775807`. Success is `204` after atomic provisioning and replacement. No provisioning request is needed. The existing `[Yomikata] Study Time (minutes)` statistic projects exact seconds to whole minutes. Repeated, lower, and zero totals preserve subsequent manual contributions. Generic integer `overwrite` still corrects the entire aggregate and can leave a signed residual. Do not log Yomikata study manually unless asked.

The daily PUT adopts all tagged legacy values once without changing their existing IDs or goals. Legacy minute values become seconds by multiplying by 60. Duplicate or wrongly typed tags return `409`. The daily ledger survives statistic deletion and the next PUT rebuilds projections. Existing generic logging and `POST /integrations/yomikata` remain available for legacy clients. Old Yomikata generic-overwrite writers must stop at cutover.

Daily PUT errors use `{"error":{"code":"invalid_field","field":"cards_reviewed","message":"..."}}`. Statuses are `400` for malformed JSON, `415` for unsupported media, `422` for invalid fields, dates, or aggregate overflow, `409` for bundle conflicts, and `500` for sanitized storage failures. `GET /openapi.json` is the generated contract for the integration day routes (Beurer, OpenNutriTracker, Yomikata, ebook).

Caffeine and nicotine sync automatically from OpenNutriTracker via `PUT /integrations/opennutritracker/days/{day}` with `{"caffeine":[{"time":"08:12:00","amount_mg":120},{"time":null,"amount_mg":40}],"nicotine":[]}`. Both arrays are required; each entry has exactly `time` (`HH:MM:SS` or `null` when unknown) and integer `amount_mg` (≥ 1); `[]` is a confirmed zero. The PUT replaces both lists on that day (other days untouched), provisions `[OpenNutriTracker] Caffeine (mg)` / `[OpenNutriTracker] Nicotine (mg)` (`TimedAmounts`, category `OpenNutriTracker`) on first use and returns `204`. The app resends about the last 14 days on every sync. Errors use the integration envelope with `field` paths like `caffeine[2].time`. Never write these statistics another way; there is no manual route.

Gym workouts sync automatically via `PUT /integrations/liftlog/days` (the LiftLog app sends the finished workouts of every day it changed; the server replaces `[LiftLog] Workout Time` on exactly those days, and an empty `workouts` list clears a day) — don't log LiftLog workouts manually unless asked. `[LiftLog] Total Weight Lifted (kg)` is no longer written.

## Sessions (live activity timers)

Heartbeat sessions accumulate time frames on a TimeSpans statistic. One active session per statistic.

```bash
curl -s http://localhost:3000/sessions                             # active sessions
curl -s -X POST http://localhost:3000/sessions/25/start            # body optional: {idle_timeout_seconds, flush_interval_seconds}
curl -s -X POST http://localhost:3000/sessions/25/beat             # keep-alive (also auto-starts)
curl -s -X POST http://localhost:3000/sessions/25/stop             # finalize
curl -s http://localhost:3000/sessions/25                          # status
```

## Plots

Fitbit sleep duration and total awake duration display as `HH:MM`. Stored values
remain seconds; `/plot/data` reports `duration_seconds` for values and goals.
TimedAmounts plot their daily total (bar/line/scatter only; `time_frames` returns 400).

```bash
curl -s "http://localhost:3000/statistics/23/plot/image?plot_type=bar_chart" -o plot.png
# optional: date_from, date_to (default last 14 days), width, height (800x600),
# value_labels=true|false (force/hide per-day value annotations; default auto = when they fit)
curl -s "http://localhost:3000/statistics/23/plot/data?plot_type=bar_chart&date_from=2026-07-01&date_to=2026-07-31"
curl -s http://localhost:3000/plots/timeline/2026-08-01 -o timeline.png
```

## Full reference

Complete route/DTO/error documentation: `~/.local/share/src/lifestats/core/infrastructure/web_api/README.md` (only consult for routes not covered here, e.g. Fitbit/Anki integration internals).
