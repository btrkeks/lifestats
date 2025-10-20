# Web API Infrastructure

HTTP REST API server for LifeStats, built with Axum. This crate is an **adapter** in the Ports & Adapters architecture, providing web access to the application's CQRS commands and queries.

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
  -d '{"value": "42", "update_policy": "Sum"}'
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
│   │   ├── goal.rs                 # Goal DTOs
│   │   ├── plot.rs                 # Plot DTOs
│   │   └── time_spans.rs           # TimeSpans-specific DTOs
│   ├── routes/                      # Route handlers
│   │   ├── mod.rs                  # Router configuration
│   │   ├── statistics.rs           # Statistics CRUD + logging
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
    ├── integrations_api.rs          # Integration tests
    ├── e2e.rs                       # End-to-end tests
    └── helpers.rs                   # Test utilities
```

## API Routes Reference

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
| `GET` | `/statistics/{id}/time_spans/total` | Get total duration for date range |

### Goals

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/statistics/{id}/goals/default` | Get default goal |
| `PUT` | `/statistics/{id}/goals/default` | Set default goal |
| `DELETE` | `/statistics/{id}/goals/default` | Delete default goal |
| `GET` | `/statistics/{id}/goals/{date}` | Get daily goal for specific date |
| `PUT` | `/statistics/{id}/goals/{date}` | Set daily goal for specific date |
| `DELETE` | `/statistics/{id}/goals/{date}` | Delete daily goal |

### Plots

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/plots/{id}/image` | Get plot image (PNG) |
| `GET` | `/plots/{id}/data` | Get plot data (JSON) |

### Integrations

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/integrations/{source}` | Provision integration (e.g., Anki bundle) |
| `GET` | `/integrations/fitbit/auth/begin` | Start Fitbit OAuth flow |
| `GET` | `/integrations/fitbit/auth/callback` | OAuth callback handler |
| `POST` | `/integrations/fitbit/sync` | Sync Fitbit data |

## Request/Response DTOs

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
  "update_policy": "Sum"  // Optional: "Sum", "Overwrite", "Max", "Min"
}
```

**Response:** `204 No Content`

### Logging a Time Frame

**Request:** `POST /statistics/{id}/log_time_frame/{date}`

```json
{
  "start_time": "09:30",
  "end_time": "11:45",
  "update_policy": "Append"  // Optional: "Append", "Overwrite"
}
```

**Response:** `204 No Content`

### Setting a Goal

**Request:** `PUT /statistics/{id}/goals/default` (same for daily goals)

```json
{
  "target": "100",
  "is_higher_better": true
}
```

**Response:** `204 No Content`

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
pub struct AppState<R, P, Rend, E, I> {
    pub statistic_repository: Arc<R>,
    pub plot_repository: Arc<P>,
    pub event_publisher: Arc<E>,
    pub renderer: Arc<Rend>,
    pub integration_service: Arc<I>,
}
```

Access in handlers:

```rust
pub async fn handler<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
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
pub async fn my_new_handler<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
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
pub fn create_router<R, P, Rend, E, I>(...) -> Router {
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
pub async fn handler<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path(id): Path<u32>,              // Optional: path params
    Query(params): Query<QueryDto>,   // Optional: query params
    Json(payload): Json<RequestDto>,  // Optional: request body
) -> Result<Json<ResponseDto>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
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
