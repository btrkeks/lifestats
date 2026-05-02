# LifeStats Web API Integration Guide

This guide is for third-party application developers integrating with a running
`lifestats-web-api.service`. It documents the HTTP API surface you should use
from external apps, scripts, browser extensions, and local automation.

For maintainers working inside the Rust web API crate, see
[`core/infrastructure/web_api/README.md`](../core/infrastructure/web_api/README.md).

## Connection Basics

The default API base URL is:

```text
http://localhost:3000
```

The bundled `lifestats-web-api.service` runs the `lifestats-web-api` binary. The
current server binds to `0.0.0.0:3000`, uses permissive CORS, and requires
`DATABASE_URL` to point at the SQLite database used by the service.

There is currently no built-in API authentication layer. Treat the service as a
trusted local or private-network API. If you expose it beyond localhost, put it
behind your own reverse proxy, TLS, and authentication.

There is no documented health endpoint. To verify that the service is reachable,
use a read-only endpoint:

```bash
curl http://localhost:3000/statistics
```

General conventions:

- Request and response bodies are JSON unless the endpoint returns an image.
- Dates use `YYYY-MM-DD`, for example `2026-05-02`.
- Times use `HH:MM` or `HH:MM:SS` in 24-hour local time.
- Date ranges are inclusive at both ends.
- IDs in paths are numeric statistic IDs.
- Errors return JSON in the form `{"error": "message"}`.

## Quickstart

### 1. Find an Existing Loggable Statistic

Derived statistics cannot be logged directly, so start by listing loggable
statistics:

```bash
curl http://localhost:3000/statistics/loggable
```

Example response:

```json
[
  {
    "id": 1,
    "name": "Push Ups",
    "category": "Fitness",
    "statistic_type": "IntegerAmount"
  }
]
```

### 2. Create a Statistic if Needed

```bash
curl -X POST http://localhost:3000/statistics \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Push Ups",
    "category": "Fitness",
    "statistic_type": "IntegerAmount"
  }'
```

Supported `statistic_type` values are `IntegerAmount`, `Habit`, `PointInTime`,
`TimeSpans`, `Summing`, and `TimeDifference`. `Summing` and `TimeDifference`
are derived statistics and require `source_statistic_ids`.

### 3. Log a Value

For an `IntegerAmount` statistic:

```bash
curl -X POST http://localhost:3000/statistics/1/log/2026-05-02 \
  -H "Content-Type: application/json" \
  -d '{
    "value": "25",
    "update_policy": "sum"
  }'
```

Successful writes return `204 No Content`.

### 4. Check the Logged Value

```bash
curl http://localhost:3000/statistics/1/logged/2026-05-02
```

Example response:

```json
{
  "statistic_id": 1,
  "date": "2026-05-02",
  "has_value": true,
  "value": 25
}
```

## Statistic Types

| Type | Directly loggable? | Log value format | Typical use |
| --- | --- | --- | --- |
| `IntegerAmount` | Yes | Integer string, for example `"25"` | Counts, pages, steps |
| `Habit` | Yes | Boolean string, `"true"` or `"false"` | Daily yes/no tracking |
| `PointInTime` | Yes | Time string, for example `"23:15"` | Wake time, bedtime, completion time |
| `TimeSpans` | Yes | Time frames, usually via `log_time_frame` | Sessions and activity intervals |
| `Summing` | No | Computed from source `TimeSpans` statistics | Aggregate duration |
| `TimeDifference` | No | Computed from source `PointInTime` statistics | Difference between two times |

Use `GET /statistics/loggable` when your app needs a user-selectable list of
statistics that can accept logs.

## Logging Values

### Generic Log Endpoint

```http
POST /statistics/{id}/log/{date}
```

Request body:

```json
{
  "value": "25",
  "update_policy": "sum"
}
```

`value` is always sent as a string. The server parses it based on the statistic
type.

| Statistic type | `value` | Update policies | Default |
| --- | --- | --- | --- |
| `IntegerAmount` | Integer, for example `"25"` | `sum`, `overwrite`, `ignore` | `sum` |
| `Habit` | Boolean, `"true"` or `"false"` | `ignore`, `overwrite` | `overwrite` |
| `PointInTime` | `HH:MM` or `HH:MM:SS` | `ignore`, `overwrite` | `overwrite` |
| `TimeSpans` | JSON-encoded time frame list string | `append`, `updateendvalue` | `append` |

Do not use this endpoint for `Summing` or `TimeDifference`; they are derived and
return `400 Bad Request` when logged directly.

### Time Frame Convenience Endpoint

For most third-party apps that track activity intervals, prefer:

```http
POST /statistics/{id}/log_time_frame/{date}
```

Request body:

```json
{
  "start_time": "09:30",
  "end_time": "10:15",
  "update_policy": "updateendvalue"
}
```

This endpoint only accepts `TimeSpans` statistics. The `update_policy` field is
optional. `updateendvalue` is useful for repeatedly extending an in-progress
time frame with the same start time.

## Heartbeat Sessions

Use sessions when your app wants LifeStats to manage an active timer and flush
time spans as heartbeats arrive.

Start a session:

```bash
curl -X POST http://localhost:3000/sessions/24/start \
  -H "Content-Type: application/json" \
  -d '{
    "idle_timeout_seconds": 120,
    "flush_interval_seconds": 60
  }'
```

Send heartbeats:

```bash
curl -X POST http://localhost:3000/sessions/24/beat
```

Stop the session:

```bash
curl -X POST http://localhost:3000/sessions/24/stop
```

Session status responses look like:

```json
{
  "statistic_id": 24,
  "status": "running",
  "is_running": true,
  "total_elapsed_seconds": 180
}
```

Only one active session is allowed per statistic. Starting a duplicate session
returns `409 Conflict`.

## Goals

Goals are type-specific targets used by score, consistency, dashboard, and
summary endpoints.

### Default Goal

```http
PUT /statistics/{id}/goals/default
```

Request body:

```json
{
  "goal_data": {
    "target": "100",
    "is_at_least": "true"
  }
}
```

Get or delete the default goal:

```bash
curl http://localhost:3000/statistics/1/goals/default
curl -X DELETE http://localhost:3000/statistics/1/goals/default
```

### Daily Goal Override

A daily override can set a one-day goal or suppress the default goal for one
date.

Set a concrete override:

```bash
curl -X PUT http://localhost:3000/statistics/1/goals/2026-05-02 \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "goal",
    "goal_data": {
      "target": "100",
      "is_at_least": "true"
    }
  }'
```

Suppress the default goal for a date:

```bash
curl -X PUT http://localhost:3000/statistics/1/goals/2026-05-02 \
  -H "Content-Type: application/json" \
  -d '{ "kind": "suppressed" }'
```

Delete an override so the default applies again:

```bash
curl -X DELETE http://localhost:3000/statistics/1/goals/2026-05-02
```

### `goal_data` Fields

All `goal_data` values are strings.

| Statistic type | Fields | Example |
| --- | --- | --- |
| `IntegerAmount` | `target`, optional `is_at_least` | `{"target":"100","is_at_least":"true"}` |
| `Habit` | `occurred` | `{"occurred":"true"}` |
| `PointInTime` | `deadline` | `{"deadline":"23:00"}` |
| `TimeSpans` | `target`, `efficiency` | `{"target":"[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]","efficiency":"80"}` |
| `Summing` | `target`, optional `is_at_least` | `{"target":"3600","is_at_least":"true"}` |
| `TimeDifference` | `target`, optional `is_at_least` | `{"target":"3600","is_at_least":"true"}` |

For `TimeSpans`, `target` is a JSON-encoded time frame list string and
`efficiency` is a percentage from `0` to `100`.

## Common Integration Workflows

### External Logger

1. Call `GET /statistics/loggable`.
2. Let the user choose or configure a statistic ID.
3. Send `POST /statistics/{id}/log/{date}` or
   `POST /statistics/{id}/log_time_frame/{date}`.
4. Treat `204 No Content` as success.

### Activity Timer

1. Require a `TimeSpans` statistic.
2. Start with `POST /sessions/{statistic_id}/start`.
3. Send `POST /sessions/{statistic_id}/beat` while the activity is active.
4. Stop with `POST /sessions/{statistic_id}/stop`.

### Dashboard or Reader App

Use these read endpoints:

- `GET /statistics`
- `GET /statistics/{id}/logged/{date}`
- `GET /statistics/{id}/score?start=YYYY-MM-DD&end=YYYY-MM-DD`
- `GET /statistics/{id}/performance?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD`
- `POST /statistics/dashboard`
- `GET /daily_summary/{date}`

### Integration Bundle Provisioning

Use bundle provisioning when your integration needs a repeatable set of
statistics. The endpoint is idempotent for the same source and context.

```bash
curl -X POST http://localhost:3000/integrations/anki \
  -H "Content-Type: application/json" \
  -d '{ "deck_name": "Japanese" }'
```

Example response:

```json
{
  "time_spent_id": 12,
  "disruptions_id": 13,
  "backlog_id": 14,
  "finished_time_id": 15
}
```

The response keys are integration-defined logical names with an `_id` suffix.

## Optional TypeScript Client

Raw HTTP is the canonical API contract. For TypeScript, browser extensions,
Node.js scripts, and Obsidian DataviewJS, the optional `@lifestats/client`
package wraps common statistics and goal operations. See
[`clients/lifestats-js/README.md`](../clients/lifestats-js/README.md).

## Endpoint Reference

### Statistics and Logging

| Method | Path | Notes |
| --- | --- | --- |
| `GET` | `/statistics` | List all statistics metadata |
| `GET` | `/statistics/loggable` | List only directly loggable statistics |
| `POST` | `/statistics` | Create a statistic |
| `GET` | `/statistics/{id}` | Get metadata for one statistic |
| `DELETE` | `/statistics/{id}` | Delete a statistic |
| `POST` | `/statistics/{id}/log/{date}` | Log a typed value |
| `POST` | `/statistics/{id}/log_time_frame/{date}` | Log one `TimeSpans` frame |
| `GET` | `/statistics/{id}/logged/{date}` | Check whether a value exists |
| `GET` | `/statistics/{id}/time_spans/total?start_date=...&end_date=...` | Total `TimeSpans` seconds |
| `GET` | `/statistics/{id}/integer_amount/total?start_date=...&end_date=...` | Total integer amount |

### Goals and Scores

| Method | Path | Notes |
| --- | --- | --- |
| `GET` | `/statistics/{id}/goals/default` | Get default goal, or `null` |
| `PUT` | `/statistics/{id}/goals/default` | Set default goal |
| `DELETE` | `/statistics/{id}/goals/default` | Delete default goal |
| `GET` | `/statistics/{id}/goals/{date}` | Get daily goal override, or `null` |
| `PUT` | `/statistics/{id}/goals/{date}` | Set or suppress daily override |
| `DELETE` | `/statistics/{id}/goals/{date}` | Delete daily override |
| `GET` | `/statistics/{id}/consistency?days=14` | Goal consistency over recent days |
| `GET` | `/statistics/{id}/score?start=...&end=...` | Mean goal achievement |
| `GET` | `/statistics/{id}/performance?start_date=...&end_date=...` | Score plus trend |

### Sessions, Dashboards, and Summaries

| Method | Path | Notes |
| --- | --- | --- |
| `GET` | `/sessions` | List active sessions |
| `POST` | `/sessions/{statistic_id}/start` | Start heartbeat session |
| `POST` | `/sessions/{statistic_id}/beat` | Send heartbeat |
| `POST` | `/sessions/{statistic_id}/stop` | Stop heartbeat session |
| `GET` | `/sessions/{statistic_id}` | Get session status |
| `POST` | `/statistics/dashboard` | Multi-statistic dashboard data |
| `GET` | `/daily_summary/{date}` | Daily category and timeline summary |

`POST /statistics/dashboard` body:

```json
{
  "statistic_ids": [80, 24, 23],
  "today": "2026-05-02",
  "performance_window_days": 14
}
```

### Plots

| Method | Path | Notes |
| --- | --- | --- |
| `GET` | `/statistics/{id}/plot/image` | PNG plot image |
| `GET` | `/statistics/{id}/plot/data` | Plot data JSON |
| `GET` | `/plots/timeline/{date}` | PNG timeline image |

### Integrations

| Method | Path | Notes |
| --- | --- | --- |
| `POST` | `/integrations/{source}` | Provision integration bundle |
| `POST` | `/integrations/ebook/events` | Ingest ebook sync event |
| `GET` | `/integrations/fitbit/auth/begin` | Start Fitbit OAuth flow |
| `GET` | `/integrations/fitbit/auth/callback` | Fitbit OAuth callback |
| `POST` | `/integrations/fitbit/sync` | Sync Fitbit data |

Fitbit sync body:

```json
{
  "since": "2026-05-01",
  "until": "2026-05-02"
}
```

## Errors

All errors use this JSON shape:

```json
{
  "error": "Human-readable message"
}
```

Important status codes:

| Status | Meaning |
| --- | --- |
| `400 Bad Request` | Invalid input, invalid date range, domain validation error |
| `401 Unauthorized` | Fitbit authorization/session problem |
| `404 Not Found` | Statistic or active session was not found |
| `409 Conflict` | Duplicate active session |
| `422 Unprocessable Entity` | Request validation failed |
| `429 Too Many Requests` | Fitbit rate limit; may include `Retry-After` |
| `500 Internal Server Error` | Database, infrastructure, or unexpected server failure |
| `502 Bad Gateway` | Fitbit upstream failure |

For write operations, retry only when the operation is safe for your integration.
For idempotent provisioning, calling `POST /integrations/{source}` again with the
same context should return the same statistic IDs.
