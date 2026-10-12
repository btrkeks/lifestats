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

- **Statistic types** — logable: `IntegerAmount`, `Habit`, `PointInTime`, `TimeSpans`; derived: `Summing`, `TimeDifference`. Logging to a derived statistic returns 400; they're computed from others. `GET /statistics/loggable` lists only logable ones.
- **Goals**: a statistic has one optional default goal; a daily goal for a specific date overrides the default for that date only.
- **Progress items** are separate from statistics: bounded, absolute-position things (books read in pages/characters) logged via checkpoints, not daily values.

## Finding statistics

```bash
curl -s http://localhost:3000/statistics           # all: [{id, name, category, statistic_type, default_goal}] (default_goal is null when unset)
curl -s http://localhost:3000/statistics/loggable  # excludes derived
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
curl -s http://localhost:3000/statistics/1/logged/2026-08-01

# Totals over a range (note param names differ per endpoint!)
curl -s "http://localhost:3000/statistics/23/time_spans/total?start_date=2026-07-01&end_date=2026-07-31"
curl -s "http://localhost:3000/statistics/1/integer_amount/total?start_date=2026-07-01&end_date=2026-07-31"

# Goal achievement score (mean % of goal, params are start/end here!)
# Optional daily_cap_percent=100: caps each day at 100% before averaging, so
# over-achieving can't offset missed days (partial days still count; response
# also includes days_achieved). This is the habit-dashboard default.
curl -s "http://localhost:3000/statistics/1/score?start=2026-07-01&end=2026-07-31&daily_cap_percent=100"

# Per-day achievement strip (params start/end). Each day: {date, has_goal, achieved,
# percentage_achieved (null without goal), override: none|suppressed|goal}
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

`today_value` units: IntegerAmount = raw value; Habit = 0/1; TimeSpans = total seconds; PointInTime = seconds since midnight. Derived statistics are rejected with 400.

## Goals

```bash
curl -s http://localhost:3000/statistics/1/goals/default     # GET → {"goal_data": {...}} | null; PUT / DELETE
curl -s http://localhost:3000/statistics/1/goals/2026-08-01  # GET → {"kind": "not_set"} | {"kind": "suppressed"} | {"kind": "goal", "goal_data": {...}}
# Set default (PUT, 204). goal_data values are all strings; fields per type:
#   IntegerAmount/Summing/TimeDifference: target (+ optional is_at_least, default "true")
#   Habit: occurred | PointInTime: deadline "HH:MM" | TimeSpans: target (TimeFrameList JSON string), efficiency
curl -s -X PUT http://localhost:3000/statistics/1/goals/default \
  -H "Content-Type: application/json" -d '{"goal_data": {"target": "480", "is_at_least": "true"}}'
# Daily override (PUT, 204) — overrides the default for that date only; DELETE removes the override
curl -s -X PUT http://localhost:3000/statistics/1/goals/2026-08-01 \
  -H "Content-Type: application/json" -d '{"kind": "goal", "goal_data": {"target": "200"}}'
# Suppress = "no goal applies that day" (e.g. travelling). Shows as override: suppressed in /achievements
curl -s -X PUT http://localhost:3000/statistics/1/goals/2026-08-01 \
  -H "Content-Type: application/json" -d '{"kind": "suppressed"}'
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

Yomikata (kanji-reading app) study syncs automatically: the app provisions `[Yomikata] Cards Reviewed` and `[Yomikata] Study Time (minutes)` via `POST /integrations/yomikata` with `{}`, then overwrites each day's totals. Don't log Yomikata study manually unless asked.

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

```bash
curl -s "http://localhost:3000/statistics/23/plot/image?plot_type=bar_chart" -o plot.png
# optional: date_from, date_to (default last 14 days), width, height (800x600),
# value_labels=true|false (force/hide per-day value annotations; default auto = when they fit)
curl -s "http://localhost:3000/statistics/23/plot/data?plot_type=bar_chart&date_from=2026-07-01&date_to=2026-07-31"
curl -s http://localhost:3000/plots/timeline/2026-08-01 -o timeline.png
```

## Full reference

Complete route/DTO/error documentation: `~/.local/share/src/lifestats/core/infrastructure/web_api/README.md` (only consult for routes not covered here, e.g. Fitbit/Anki integration internals).
