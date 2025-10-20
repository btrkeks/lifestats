# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LifeStats is a personal observability platform for tracking daily habits and wellness metrics. Built in Rust using **Ports & Adapters (Hexagonal) Architecture**. Currently mid-refactor on the `refactor` branch.

## Essential Commands

### Building
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo check                    # Quick compile check
```

### Testing
```bash
cargo test --workspace         # All tests
just test                      # Same as above (using justfile)

# Database backward compatibility test
just test-db-compat

# Test specific packages
cargo test -p lifestats-core
cargo test -p lifestats-application
cargo test -p lifestats-persistence
```

### Running the HTTP API
```bash
cargo run -p lifestats-web-api
# Binds to 0.0.0.0:3000
# Requires DATABASE_URL environment variable for live usage
```

### Linting & Formatting
```bash
cargo clippy                   # Lint
cargo fmt                      # Format
```

## Environment Variables

| Variable | Purpose | Default/Required |
|----------|---------|------------------|
| `DATABASE_URL` | SQLite DB path for live usage | Required for live (tests use in-memory) |
| `COMPAT_DB_URL` | Snapshot DB for compatibility tests | Required for compat test |
| `RUST_LOG` | Log level (tracing-subscriber) | `info,web_api=debug,tower_http=debug` |
| `FITBIT_CLIENT_ID` | Fitbit OAuth client ID | Optional (defaults to dummy) |
| `FITBIT_CLIENT_SECRET` | Fitbit OAuth client secret | Optional |
| `FITBIT_REDIRECT_URI` | Fitbit OAuth redirect URI | `http://localhost:3000/integrations/fitbit/auth/callback` |
| `FITBIT_SCOPES` | Space-separated Fitbit scopes | `activity sleep` |
| `FITBIT_ENABLED_METRICS` | Comma-separated metrics to sync | `steps,sleep_duration` |
| `TIMEZONE` | Timezone for Fitbit sync | `UTC` |

## Architecture

LifeStats follows **Hexagonal (Ports & Adapters) Architecture** with strict dependency direction: dependencies always point inward toward the domain.

### Crate Structure

```
core/
├── core/                              # Domain model (ZERO dependencies)
│   └── Contains: Statistic types, Goal types, Port trait definitions
├── application/                       # Use case orchestration (CQRS)
│   └── Contains: Command/Query handlers, session tracking
│   └── Depends ONLY on: core/core
├── infrastructure/
│   ├── persistence/                   # SQLite + Diesel repository implementations
│   ├── plotting/                      # Plot rendering (plotters crate)
│   ├── web_api/                       # HTTP server (Axum, port 3000)
│   ├── fitbit_client/                 # Fitbit API HTTP client
│   └── system_clock/                  # System clock implementation
└── plot-domain/                       # Plot-specific domain types

events/
├── event_impl/                        # Event types & publisher trait
└── event_sender/                      # CLI to emit synthetic events

logger_applications/                   # Legacy loggers (excluded from workspace)
├── AnkiLogger/                        # Anki add-on (Python)
├── cli_logger/                        # Terminal logger
├── stopwatch_logger/                  # Interactive stopwatch
├── fitbit_logger/                     # Legacy Fitbit sync
├── midi_logger/                       # MIDI input tracker
├── ebook-reader/                      # E-reader integration
├── mpv_logger/                        # MPV video tracker
└── web-activity-time-tracker/         # Browser extension

ui/
├── wallpaper/                         # Desktop wallpaper updater
└── web_ui/                            # Browser-based logging/gallery

lifestats-bootstrap/                   # Dependency injection composition root
```

### Key Architectural Constraints

1. **Domain purity**: `core/core` has ZERO external dependencies (except std library traits like Clone, Debug)
2. **Dependency direction**: Always points inward toward domain
3. **Port definitions**: All ports (traits) are defined in the domain layer
4. **Adapter implementations**: Infrastructure provides concrete implementations
5. **Composition root**: `lifestats-bootstrap` wires everything together via dependency injection

### Workspace Members

**Active (in workspace):**
- `core/core`, `core/application`
- `core/infrastructure/{persistence,plotting,web_api,fitbit_client,system_clock}`
- `core/plot-domain`
- `logger_applications/{stopwatch_logger,midi_logger}`
- `lifestats-bootstrap`

**Excluded (legacy/WIP):**
- Most `logger_applications/*`, `events/*`, `ui/*`

## Domain Model (Critical Concepts)

### Statistic Types

**Two fundamental categories enforced at compile-time:**

1. **Logable Statistics** (accept direct input):
   - `TimeSpans` – tracks intervals (start/end times)
   - `IntegerAmount` – numeric counts
   - `Habit` – boolean occurrences
   - `PointInTime` – specific timestamps
   - Implements: `Logable` trait with pure `calculate_new_log_value` method

2. **Derived Statistics** (computed from other statistics):
   - `SummingStatistic` – aggregates multiple `TimeSpans`
   - `TimeDifference` – delta between two `PointInTime` stats
   - Implements: `Computable` trait with pure `compute_value` method
   - **CRITICAL**: You cannot log directly to a derived statistic (enforced at compile-time)

### Goals

- **Daily goals**: Targets for specific dates
- **Default goals**: Fallback targets when no daily goal exists
- Each statistic type supports different goal metadata (e.g., "is higher better?", efficiency percentages)

### Events

When data or goals change, the system publishes events (`ValueLogged`, `GoalSet`). Subscribers (wallpaper updater, plot renderer) listen and react.

### Core Ports (Trait Definitions)

Defined in domain, implemented by infrastructure:
- `StatisticRepository` – load/save statistics and goals
- `TagQueryableStatisticRepository` – query statistics by tags
- `PlotRenderer` – generate plot images
- `EventPublisher` – publish domain events
- `FitbitOAuthApi`, `FitbitDataApi` – Fitbit integration
- `TokenStorePort` – OAuth token persistence
- `Clock` – Time abstraction for testing

## CQRS Pattern

The application layer uses Command Query Responsibility Segregation:

### Commands (write operations)
Located in `core/application/src/use_cases/commands/`:
- `CreateStatisticCommand` – create new statistic
- `LogValueCommand` – log a value for a statistic
- `SetDailyGoalCommand` – set goal for specific date
- `SetDefaultGoalCommand` – set default goal
- `DeleteDailyGoalCommand` – remove daily goal
- `DeleteDefaultGoalCommand` – remove default goal
- `DeleteStatisticCommand` – delete statistic
- `SyncFitbitDataCommand` – sync data from Fitbit

Each command has a handler (`*CommandHandler`) that orchestrates domain objects and ports.

### Queries (read operations)
Located in `core/application/src/use_cases/queries/`:
- `GetStatisticById` – retrieve statistic metadata
- `GetDefaultGoal` – retrieve default goal
- `GetTimeSpansTotal` – compute total duration
- `get_loggable_metadata` – list all loggable statistics

## Composition Root Pattern

The `lifestats-bootstrap` crate is the **composition root** where dependency injection happens:

1. Instantiates concrete infrastructure (repositories, renderers, HTTP clients)
2. Wires command/query handlers with their dependencies
3. Returns an `Application` struct containing all services
4. Used by `web_api` and logger applications

**Key function:** `build_services() -> Result<Application>`

## Data & Persistence

### Database
- **Engine**: SQLite via Diesel ORM
- **Connection pooling**: Configured with WAL mode in `core/infrastructure/persistence/src/pool.rs`
- **Migrations**: Located in `core/infrastructure/persistence/migrations/`
- **Auto-migration**: `web_api` runs migrations automatically on startup

### Migrations
```bash
cd core/infrastructure/persistence
diesel migration run
```

### Backward Compatibility
The compatibility test (`just test-db-compat`) ensures new code can read older database snapshots. Requires snapshot DB in `test_data/test_snapshot.db`.

## HTTP API Routes

Server binds to `0.0.0.0:3000` (see `core/infrastructure/web_api/src/main.rs`):

- `GET /statistics` – list all statistics
- `POST /statistics/:id/log/:date` – log a value
- `POST /statistics/:id/log_time_frame/:date` – log a time frame
- `GET /plots/:id/image` – retrieve plot image
- `POST /integrations/provision` – provision integration (e.g., Anki bundles)
- Fitbit integration routes (OAuth flow)

Routes defined in `core/infrastructure/web_api/src/routes/`.

## Adding New Features

### Adding a New Statistic Type

1. Add type to `core/core/src/statistic/types/{logable,derived}_statistics/`
2. Implement `Logable` or `Computable` trait
3. Update `AnyStatistic` enum if needed
4. Add domain tests (pure business logic, no mocking)
5. Update application layer if needed

### Adding a New Use Case (Command/Query)

1. Add command/query struct under `core/application/src/use_cases/{commands,queries}/`
2. Create handler struct implementing the use case
3. Keep handler thin: orchestrate domain types via repository/event ports
4. Domain logic stays in domain entities (pure methods)
5. Add unit tests using mock ports (mockall generates these)

### Adding a New Logger Application

1. Create binary crate (or add to `logger_applications/`)
2. Add dependencies:
   ```toml
   lifestats-bootstrap = { path = "../lifestats-bootstrap" }
   lifestats-application = { path = "../core/application" }
   ```
3. In `main.rs`:
   ```rust
   use lifestats_bootstrap::build_services;

   fn main() -> anyhow::Result<()> {
       let app = build_services()?;

       // Your logger-specific logic
       let data = capture_activity_data()?;

       // Instantiate handler on-demand
       let handler = LogValueCommandHandler::new(
           app.statistic_repository.clone(),
           app.event_publisher.clone(),
       );

       handler.handle(LogValueCommand { /* ... */ })?;
       Ok(())
   }
   ```

### Adding a New HTTP Route

1. Add handler function to `core/infrastructure/web_api/src/routes/`
2. Register route in `routes::create_router`
3. Add integration test
4. Handler should instantiate command/query handlers on-demand

## Testing Strategy

### Unit Tests (Domain Layer)
Test pure business logic with zero mocking:
```rust
#[test]
fn test_log_value_calculation() {
    let statistic = IntegerAmountStatistic::new(/* metadata */);
    let (new_value, changed) = statistic.calculate_new_log_value(
        Some(10), 5, AmountUpdatePolicy::Sum
    ).unwrap();
    assert_eq!(new_value, 15);
}
```

### Integration Tests (Application Layer)
Test use case handlers using mock ports (generated by mockall):
```rust
#[test]
fn test_log_value_command() {
    let mut mock_repo = MockStatisticRepository::new();
    mock_repo.expect_find_metadata_by_id().returning(/* ... */);
    mock_repo.expect_save_log_value().returning(|| Ok(()));

    let handler = LogValueCommandHandler::new(
        Arc::new(mock_repo),
        Arc::new(NoOpEventPublisher)
    );

    let result = handler.handle(LogValueCommand { /* ... */ });
    assert!(result.is_ok());
}
```

## Build Configuration

### Shared Target Directory
Builds use `/tmp/cargo-target-shared` (see `.cargo/config.toml`). This is configured for performance on systems with fast temp storage.

### sccache
Configured in `.cargo/config.toml` for faster rebuilds. If unavailable, remove or comment out:
```toml
rustc-wrapper = "sccache"
```

## Key Files to Reference

- `docs/ARCHITECTURE.md` – detailed architecture diagrams
- `docs/statistics.md` – statistic type behaviors
- `docs/IDEAS.md` – design ideas and future explorations
- `docs/COMMIT_CONVENTIONS.md` – git commit message format and conventions
- `README.md` – comprehensive project documentation
- `lifestats-bootstrap/src/lib.rs` – dependency injection patterns
- `core/application/src/use_cases/commands/log_value/handler.rs` – canonical command handler example

## Common Patterns

### Instantiating Command Handlers

Handlers are instantiated on-demand (not stored in AppState) to keep dependencies lean:

```rust
// In route handler
let handler = LogValueCommandHandler::new(
    state.statistic_repository.clone(),
    state.event_publisher.clone(),
);
handler.handle(command).await?;
```

### Pure Domain Methods

Domain entities should only contain pure methods (no I/O):

```rust
// GOOD: Pure method in domain
impl TimeSpansStatistic {
    pub fn calculate_new_log_value(&self, existing: Option<Vec<TimeFrame>>, new: TimeFrame)
        -> Result<Vec<TimeFrame>> {
        // Pure logic only
    }
}

// BAD: Domain entity calling repository
impl TimeSpansStatistic {
    pub fn save_to_db(&self, repo: &dyn Repository) {
        // NEVER do this!
    }
}
```

### Orchestration in Application Layer

The application layer orchestrates domain calls and repository calls:

```rust
pub fn handle(&self, command: LogValueCommand) -> Result<()> {
    // 1. Load from repository
    let statistic = self.repo.find_by_id(command.id)?;
    let current = self.repo.find_log_value(command.id, command.date)?;

    // 2. Call pure domain logic
    let (new_value, changed) = statistic.calculate_new_log_value(current, command.value)?;

    // 3. Save via repository
    if changed {
        self.repo.save_log_value(command.id, command.date, new_value)?;
        self.publisher.publish(ValueLogged { ... })?;
    }

    Ok(())
}
```

## Troubleshooting

### DATABASE_URL not set
- Tests pass (use in-memory SQLite)
- Live usage requires setting this variable
- Create `.env` file in project root or export variable

### Port 3000 in use
- Change in `core/infrastructure/web_api/src/main.rs` (line 78)

### sccache errors
- Remove `rustc-wrapper = "sccache"` from `.cargo/config.toml`

### Shared target directory too large
- Clear with `cargo clean`
- Or change `target-dir` in `.cargo/config.toml` to `./target`

### Legacy logger applications
- Excluded from workspace
- May use outdated APIs
- Refer to this file for architectural guidance when updating them
