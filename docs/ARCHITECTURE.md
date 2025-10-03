# Architecture Documentation

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [System Architecture](#system-architecture)
4. [Folder Structure](#folder-structure)
5. [Component Interactions](#component-interactions)
6. [Adding New Components](#adding-new-components)
7. [Data Flow](#data-flow)
8. [Testing Strategy](#testing-strategy)

---

## Overview

This is a personal life statistics tracking system built in Rust following **onion (clean) architecture principles**. The system allows logging various life statistics (time slept, studied, piano practice, etc.) and visualizing them through plots.

**Key Features:**

- Multiple logging methods (CLI, stopwatch, MIDI, heartbeat, web interface)
- Support for 5 statistic types: Habit, Integer Amount, Point In Time, Time Amount, Time Spans
- Goal tracking and progress visualization
- Real-time plot generation
- Cross-language support (Rust, Python, Lua)

**Design Philosophy:**

- Domain logic is isolated and independent
- Dependencies point inward (outer layers depend on inner layers, never the reverse)
- Type safety where possible, with clear boundaries for external integrations
- Single source of truth for statistics data

---

## Design Principles

### 1. Onion Architecture

The system follows strict onion/clean architecture with clearly defined layers:

```
┌─────────────────────────────────────────────┐
│         Application Layer (Loggers)         │  ← CLI, MIDI, Stopwatch, Web UI
├─────────────────────────────────────────────┤
│        Interface Adapters (Web API)         │  ← HTTP endpoints, transfer objects
├─────────────────────────────────────────────┤
│    Application Services (Use Cases)         │  ← Business logic orchestration
├─────────────────────────────────────────────┤
│    Domain Layer (Core Business Logic)       │  ← Statistics, goals, value objects
├─────────────────────────────────────────────┤
│    Infrastructure (Persistence, Events)     │  ← Database, event publishing
└─────────────────────────────────────────────┘
```

### 2. Dependency Rule

**All dependencies point inward.** The domain layer has zero external dependencies. Application services depend on domain. Infrastructure implements domain contracts (repositories).

### 3. Single Responsibility

Each logger application has one job: capture a specific type of activity and log it. Core business rules live in the domain layer, not scattered across loggers.

### 4. Contracts via Traits

Boundaries between layers are defined by Rust traits (Repository patterns, etc.). This enables testing with mocks and keeps layers decoupled.

---

## System Architecture

### High-Level Component Diagram

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ Stopwatch    │  │ MIDI Logger  │  │ CLI Logger   │
│ Logger       │  │              │  │              │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └─────────────────┼─────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │   Application        │
              │   Services           │◄──────────┐
              │  (logging.rs, etc)   │           │
              └──────────┬───────────┘           │
                         │                       │
                         ▼                       │
              ┌──────────────────────┐           │
              │   Domain Model       │           │
              │  (Statistics, Goals) │           │
              └──────────┬───────────┘           │
                         │                       │
                         ▼                       │
              ┌──────────────────────┐           │
              │   Repositories       │           │
              │   (Traits)           │           │
              └──────────┬───────────┘           │
                         │                       │
              ┌──────────▼───────────┐           │
              │   Infrastructure     │           │
              │  (SQLite/Postgres)   │───────────┘
              └──────────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │   Event Publisher    │
              │   (D-Bus/Unix Socket)│
              └──────────────────────┘
                         │
                         ▼
              ┌──────────────────────┐
              │   Wallpaper UI       │
              └──────────────────────┘
```

### External Integrations

```
┌─────────────┐     HTTP      ┌──────────────┐
│  Browser    │◄─────────────►│   Web UI     │
│  Extension  │               │ (Rocket/Axum)│
└─────────────┘               └──────┬───────┘
                                     │
┌─────────────┐     HTTP            │
│  Anki       │◄────────────────────┤
│  Addon      │                     │
│  (Python)   │                     ▼
└─────────────┘          ┌──────────────────┐
                         │   Web API        │
                         │   (web_api crate)│
                         └──────────────────┘
```

---

## Folder Structure

### Core Domain (`core/core/domain/model/`)

**Purpose:** Pure business logic with zero dependencies.

```
model/
├── statistic/
│   ├── entities/           # Statistic types (Habit, IntegerAmount, etc.)
│   ├── value_objects/      # MyTime, TimeFrame, Percentage, etc.
│   ├── repositories/       # Repository trait definitions
│   └── goals/             # Goal definitions and scoring
├── plot/                  # Plot generation logic
└── value_objects/         # Shared value objects
```

**Key files:**

- `statistic/entities/logable_statistics/*.rs` - The 5 statistic types
- `statistic/repositories/statistic_repository.rs` - Repository contract
- `statistic/value_objects/goals/*.rs` - Goal definitions

**Rule:** This layer imports NOTHING from outer layers. Only standard library and internal domain types.

### Application Services (`core/application/service/`)

**Purpose:** Orchestrate use cases by coordinating domain objects and repositories.

```
service/
├── logging.rs           # Core logging use cases
├── goal_management.rs   # Goal CRUD operations
├── statistics.rs        # Statistics queries
├── plot.rs             # Plot generation coordination
├── app_context.rs      # Dependency injection container
└── repositories/       # Repository trait re-exports
```

**Key functions:**

```rust
pub fn log_statistic(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
    value: &str,
    update_policy: Option<&str>,
) -> Result<()>
```

**Rule:** Services coordinate domain objects but contain no business logic themselves.

### Infrastructure Layer

#### Persistence (`core/infrastructure/persistence/`)

**Purpose:** Implement repository traits with actual database access.

```
persistence/
├── repositories/              # Repository implementations
│   ├── postgres_log_repository.rs
│   └── postgres_goal_repository.rs
├── sqlite_database/          # SQLite schema and queries
├── migrations/               # Diesel migrations
└── pool.rs                  # Database connection pooling
```

**Rule:** Implements domain repository traits. Domain never imports from here.

#### Web API (`core/infrastructure/web_api/`)

**Purpose:** HTTP interface for external clients (non-Rust applications).

```
web_api/
├── client.rs    # HTTP client for consumers
├── goal.rs      # Goal-related endpoints
└── types.rs     # Request/response DTOs
```

**Endpoints provided:**

- `POST /api/log` - Log a statistic value
- `GET /api/statistics/{id}` - Get statistic data
- `POST /api/goals` - Create/update goals
- `GET /api/goals` - List goals

### Logger Applications (`logger_applications/`)

**Purpose:** Independent applications that log specific activities.

Each logger is a separate binary that imports the application services:

```
logger_applications/
├── stopwatch_logger/    # Manual time tracking with UI
├── midi_logger/         # Log piano practice via MIDI
├── cli_logger/          # Command-line logging interface
├── heart_beat_logger/   # Periodic background logging
├── mpv_logger/          # Video watching (Lua script)
└── third_party/
    └── fitbit/          # Sync Fitbit data
```

**Pattern:** Each logger:

1. Captures activity-specific data
2. Transforms it to domain format
3. Calls application service functions
4. Handles its own UI/interaction logic

### Transfer Objects (`transfer_objects/`)

**Purpose:** Shared data structures for cross-crate communication.

```
transfer_objects/
├── statistic_id.rs    # Shared identifier type
├── raw_goal.rs        # Goal transfer format
└── plot_image.rs      # Plot image data
```

**Usage:** Imported by both domain and external clients to avoid circular dependencies.

### UI Components (`ui/`)

**Purpose:** User-facing visualization and web interface.

```
ui/
├── wallpaper/        # Desktop wallpaper plot rendering
├── web_ui/           # Web-based logging interface (Rocket)
└── event_sender/     # Event publishing utility
```

### Events (`events/`)

**Purpose:** Event publishing system for real-time updates.

Currently uses D-Bus/Unix sockets to notify wallpaper of data changes.

---

## Component Interactions

### Example: Logging a Statistic from CLI

```rust
// 1. User runs CLI logger
$ cli_logger --statistic sleep --date 2025-10-02 --value "8h"

// 2. CLI Logger (Application Layer)
fn main() {
    let app_context = build_app_context(); // DI container

    // 3. Call application service
    logging::log_statistic(
        &app_context,
        StatisticId::from("sleep"),
        NaiveDate::from_ymd(2025, 10, 2),
        "8h",
        None,
    )?;
}

// 4. Application Service (logging.rs)
pub fn log_statistic(...) -> Result<()> {
    // Get statistic entity from repository
    let statistic = app_context.statistic_repository.get_by_id(statistic_id)?;

    // 5. Domain logic validates and stores
    statistic.log_from_string(date, value, update_policy)?;

    // 6. Publish event for UI updates
    app_context.event_publisher.publish_event_async(Event::ValueUpdated(statistic_id));

    Ok(())
}

// 7. Infrastructure persists to database
// 8. Wallpaper receives event and redraws plots
```

### Example: Non-Rust Client (Anki Addon)

```python
# AnkiLogger/statistic_logging.py
import requests

def log_study_time(minutes):
    response = requests.post(
        "http://localhost:8000/api/log",
        json={
            "statistic_id": "anki_study",
            "date": "2025-10-02",
            "value": f"{minutes}m"
        }
    )
    return response.json()
```

The web API translates this to application service calls internally.

---

## Adding New Components

### How to Add a New Logger Application

**Goal:** Create a new logger in 30 minutes without touching core domain.

#### Step 1: Create the Binary Crate

```bash
cd logger_applications
cargo new my_logger
```

#### Step 2: Add Dependencies

```toml
# logger_applications/my_logger/Cargo.toml
[dependencies]
application_service = { path = "../../core/application/service" }
transfer_objects = { path = "../../transfer_objects" }
chrono = "0.4"
anyhow = "1.0"
```

#### Step 3: Build AppContext

```rust
// src/main.rs
use application_service::app_context::AppContext;

fn main() -> anyhow::Result<()> {
    // Connect to database and build dependencies
    let app_context = AppContext::build_from_config("config.json")?;

    // Your logger logic here
    log_my_activity(&app_context)?;

    Ok(())
}
```

#### Step 4: Log Statistics

```rust
use application_service::logging;
use chrono::Local;
use transfer_objects::StatisticId;

fn log_my_activity(ctx: &AppContext) -> anyhow::Result<()> {
    let today = Local::now().date_naive();

    // Example: Log that user completed a habit
    logging::log_statistic(
        ctx,
        StatisticId::from("my_habit"),
        today,
        "true",  // Boolean value for habits
        None,
    )?;

    // Example: Log time spent
    logging::log_statistic(
        ctx,
        StatisticId::from("my_activity_time"),
        today,
        "2h30m",  // Duration format
        Some("add"),  // Update policy: add to existing
    )?;

    Ok(())
}
```

#### Step 5: Value Formatting Reference

Refer to `application_service/logging.rs` documentation:

```rust
/// Value Formatting Guidelines:
/// - Times: "HH:MM:SS" or "HH:MM" (e.g., "12:30:00", "12:30")
/// - Durations: number + unit (s/m/h) (e.g., "5m", "2h", "30s")
/// - Booleans: "true" or "false"
/// - Integers: any valid i64 (e.g., "42", "1000")
/// - Time frames: "HH:MM-HH:MM" (e.g., "09:00-10:30")
```

**That's it!** Your logger is now integrated into the system.

### How to Add a Non-Rust Client

Use the HTTP API at `http://localhost:8000`:

```javascript
// Example: Browser extension
fetch("http://localhost:8000/api/log", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    statistic_id: "web_browsing",
    date: "2025-10-02",
    value: "3h15m",
  }),
});
```

See `core/infrastructure/web_api/README.md` for full API documentation.

---

## Data Flow

### Write Path (Logging)

```
Logger App → AppContext → log_statistic()
    → Repository.get_by_id()
    → Statistic.log_from_string() [domain validation]
    → Repository.save() [persistence]
    → EventPublisher.publish() [notification]
    → Wallpaper updates
```

### Read Path (Visualization)

```
Wallpaper → AppContext → get_statistics()
    → Repository.get_logs()
    → Plot.generate() [domain plotting logic]
    → Image written to disk
    → Desktop wallpaper updated
```

### Goal Management

```
Web UI → goal_management::set_goal()
    → Goal entity created/updated
    → Repository.save_goal()
    → Statistics queries recalculate scores
```

---

## Testing Strategy

### Unit Tests (Domain Layer)

Test pure business logic without any infrastructure:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_habit_logging() {
        let habit = HabitStatistic::new(/* ... */);
        habit.log_value(date, true).unwrap();
        assert_eq!(habit.get_value(date), Some(true));
    }
}
```

### Integration Tests (Application Services)

Use mock repositories from `testing_utilities`:

```rust
use application_service::testing::MockAppContextBuilder;

#[test]
fn test_log_statistic_end_to_end() {
    let ctx = MockAppContextBuilder::new()
        .with_mock_statistic_repository()
        .build();

    let result = logging::log_statistic(&ctx, id, date, "5h", None);
    assert!(result.is_ok());
}
```

### Contract Tests (Web API)

Test that HTTP endpoints correctly map to application services.

---

## Common Patterns

### Pattern 1: Value Objects

Prefer value objects over primitives:

```rust
// ❌ Bad
fn calculate_score(minutes: i64) -> f64 { ... }

// ✅ Good
fn calculate_score(duration: MyDuration) -> Percentage { ... }
```

### Pattern 2: Repository Pattern

Never access database directly from domain:

```rust
// ❌ Bad (domain depends on infrastructure)
impl Statistic {
    fn save(&self) {
        diesel::insert_into(table).execute(/* ... */);
    }
}

// ✅ Good (domain defines contract, infrastructure implements)
trait StatisticRepository {
    fn save(&self, statistic: &Statistic) -> Result<()>;
}
```

### Pattern 3: Error Handling

Use `anyhow` for application errors, custom types for domain errors:

```rust
// Domain layer
pub enum StatisticError {
    InvalidValue(String),
    DateOutOfRange,
}

// Application layer
pub fn log_statistic(...) -> anyhow::Result<()> {
    statistic.log_value(...).context("Failed to log statistic")?;
    Ok(())
}
```

---

## Future Considerations

### Planned Enhancements

- [ ] Remove wallpaper dependency (replace with web-based dashboards)
- [ ] Remote deployment support (run core service on home server)
- [ ] Mobile app integration via HTTP API
- [ ] Backup and sync across devices

### Non-Goals

- We will NOT split into microservices (unnecessary complexity)
- We will NOT add real-time streaming (current event system is sufficient)
- We will NOT support multi-user scenarios (personal tool)

---

## Getting Started (For Contributors)

1. **Clone and build:**

   ```bash
   git clone <repo>
   cargo build --release
   ```

2. **Run database migrations:**

   ```bash
   cd core/infrastructure/persistence
   diesel migration run
   ```

3. **Start the web UI:**

   ```bash
   cd ui/web_ui
   cargo run
   ```

4. **Try a logger:**

   ```bash
   cd logger_applications/cli_logger
   cargo run -- --help
   ```

5. **Read the docs:**
   - `statistics.md` - Statistic types reference
   - `README.md` - Project overview
   - This file - Architecture details

---

## Questions?

- **"Where do I add a new statistic type?"** → `core/domain/model/src/statistic/entities/logable_statistics/`
- **"How do I add a new logger?"** → See [Adding New Components](#adding-new-components)
- **"Where's the database schema?"** → `infrastructure/persistence/migrations/`
- **"How do goals work?"** → `core/domain/model/src/statistic/value_objects/goals/`
- **"Can I use this from Python/JavaScript?"** → Yes! Use the HTTP API at `localhost:8000`
