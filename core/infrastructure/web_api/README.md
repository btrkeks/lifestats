# Web API Infrastructure

HTTP REST API server for LifeStats, built with Axum. This crate is an **adapter** in the Ports & Adapters architecture, providing web access to the application's CQRS commands and queries.

For third-party applications integrating with a running `lifestats-web-api.service`,
see the public [Web API Integration Guide](../../../docs/web-api-integration.md).

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [API Routes Reference](#api-routes-reference)
- [Request/Response DTOs](#requestresponse-dtos)
- [Error Handling](#error-handling)
- [State Management](#state-management)
- [Testing](#testing)
- [Adding New Features](#adding-new-features)
- [Environment Configuration](#environment-configuration)
- [Common Patterns](#common-patterns)

## Overview

The web API server provides a RESTful HTTP interface for:

- Managing statistics (create, read, delete)
- Logging values to statistics
- Managing progress items and absolute progress checkpoints
- Managing goals (default and daily)
- Retrieving plot images and data
- Integration provisioning (Anki, Fitbit, etc.)
- Fitbit OAuth flow and data synchronization

**Key characteristics:**

- **Adapter layer**: Translates HTTP requests/responses into CQRS commands/queries
- **Stateless handlers**: Command/query handlers instantiated on-demand per request
- **Dependency injection**: Shared dependencies via `AppState`, feature-specific via Axum layers
- **Type safety**: Strong typing enforced at adapter boundary before reaching application layer
- **Comprehensive error mapping**: Domain/application errors mapped to appropriate HTTP status codes

## Architecture

### Hexagonal Architecture Position

```
┌─────────────────────────────────────────────────┐
│           web_api (This Crate)                  │
│              Infrastructure Layer                │
│                                                  │
│  ┌──────────┐      ┌────────────────────────┐  │
│  │  Axum    │ ───> │  CQRS Handlers         │  │
│  │  Routes  │      │  (instantiated per     │  │
│  │  (DTOs)  │      │   request)             │  │
│  └──────────┘      └────────────────────────┘  │
│                              │                   │
│                              v                   │
│                    ┌──────────────────┐         │
│                    │   Application    │         │
│                    │     Layer        │         │
│                    │  (use cases)     │         │
│                    └──────────────────┘         │
│                              │                   │
│                              v                   │
│                    ┌──────────────────┐         │
│                    │  Domain Layer    │         │
│                    │  (pure logic)    │         │
│                    └──────────────────┘         │
└─────────────────────────────────────────────────┘
```

### Design Patterns

1. **CQRS (Command Query Responsibility Segregation)**
   - Handlers instantiated per-request with required dependencies
   - Commands modify state (POST, PUT, DELETE)
   - Queries read state (GET)

2. **Adapter Pattern**
   - HTTP layer is responsible for parsing and validation
   - Application layer receives strongly-typed data
   - Never pass raw strings into command/query handlers

3. **Dependency Injection**
   - Core dependencies in `AppState` (repositories, publishers, renderer)
   - Feature-specific dependencies via Axum `Extension` layers (Fitbit OAuth, token store)

4. **Error Mapping**
   - Domain errors → 400 Bad Request
   - Repository NotFound → 404 Not Found
   - Infrastructure errors → 500 Internal Server Error
   - Fitbit rate limits → 429 Too Many Requests

## Getting Started

### Running the Server

```bash
# From project root
cargo run -p lifestats-web-api

# Or with environment variables
DATABASE_URL=/path/to/db.sqlite \
RUST_LOG=info,web_api=debug \
cargo run -p lifestats-web-api
```

The server binds to `0.0.0.0:3000` by default (configurable in `src/main.rs:78`).

### Quick Test

```bash
# List all statistics
curl http://localhost:3000/statistics

# Get loggable statistics
curl http://localhost:3000/statistics/loggable

# Log a value (assuming statistic ID 1 exists)
curl -X POST http://localhost:3000/statistics/1/log/2024-01-15 \
  -H "Content-Type: application/json" \
  -d '{"value": "42", "update_policy": "sum"}'
```

## Project Structure

```
web_api/
├── src/
│   ├── main.rs                      # Server entry point, dependency wiring
│   ├── lib.rs                       # Public module exports
│   ├── state.rs                     # AppState definition
│   ├── error.rs                     # Centralized error handling
│   ├── helpers.rs                   # Shared utilities
│   ├── dto/                         # Data Transfer Objects
│   │   ├── mod.rs
│   │   ├── statistic.rs            # Statistic request/response DTOs
│   │   ├── statistic_score.rs      # Score DTOs
│   │   ├── goal.rs                 # Goal DTOs
│   │   ├── goal_performance.rs     # Performance DTOs
│   │   ├── consistency.rs          # Consistency-score DTOs
│   │   ├── streak.rs               # Streak DTOs
│   │   ├── dashboard.rs            # Dashboard DTOs
│   │   ├── progress.rs             # Progress item/checkpoint DTOs
│   │   ├── session.rs              # Heartbeat session DTOs
│   │   ├── plot.rs                 # Plot DTOs
│   │   ├── integer_amount.rs       # IntegerAmount-specific DTOs
│   │   └── time_spans.rs           # TimeSpans-specific DTOs
│   ├── routes/                      # Route handlers
│   │   ├── mod.rs                  # Router configuration
│   │   ├── statistics.rs           # Statistics CRUD + logging
│   │   ├── progress.rs             # Progress items + checkpoints
│   │   ├── sessions.rs             # Heartbeat session management
│   │   ├── dashboard.rs            # Multi-statistic dashboard
│   │   ├── daily_summary.rs        # Daily summary endpoint
│   │   ├── goals.rs                # Goal management
│   │   ├── plots.rs                # Plot retrieval
│   │   └── integrations.rs         # Integration provisioning
│   └── integrations/                # Integration-specific modules
│       ├── mod.rs
│       └── fitbit/                  # Fitbit OAuth & sync
│           ├── mod.rs
│           ├── config.rs           # Fitbit configuration
│           ├── handlers.rs         # OAuth/sync handlers
│           ├── types.rs            # Fitbit-specific types
│           ├── pkce.rs             # PKCE implementation
│           └── state_cache.rs      # OAuth state cache
└── tests/
    ├── statistics_api.rs            # Statistics endpoint tests
    ├── progress_api.rs              # Progress endpoint tests
    ├── integrations_api.rs          # Integration tests
    ├── e2e.rs                       # End-to-end tests
    └── helpers.rs                   # Test utilities
```

## API Routes Reference

**General Notes:**
- All date parameters use the format `YYYY-MM-DD` (e.g., `2025-01-31`)
- Date ranges are **inclusive on both ends** `[start_date, end_date]` unless otherwise specified
- Time parameters use 24-hour format `HH:MM` (e.g., `14:30`)

### Statistics

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/statistics` | List all statistics metadata |
| `GET` | `/statistics/loggable` | List only loggable statistics (excludes derived) |
| `POST` | `/statistics` | Create a new statistic |
| `GET` | `/statistics/{id}` | Get metadata for a specific statistic |
| `DELETE` | `/statistics/{id}` | Delete a statistic |
| `POST` | `/statistics/{id}/log/{date}` | Log a value for a statistic on a date |
| `POST` | `/statistics/{id}/log_time_frame/{date}` | Log a time frame for TimeSpans |
| `GET` | `/statistics/{id}/logged/{date}` | Check if a value has been logged for a specific date |
| `GET` | `/statistics/{id}/time_spans/total` | Get total duration for date range |
| `GET` | `/statistics/{id}/integer_amount/total` | Get total amount for date range |
| `GET` | `/statistics/{id}/consistency` | Get consistency score (% of days achieved) |
| `GET` | `/statistics/{id}/score` | Get mean goal achievement score over date range |
| `GET` | `/statistics/{id}/performance` | Get goal performance analysis with trend over date range |
| `GET` | `/statistics/{id}/streak` | Get current goal-achievement streak in days |
| `POST` | `/statistics/dashboard` | Get dashboard data for multiple statistics |

### Goals

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/statistics/{id}/goals/default` | Get default goal |
| `PUT` | `/statistics/{id}/goals/default` | Set default goal |
| `DELETE` | `/statistics/{id}/goals/default` | Delete default goal |
| `GET` | `/statistics/{id}/goals/{date}` | Get daily goal for specific date |
| `PUT` | `/statistics/{id}/goals/{date}` | Set daily goal for specific date |
| `DELETE` | `/statistics/{id}/goals/{date}` | Delete daily goal |

### Progress Items

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/progress-items` | Create a bounded progress item |
| `GET` | `/progress-items` | List progress items |
| `GET` | `/progress-items/{id}` | Get a progress item |
| `PATCH` | `/progress-items/{id}/target` | Update a progress item's target value |
| `POST` | `/progress-items/{id}/checkpoints` | Log an absolute progress checkpoint |
| `GET` | `/progress-items/{id}/checkpoints` | List checkpoints ordered by recorded time |
| `GET` | `/progress-items/{id}/summary` | Get latest/furthest position and completion percentage |
| `GET` | `/progress-items/{id}/daily-deltas` | Derive daily new progress for an inclusive date range |

### Plots

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/statistics/{id}/plot/image` | Get plot image (PNG) - stateless on-the-fly generation |
| `GET` | `/statistics/{id}/plot/data` | Get plot data (JSON) - stateless on-the-fly generation |
| `GET` | `/plots/timeline/{date}` | Get timeline plot for specific date |

`GET /statistics/{id}/plot/data` accepts `plot_type`, `date_from`, and `date_to`.
The response uses typed drawable values so clients can distinguish counts from time values:

```json
{
  "title": "Deep Work",
  "plot_type": "bar_chart",
  "date_range": { "start": "2026-05-01", "end": "2026-05-02" },
  "points": [
    {
      "date": "2026-05-01",
      "value": {
        "type": "Scalar",
        "value": { "value": 7200, "unit": "duration_seconds" }
      },
      "color": [169, 252, 177],
      "goal": {
        "type": "Scalar",
        "value": { "value": 3600, "unit": "duration_seconds" }
      }
    }
  ],
  "y_axis": {
    "type": "Scalar",
    "min": 0,
    "max": 7560,
    "label_format": "duration_hh_mm"
  }
}
```

Scalar units are `number`, `binary`, `duration_seconds`, `time_of_day_seconds`, and `percent`.
Habit values use `binary`, with `0` for false and `1` for true.
Axis label formats are `number`, `duration_hh_mm`, `time_of_day_hh_mm`, and `percent`.

### Sessions

Heartbeat sessions track ongoing activity for a single statistic. Only one active session per statistic is allowed.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/sessions` | List all currently active sessions |
| `POST` | `/sessions/{statistic_id}/start` | Start a new heartbeat session |
| `POST` | `/sessions/{statistic_id}/beat` | Send a heartbeat (extends idle timeout; creates session on first call) |
| `POST` | `/sessions/{statistic_id}/stop` | Stop the session and finalize pending time frames |
| `GET` | `/sessions/{statistic_id}` | Get current session status |

### Integrations

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/integrations/{source}` | Provision integration (e.g., Anki bundle) |
| `POST` | `/integrations/ebook/events` | Ingest an idempotent ebook reading event and log per-book progress |
| `POST` | `/integrations/liftlog/backup` | Ingest a LiftLog remote-backup snapshot and re-log workout statistics |
| `GET` | `/integrations/fitbit/auth/begin` | Start Fitbit OAuth flow |
| `GET` | `/integrations/fitbit/auth/callback` | OAuth callback handler |
| `POST` | `/integrations/fitbit/sync` | Sync Fitbit data |

### Daily Summary

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/daily_summary/{date}` | Get daily summary for a specific date |

## Request/Response DTOs

### Creating a Progress Item

**Request:** `POST /progress-items`

```json
{
  "name": "The Left Hand of Darkness",
  "category": "Reading",
  "target_value": 304,
  "unit": { "type": "Pages" },
  "tags": { "media_type": "book" }
}
```

**Response:** `201 Created`

```json
{
  "id": 1,
  "name": "The Left Hand of Darkness",
  "category": "Reading",
  "target_value": 304,
  "unit": { "type": "Pages" },
  "tags": { "media_type": "book" },
  "created_at": "2026-05-02T12:00:00Z",
  "updated_at": "2026-05-02T12:00:00Z"
}
```

### Logging a Progress Checkpoint

**Request:** `POST /progress-items/{id}/checkpoints`

```json
{
  "recorded_at": "2026-05-02T20:15:00Z",
  "effective_date": "2026-05-02",
  "position": 73,
  "source": null,
  "source_event_id": null,
  "note": "Finished chapter 5"
}
```

`recorded_at` is optional at the HTTP edge and defaults to server time. Imported checkpoints can
provide both `source` and `source_event_id` to make repeated submissions idempotent.

**Summary:** `GET /progress-items/{id}/summary`

```json
{
  "latest_position": 73,
  "furthest_position": 73,
  "percent_latest": 0.24013157894736842,
  "percent_furthest": 0.24013157894736842,
  "is_complete": false
}
```

### Ingesting an Ebook Event

**Request:** `POST /integrations/ebook/events`

```json
{
  "event_id": "evt-123",
  "date": "2026-05-02",
  "metric": "characters_read",
  "delta": 42,
  "book_id": 7,
  "book_title": "The Left Hand of Darkness",
  "confirmed_position_after": 420,
  "book_target_characters": 1000
}
```

The endpoint keeps the ebook-wide `characters_read` statistic idempotent by `event_id`. It also
ensures a per-book progress item tagged with `source=ebook`, `progress_type=book`, and
`reader_data_id=<book_id>`, then logs an absolute `Characters` checkpoint at
`confirmed_position_after`. Replaying a previously accepted event can backfill the checkpoint
without adding the daily `delta` twice.

**Response:** `200 OK`

```json
{
  "accepted": true,
  "duplicate": false
}
```

### Ingesting a LiftLog Backup

**Request:** `POST /integrations/liftlog/backup`

The body is the raw payload LiftLog's remote-backup feature sends: a gzipped, serialized SQLite
snapshot of the app database (`Content-Type: application/octet-stream`, up to 32 MB). Point the
LiftLog app's *Settings → Backup and restore → Remote backup* endpoint at this route; the app's
optional `X-Api-Key` header is ignored.

On each upload the server parses the `session` table, auto-provisions the `liftlog` bundle on
first use (`[LiftLog] Workout Time` as `TimeSpans` and `[LiftLog] Total Weight Lifted (kg)` as
`IntegerAmount`, both in category `LiftLog`), and rewrites the values for every day present in
the snapshot: merged first-set→last-set time frames, and bodyweight-aware total weight in kg.
Because every backup is a full snapshot, uploads are idempotent and edits to past workouts are
propagated; deleted sessions only disappear from days that still have other workouts (a day is
never cleared retroactively). Legacy protobuf exports are rejected with `400`.

**Response:** `200 OK`

```json
{
  "workouts_imported": 42,
  "days_logged": 17
}
```

### Creating a Statistic

**Request:** `POST /statistics`

```json
{
  "name": "Push Ups",
  "category": "Fitness",
  "statistic_type": "IntegerAmount"
}
```

**Response:** `201 Created`

```json
{
  "id": 42,
  "name": "Push Ups",
  "category": "Fitness",
  "statistic_type": "IntegerAmount"
}
```

### Logging a Value

**Request:** `POST /statistics/{id}/log/{date}`

```json
{
  "value": "50",
  "update_policy": "sum"  // Optional: "sum", "overwrite", "ignore"
}
```

**Response:** `204 No Content`

### Logging a Time Frame

**Request:** `POST /statistics/{id}/log_time_frame/{date}`

```json
{
  "start_time": "09:30",
  "end_time": "11:45",
  "update_policy": "updateendvalue"  // Optional: "append", "updateendvalue"
}
```

**Response:** `204 No Content`

### Checking if Value is Logged

**Request:** `GET /statistics/{id}/logged/{date}`

**Response:** `200 OK`

```json
{
  "statistic_id": 1,
  "date": "2025-11-18",
  "has_value": true,
  "value": 42
}
```

**When no value exists:**

```json
{
  "statistic_id": 1,
  "date": "2025-11-18",
  "has_value": false
}
```

**Notes:**
- Returns 404 if statistic doesn't exist
- Returns 400 for derived statistics (Summing, TimeDifference) - they are computed, not logged
- The `value` field is omitted when `has_value` is false
- Date must be in `YYYY-MM-DD` format

### Setting a Goal

**Request:** `PUT /statistics/{id}/goals/default` (same for daily goals)

```json
{
  "target": "100",
  "is_higher_better": true
}
```

**Response:** `204 No Content`

### Getting Statistic Score

**Request:** `GET /statistics/{id}/score?start=2025-01-01&end=2025-01-31`

Optional: `daily_cap_percent` (1-100) caps each day's achievement percentage before
averaging — with `daily_cap_percent=100`, over-achieving one day cannot compensate
for missing another, while partial days still contribute proportionally (compare
`/consistency`, which is binary per day).

**Response:** `200 OK`

```json
{
  "statistic_id": 1,
  "start_date": "2025-01-01",
  "end_date": "2025-01-31",
  "mean_percentage_achieved": 85,
  "days_with_goal": 25,
  "days_achieved": 20
}
```

**Notes:**
- `mean_percentage_achieved` is the arithmetic mean of daily goal achievement percentages
- Value ranges from 0-255 (or null when no goals exist in the range)
- Only days with goals contribute to the score
- Supports over-achievement (percentages > 100)

### Getting Goal Performance Analysis

**Request:** `GET /statistics/{id}/performance?start_date=2025-01-01&end_date=2025-01-31`

**Response:** `200 OK`

```json
{
  "overall_score": 87,
  "trend": {
    "direction": "improving",
    "strength": 0.64
  }
}
```

**Trend Variants:**

```json
// Improving performance
{
  "overall_score": 87,
  "trend": {
    "direction": "improving",
    "strength": 0.64  // 0.0 to 1.0, higher = stronger trend
  }
}

// Declining performance
{
  "overall_score": 65,
  "trend": {
    "direction": "declining",
    "strength": 0.42
  }
}

// Stable performance
{
  "overall_score": 80,
  "trend": {
    "direction": "stable"
  }
}

// Insufficient data for trend analysis
{
  "overall_score": 75,
  "trend": {
    "direction": "insufficient_data"
  }
}
```

**Notes:**
- **Date range is inclusive on both ends:** `[start_date, end_date]`
  - Example: `start_date=2025-01-01&end_date=2025-01-31` includes all 31 days
- `overall_score`: Average goal achievement percentage across the date range (0-100)
- `trend`: Calculated using Exponentially Weighted Moving Averages (EWMA)
  - Fast EWMA (3-day span) vs Slow EWMA (7-day span)
  - Requires minimum **14 data points** for trend calculation
  - If fewer than 14 points, returns `direction: "insufficient_data"`
  - `strength` field only present for `improving` and `declining` trends
  - Stability threshold: 1.0 percentage point (differences within this are `stable`)
- **Days without goals or with suppressed goals are treated as 100% achievement** for performance analysis
- Returns `400 Bad Request` if no data exists in the range
- Returns `404 Not Found` if statistic doesn't exist
- Returns `400 Bad Request` for invalid date formats or `start_date > end_date`

### Getting Dashboard Data

**Request:** `POST /statistics/dashboard`

```json
{
  "statistic_ids": [80, 24, 23],
  "today": "2025-12-22",
  "performance_window_days": 14
}
```

**Response:** `200 OK`

```json
{
  "statistics": [
    {
      "statistic_id": 80,
      "statistic_name": "Steps",
      "statistic_type": "IntegerAmount",
      "today_value": 573,
      "avg_7d_value": 412.5,
      "performance": {
        "overall_score": 61,
        "trend": {
          "direction": "improving",
          "strength": 0.24
        }
      }
    },
    {
      "statistic_id": 24,
      "statistic_name": "Reading",
      "statistic_type": "TimeSpans",
      "today_value": 3600,
      "avg_7d_value": 2700.0,
      "performance": {
        "overall_score": 85,
        "trend": {
          "direction": "stable"
        }
      }
    }
  ]
}
```

**Notes:**
- `statistic_ids`: Array of statistic IDs to include (must not be empty)
- `today`: Reference date for "today's value" calculation
- `performance_window_days`: Number of days for performance analysis (1-365)
- `today_value`: Today's logged value as i64 (null if no value logged)
  - IntegerAmount: direct value
  - Habit: 1 (true) or 0 (false)
  - TimeSpans: total duration in seconds
  - PointInTime: seconds from midnight
- `avg_7d_value`: 7-day rolling average (null if insufficient data)
- `performance`: Goal performance analysis (null if no goals or insufficient data)
- Returns `404 Not Found` if any statistic ID doesn't exist
- Returns `400 Bad Request` if derived statistics (Summing, TimeDifference) are requested

### Starting a Session

**Request:** `POST /sessions/{statistic_id}/start`

```json
{
  "idle_timeout_seconds": 60,
  "flush_interval_seconds": 60
}
```

Both fields are optional; defaults from `HeartbeatConfig` are used when omitted. Values are clamped between 1 and 86400.

**Response:** `201 Created`

```json
{
  "statistic_id": 42,
  "status": "running",
  "is_running": true,
  "total_elapsed_seconds": 0
}
```

The same `SessionStatusResponse` shape is returned by `GET /sessions/{statistic_id}` and (as an array) by `GET /sessions`. `POST /sessions/{statistic_id}/beat` and `/stop` return `204 No Content`.

### Getting Daily Summary

**Request:** `GET /daily_summary/{date}`

**Example:** `GET /daily_summary/2025-01-15`

**Response:** `200 OK`

```json
{
  "date": "2025-01-15",
  "category_percentages": {
    "Fitness": 75.5,
    "Study": 42.0,
    "Sleep": 100.0
  },
  "timeline_events": [
    {
      "timeframe": {
        "start": "09:30",
        "end": "11:45"
      },
      "label": "Deep Work"
    },
    {
      "timeframe": {
        "start": "14:00",
        "end": "15:30"
      },
      "label": "Exercise"
    }
  ]
}
```

**Notes:**
- `category_percentages`: Percentage of goal achievement for each category (0-100+)
- `timeline_events`: Time-based activities that occurred during the day
- Returns empty arrays/objects if no data exists for the date

## Error Handling

Errors are centralized in `src/error.rs` and mapped to appropriate HTTP status codes:

| Error Type | HTTP Status | Use Case |
|------------|-------------|----------|
| `ValidationError` | 400 Bad Request | Invalid request format/validation failure |
| `BadRequest` | 400 Bad Request | Domain validation errors, invalid input |
| `NotFound` | 404 Not Found | Resource doesn't exist |
| `Unauthorized` | 401 Unauthorized | Fitbit invalid grant |
| `InternalError` | 500 Internal Server Error | Infrastructure/repository errors |
| `BadGateway` | 502 Bad Gateway | Fitbit upstream failures |
| `RateLimited` | 429 Too Many Requests | Fitbit rate limiting (includes `Retry-After` header) |

### Error Response Format

All errors return JSON:

```json
{
  "error": "Human-readable error message"
}
```

**Note:** Internal server errors (500) do NOT leak implementation details to the client. Full error details are logged via `tracing::error!`.

## State Management

### AppState

The `AppState` struct holds **globally shared dependencies** injected into every handler:

```rust
pub struct AppState<R, Rend, E, I> {
    pub statistic_repository: Arc<R>,
    pub progress_repository: Arc<dyn ProgressRepository + Send + Sync>,
    pub event_publisher: Arc<E>,
    pub renderer: Arc<Rend>,
    pub integration_service: Arc<I>,
    pub session_manager: Arc<SessionManager<R>>,
}
```

Plotting is **stateless** — there is no `plot_repository`; plots are rendered on-the-fly via `renderer`. The `progress_repository` is type-erased (`dyn`) rather than a generic parameter.

Access in handlers:

```rust
pub async fn handler<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<ResponseDto>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    // ... other bounds
{
    // Use app_state.statistic_repository, etc.
}
```

### Extension Layers

**Feature-specific dependencies** are injected via Axum `Extension` layers:

```rust
// In main.rs
let app = routes::create_router(app_state)
    .layer(axum::Extension(fitbit_config))
    .layer(axum::Extension(app.fitbit_oauth_api))
    .layer(axum::Extension(app.fitbit_token_store))
    .layer(axum::Extension(fitbit_state_cache))
    .layer(axum::Extension(app.fitbit_sync_handler));
```

Access in Fitbit handlers:

```rust
use axum::Extension;

pub async fn auth_begin_handler(
    Extension(config): Extension<Arc<FitbitConfig>>,
    Extension(oauth_api): Extension<Arc<dyn FitbitOAuthApi>>,
) -> Result<Response, ApiError> {
    // Use config and oauth_api
}
```

**Why this split?**

- `AppState`: Core dependencies used across most routes
- `Extension`: Optional/feature-specific dependencies that don't bloat every handler signature

## Testing

### Test Structure

1. **Unit tests**: Inline in modules (e.g., `src/error.rs`)
2. **Integration tests**: In `tests/` directory using `axum-test`
3. **End-to-end tests**: Full database + HTTP stack in `tests/e2e.rs`

### Running Tests

```bash
# All tests
cargo test -p lifestats-web-api

# Specific test file
cargo test -p lifestats-web-api --test statistics_api

# With logs
RUST_LOG=debug cargo test -p lifestats-web-api -- --nocapture
```

### Test Utilities

Helpers in `tests/helpers.rs` provide:

- In-memory test database setup
- Mock repositories
- Pre-seeded test statistics
- Axum test server builder

### Example Integration Test

```rust
use axum_test::TestServer;
use lifestats_web_api::routes::create_router;

#[tokio::test]
async fn test_get_statistic() {
    let (repo, publisher) = create_test_dependencies();
    let app_state = AppState::new(repo, /* ... */);
    let router = create_router(app_state);
    let server = TestServer::new(router).unwrap();

    let response = server.get("/statistics/1").await;
    response.assert_status_ok();

    let json: StatisticMetadataResponse = response.json();
    assert_eq!(json.id, 1);
}
```

## Adding New Features

### Adding a New Route

1. **Create the handler** in the appropriate `routes/*.rs` file:

```rust
// routes/statistics.rs
pub async fn my_new_handler<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Json(payload): Json<MyRequestDto>,
) -> Result<Json<MyResponseDto>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    // ... other bounds
{
    // 1. Validate input
    payload.validate()?;

    // 2. Instantiate command/query handler
    let handler = MyCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );

    // 3. Execute
    let result = handler.handle(MyCommand { /* ... */ })?;

    // 4. Return response
    Ok(Json(MyResponseDto::from(result)))
}
```

2. **Register the route** in `routes/mod.rs`:

```rust
pub fn create_router<R, Rend, E, I>(...) -> Router {
    Router::new()
        // ... existing routes
        .route("/my-new-endpoint", get(my_new_handler))
        .with_state(app_state)
}
```

3. **Add DTOs** in `dto/` if needed:

```rust
// dto/my_feature.rs
#[derive(Debug, Deserialize, Validate)]
pub struct MyRequestDto {
    #[validate(length(min = 1))]
    pub field: String,
}

#[derive(Debug, Serialize)]
pub struct MyResponseDto {
    pub result: String,
}
```

4. **Write tests** in `tests/`:

```rust
#[tokio::test]
async fn test_my_new_endpoint() {
    let server = create_test_server();
    let response = server.get("/my-new-endpoint").await;
    response.assert_status_ok();
}
```

### Adding a New Integration

For complex integrations like Fitbit:

1. Create module under `src/integrations/my_integration/`
2. Add configuration struct
3. Implement OAuth flow if needed (see `integrations/fitbit/handlers.rs`)
4. Add handlers for auth and data sync
5. Register routes in `routes/mod.rs`
6. Inject dependencies via Extension layers in `main.rs`

## Environment Configuration

Configuration via environment variables (loaded from `.env` if present):

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | No* | In-memory | Path to SQLite database |
| `RUST_LOG` | No | `info,web_api=debug,tower_http=debug` | Log level |
| `FITBIT_CLIENT_ID` | No | `dummy_client_id` | Fitbit OAuth client ID |
| `FITBIT_CLIENT_SECRET` | No | `dummy_secret` | Fitbit OAuth secret |
| `FITBIT_REDIRECT_URI` | No | `http://localhost:3000/integrations/fitbit/auth/callback` | OAuth redirect |
| `FITBIT_SCOPES` | No | `activity sleep` | Space-separated scopes |
| `FITBIT_ENABLED_METRICS` | No | `steps,sleep_duration` | Comma-separated metrics |
| `TIMEZONE` | No | `UTC` | Timezone for Fitbit sync |

**Note:** Tests use in-memory databases; `DATABASE_URL` only required for live usage.

### Example `.env` File

```bash
DATABASE_URL=/home/user/.local/share/lifestats/db.sqlite
RUST_LOG=debug
FITBIT_CLIENT_ID=your_client_id
FITBIT_CLIENT_SECRET=your_client_secret
```

## Common Patterns

### 1. Handler Structure

Every handler follows this pattern:

```rust
pub async fn handler<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,              // Optional: path params
    Query(params): Query<QueryDto>,   // Optional: query params
    Json(payload): Json<RequestDto>,  // Optional: request body
) -> Result<Json<ResponseDto>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // 1. Validate input (if using validator crate)
    payload.validate()?;

    // 2. Parse and transform data (adapter responsibility)
    let domain_value = parse_into_domain_type(&payload)?;

    // 3. Instantiate handler with required dependencies
    let handler = CommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );

    // 4. Execute command/query
    let result = handler.handle(Command {
        field: domain_value,
        // ...
    })?;

    // 5. Transform result to DTO and return
    Ok(Json(ResponseDto::from(result)))
}
```

### 2. Type Conversion at Adapter Boundary

The adapter is responsible for parsing strings into domain types:

```rust
// BAD: Passing string to application layer
let command = LogValueCommand {
    value: payload.value,  // ❌ Don't pass raw strings
};

// GOOD: Parse in adapter
let value: i64 = payload.value.parse()
    .map_err(|_| ApiError::BadRequest("Expected integer".to_string()))?;

let command = LogValueCommand {
    value,  // ✅ Strongly typed
};
```

### 3. Error Conversion

Use the `?` operator with `From<ApplicationError> for ApiError`:

```rust
let result = handler.handle(command)?;  // Automatically converts errors
```

Or manually map for custom messages:

```rust
let result = handler.handle(command)
    .map_err(|e| ApiError::InternalError(format!("Failed: {}", e)))?;
```

### 4. Instantiating Handlers Per-Request

Handlers are **not** stored in `AppState`:

```rust
// ❌ DON'T store handlers in AppState
pub struct AppState {
    pub log_value_handler: LogValueCommandHandler,  // Don't do this
}

// ✅ DO instantiate per-request
pub async fn handler(State(app_state): State<AppState>) {
    let handler = LogValueCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler.handle(command)?;
}
```

**Why?** Keeps dependencies explicit and allows different handlers to require different dependencies.

### 5. HTTP Status Codes

Follow REST conventions:

- `200 OK`: Successful GET with response body
- `201 Created`: Resource created (return resource in body)
- `204 No Content`: Successful POST/PUT/DELETE with no response body
- `400 Bad Request`: Validation errors, invalid input
- `404 Not Found`: Resource doesn't exist
- `500 Internal Server Error`: Unexpected server errors

## Related Documentation

- **Project root**: See `../../CLAUDE.md` for overall architecture
- **Application layer**: See `../../application/` for CQRS patterns
- **Domain layer**: See `../../core/` for business logic
- **Persistence**: See `../persistence/` for repository implementations

## Troubleshooting

### Port 3000 Already in Use

Change the bind address in `src/main.rs:78`:

```rust
let addr = "0.0.0.0:8080";  // Use different port
```

### Database Migration Errors

Migrations run automatically on startup. If they fail:

```bash
cd ../persistence
diesel migration run
```

### CORS Errors

The server uses `CorsLayer::permissive()` for development. For production, configure CORS in `main.rs:75`.

### Request Validation Failures

Ensure DTOs implement `Validate` and call `payload.validate()?` in handlers.

---

**For questions or issues**, refer to the project root documentation or open an issue in the repository.
