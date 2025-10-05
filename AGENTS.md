# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LifeStats is a personal observability platform for tracking daily habits and wellness metrics. It follows strict **Ports & Adapters (Hexagonal) Architecture** with a Rust workspace structure.

## Build & Test Commands

```bash
# Build entire workspace
cargo build

# Build in release mode
cargo build --release

# Run tests for all crates
cargo test

# Run tests for a specific crate
cargo test -p lifestats-domain
cargo test -p lifestats-application
cargo test -p persistence
cargo test -p web_api

# Lint with clippy
cargo clippy

# Format code
cargo fmt

# Check code without building
cargo check
```

**Note:** The project uses a shared target directory at `/tmp/cargo-target-shared` and `sccache` for faster builds (see `.cargo/config.toml`).

## Architecture: Ports & Adapters (Hexagonal)

The codebase strictly separates business logic from external concerns. **Dependencies always point inward toward the domain.**

### Workspace Structure

```
core/
├── domain/              # Pure business logic, zero external dependencies
│   ├── statistic/
│   │   ├── types/      # SourceStatistic & DerivedStatistic types
│   │   ├── ports/      # Trait definitions (e.g., StatisticRepository)
│   │   └── traits/     # Logable, Computable
│   └── shared/
├── application/         # Use case orchestration layer
│   └── use_cases/
│       ├── logging_service.rs
│       ├── goal_management_service.rs
│       └── statistic_query_service.rs
└── infrastructure/      # Implementations of domain ports (Driven Adapters)
    ├── persistence/     # SQLite implementations of repository traits
    └── web_api/         # HTTP API adapter

client/                  # HTTP client (in progress)
```

### Critical Architectural Constraints

1. **`core/domain` has ZERO dependencies** on other workspace crates. It defines all ports as traits.
2. **`core/application` depends only on `domain`**. It coordinates domain logic but contains no business rules.
3. **`core/infrastructure/*` implements domain ports**. The domain never depends on infrastructure.
4. **Driving adapters** (CLI tools, web servers, loggers) wire everything together by injecting concrete implementations into application services.

## Domain Model: Two Kinds of Statistics

### SourceStatistic (Logable)

Statistics that accept logged values directly:

- `TimeSpans` - tracks time ranges
- `IntegerAmount` - numeric counts
- `Habit` - boolean occurrences
- `PointInTime` - specific timestamps

Each implements the `Logable` trait with pure methods like `calculate_new_log_value`.

### DerivedStatistic (Computable)

Computed views over source statistics. **You cannot log to these directly.**

- `SummingStatistic` - aggregates multiple TimeSpans
- `TimeDifference` - computes delta between PointInTime stats

Each implements the `Computable` trait with a pure `compute_value` method.

**This separation prevents logical errors at compile-time** (e.g., trying to log to a SummingStatistic).

## Typical Flow: Logging a Value

All logging follows this pattern (see `core/application/src/use_cases/logging_service.rs`):

1. **Load** current state from repository
2. **Call pure domain method** to calculate new state (e.g., `statistic.calculate_new_log_value()`)
3. **Save** new state back via repository
4. **Publish** event if changed

**The domain entity never saves itself.** Application services orchestrate all I/O.

## Database

- SQLite is the default backing store
- Environment variable `DATABASE_URL` points to the live database
- Tests use in-memory SQLite
- Connection pooling with WAL mode enabled
- Repositories live in `core/infrastructure/persistence/`

## Current State: Refactor Branch

The `refactor` branch is bringing the architecture up to production quality. The old codebase had many loggers and interfaces; the new structure consolidates around clean hexagonal boundaries.

**Main branch:** `main`
**Active development:** `refactor` (current branch)

## Testing Strategy

### Unit Tests (`core/domain`)

Test pure business logic with zero I/O or mocking:

```rust
#[test]
fn test_calculate_new_log_value() {
    let stat = IntegerAmountStatistic::new(/* ... */);
    let (new_val, changed) = stat.calculate_new_log_value(
        Some(10), 5, AmountUpdatePolicy::Sum
    ).unwrap();
    assert_eq!(new_val, 15);
}
```

### Integration Tests (`core/application`)

Test use cases with mock repository implementations (use `mockall` if available or manual test doubles).

### Database Compatibility Tests

The project includes backward compatibility tests to ensure new code can read data from older database schemas. This is critical for migrations and schema evolution.

**Running the test:**

```bash
# Using just
just test-db-compat
```

## Adding New Functionality

### Adding a New Logger/Interface

1. Create a new crate in `interfaces/` (if it existed, though current workspace has `client/` instead)
2. Add dependencies on `lifestats-application` and `lifestats-domain`
3. Add concrete adapter dependencies (e.g., `persistence`)
4. In `main.rs`:
   - Instantiate concrete adapters (e.g., `SqliteStatisticRepository`)
   - Instantiate application services with dependency injection
   - Create command objects
   - Execute use cases

See `docs/ARCHITECTURE.md` lines 185-235 for detailed example.

### Adding a New Statistic Type

1. Add type to `core/domain/src/statistic/types/logable_statistics/` or `derived_statistics/`
2. Implement `Logable` or `Computable` trait
3. Update `AnyStatistic` enum if needed
4. Add tests in domain crate
