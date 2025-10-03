# Architecture Documentation

## Table of Contents

1. [Overview](#overview)
2. [Design Principles: Ports & Adapters](#design-principles-ports--adapters)
3. [Architectural Layers & Crates](#architectural-layers--crates)
4. [The Domain Model](#the-domain-model)
5. [Component Interaction Example](#component-interaction-example)
6. [Adding a New Logger](#adding-a-new-logger)
7. [Testing Strategy](#testing-strategy)

---

## Overview

LifeStats is a personal observability platform built in Rust. It follows a strict **Ports & Adapters (Hexagonal) Architecture** to ensure a clean separation between core business logic and external technologies like databases, user interfaces, and third-party integrations.

**Design Philosophy:**

- The core domain logic is the most valuable asset and must be protected from external changes.
- Dependencies always point inward toward the domain.
- The core is completely unaware of how it is being used or where its data is stored.
- Boundaries between the core and the outside world are defined by explicit contracts (Rust traits), known as "Ports".

---

## Design Principles: Ports & Adapters

We visualize the system as a hexagon. The code inside the hexagon represents our application's core, while everything outside is an "adapter" that interacts with the core through well-defined "ports".

```
                          DRIVING ADAPTERS (Users of the core)
                 ┌──────────────────┐      ┌──────────────────┐
                 │     CLI Tool     │      │    Web Server    │
                 └────────┬─────────┘      └────────┬─────────┘
                          │                         │
                          │   calls Application     │
                          │        Services         │
                          ▼                         ▼
           ╔══════════════════════════════════════════════════╗
           ║                                                  ║
           ║    APPLICATION CORE (THE "HEXAGON")              ║
           ║                                                  ║
           ║   ┌──────────────────────────────────────────┐   ║
           ║   │         Application Services             │   ║
           ║   │  (Use Cases: LoggingService, etc.)       │   ║
           ║   └─────────────────────┬────────────────────┘   ║
           ║                         │ orchestrates           ║
           ║                         ▼                        ║
           ║   ┌──────────────────────────────────────────┐   ║
           ║   │             Domain Model                 │   ║
           ║   │ (Entities, Value Objects, Business Rules)│   ║
           ║   └─────────────────────┬────────────────────┘   ║
           ║                         │ uses                  .─.
           ║                         ▼                       `·´ PORTS (Traits)
           ║   ┌──────────────────────────────────────────┐   ║  e.g., `StatisticRepository`
           ║   │                  Ports                   │   ║
           ║   │            (Trait Definitions)           │   ║
           ║   └──────────────────────────────────────────┘   ║
           ║                                                  ║
           ╚══════════════════════════════════════════════════╝
                                     │
                                     │ implements Ports
                                     │
                                     ▼
                 ┌───────────────────┴────────────────────┐
                 │          DRIVEN ADAPTERS               │
                 │   (Implementations used by the core)   │
                 │                                        │
       ┌─────────┴───────────┐                  ┌─────────┴─────────┐
       │ persistence-sqlite  │                  │   plot-renderer   │
       │ (implements         │                  │ (implements       │
       │ StatisticRepository)│                  │ PlotRenderer port)│
       └─────────────────────┘                  └───────────────────┘
```

---

## Architectural Layers & Crates

Our architecture is enforced by the Rust crate structure.

### `core/lifestats-domain`

- **Description:** The absolute center of the hexagon. Contains all core business logic, entities, value objects, and the definitions of the Ports (traits).
- **Key Contents:** `SourceStatistic`, `DerivedStatistic`, `Goal` types, `StatisticRepository` trait.
- **Rule:** Has **zero** dependencies on other crates in the workspace. It is pure, portable Rust.

### `core/lifestats-application`

- **Description:** The use case layer. It orchestrates the domain objects to execute specific user actions.
- **Key Contents:** `LoggingService`, `GoalManagementService`, `StatisticQueryService`.
- **Rule:** Depends only on `lifestats-domain`. Contains no business logic; it only coordinates.

### `core/adapters/*`

- **Description:** Concrete implementations of the Ports defined in the domain. These are the "Driven Adapters".
- **Crates:**
  - `adapters/persistence-sqlite`: Implements the `StatisticRepository` trait using `diesel` and SQLite.
  - `adapters/plot-renderer`: (Future) Implements a `PlotRenderer` trait using the `plotters` library.
- **Rule:** Adapters depend on `lifestats-application` and `lifestats-domain`, but the core **never** depends on an adapter.

### `interfaces/*`

- **Description:** User-facing applications. These are the "Driving Adapters" that initiate actions.
- **Crates:** `cli`, `web-server`, `stopwatch-logger`, `midi-logger`, etc.
- **Rule:** These are binaries that wire the system together. They instantiate the application services and inject the concrete adapter implementations (e.g., `persistence-sqlite`).

---

## The Domain Model

To ensure type safety and clarity, our domain model makes a crucial distinction between two kinds of statistics.

### `SourceStatistic`

- **Description:** A statistic that is a primary source of data. Its values are logged directly.
- **Examples:** `TimeSpans`, `IntegerAmount`, `Habit`, `PointInTime`.
- **Key Trait:** Implements the `Logable` trait, providing pure methods like `calculate_new_log_value`.

### `DerivedStatistic`

- **Description:** A computed view over one or more `SourceStatistic`s. You cannot log data to it directly.
- **Examples:** `SummingStatistic` (sums durations from multiple `TimeSpans` stats), `TimeDifference` (calculates the difference between two `PointInTime` stats).
- **Key Trait:** Implements the `Computable` trait, providing a pure `compute_value` method that takes the necessary source data as an argument.

This separation prevents logical errors at compile-time (e.g., trying to log a value to a `SummingStatistic`) and makes the application layer's job simpler and safer.

---

## Component Interaction Example

This shows the new, orchestrated flow for logging a value from the CLI. Notice how the domain entity (`statistic`) never saves itself.

```rust
// 1. User runs the CLI interface
$ lifestats-cli log --id study_time --date 2025-10-02 --value "45m"

// 2. `interfaces/cli/src/main.rs` (Driving Adapter)
fn main() {
    // A. Instantiate concrete adapters
    let pool = create_db_pool();
    let stats_repo = Arc::new(SqliteStatisticRepository::new(pool));
    let event_pub = Arc::new(DbusEventPublisher::new());

    // B. Instantiate the Application Service, injecting dependencies
    let logging_service = LoggingService::new(stats_repo, event_pub);

    // C. Create and dispatch the command
    let command = LogValueCommand { /* ... from args ... */ };
    logging_service.handle_log_value(command)?;
}

// 3. `application/src/use_cases/logging_service.rs` (Application Service)
pub fn handle_log_value(&self, command: LogValueCommand) -> Result<()> {
    // A. Load the necessary data from persistence
    let statistic = self.stats_repo.find_metadata_by_id(command.id)?;
    let current_value = self.stats_repo.find_log_value(command.id, command.date)?;

    // B. Call the PURE domain logic to get the new state
    //    The domain knows nothing about the database.
    let (new_value, changed) = statistic.calculate_new_log_value(
        current_value,
        command.value,
        command.policy
    )?;

    // C. Save the new state back to persistence
    if changed {
        self.stats_repo.save_log_value(command.id, command.date, new_value)?;
        self.event_pub.publish(ValueUpdatedEvent { ... });
    }

    Ok(())
}
```

---

## Adding a New Logger

**Goal:** Create a new logger application without touching any `core` crates.

#### Step 1: Create the Binary Crate

```bash
cd interfaces
cargo new my_logger
```

#### Step 2: Add Dependencies

```toml
# interfaces/my_logger/Cargo.toml
[dependencies]
lifestats-application = { path = "../../core/lifestats-application" }
lifestats-domain = { path = "../../core/lifestats-domain" }
# IMPORTANT: Add the concrete adapter you need
persistence-sqlite = { path = "../../core/adapters/persistence-sqlite" }
# ... other dependencies like `chrono`, `anyhow` ...
```

#### Step 3: Implement `main.rs`

```rust
// interfaces/my_logger/src/main.rs
use lifestats_application::use_cases::{LoggingService, LogValueCommand};
use persistence_sqlite::SqliteStatisticRepository; // The concrete implementation
// ... other imports

fn main() -> anyhow::Result<()> {
    // 1. Wire the application together
    let pool = create_db_pool_from_env();
    let repo = Arc::new(SqliteStatisticRepository::new(pool));
    let publisher = Arc::new(NoOpEventPublisher::new()); // Or a real one
    let logging_service = LoggingService::new(repo, publisher);

    // 2. Your logger's specific logic
    let my_data = capture_activity_data()?; // e.g., read from a sensor

    // 3. Create a command object
    let command = LogValueCommand {
        statistic_id: /* ... */,
        date: /* ... */,
        value: my_data.to_string(),
        policy: Some("sum".to_string()),
    };

    // 4. Execute the use case
    logging_service.handle_log_value(command)?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests (`lifestats-domain`)

Test pure business logic with zero mocking of external dependencies.

```rust
#[cfg(test)]
mod tests {
    #[test]fn test_log_value_calculation() {
        let statistic = IntegerAmountStatistic::new(/* metadata only */);
        let (new_value, changed) = statistic.calculate_new_log_value(
            Some(10), 5, AmountUpdatePolicy::Sum
        ).unwrap();
        assert!(changed);
        assert_eq!(new_value, 15);
    }
}
```

### Integration Tests (`lifestats-application`)

Test application services by injecting mock implementations of the repository ports.

```rust
use lifestats_domain::statistic::ports::MockStatisticRepository;

#[test]
fn test_log_value_use_case() {
    let mut mock_repo = MockStatisticRepository::new();

    // Setup expectations on the mock
    mock_repo.expect_find_metadata_by_id().returning(...);
    mock_repo.expect_find_log_value().returning(|| Ok(Some(10)));
    mock_repo.expect_save_log_value()
        .with(.., .., 15) // Assert it saves the correct value
        .returning(|| Ok(()));

    let service = LoggingService::new(Arc::new(mock_repo), ...);
    let command = LogValueCommand { value: 5, policy: "sum", .. };

    let result = service.handle_log_value(command);
    assert!(result.is_ok());
}
```
