# Application Layer: CQRS Guide

The LifeStats application crate provides the thin orchestration layer that sits between driving adapters (web API, logger apps, integrations) and the pure domain model. The layer is intentionally lightweight: every write is represented as a **command handler**, every read passes through a **query handler**, and no long-lived "service" objects remain.

## Architectural Overview

- **Commands** mutate state. Each command is a strongly-typed request object that implements `cqrs::Command`. Handlers implement `cqrs::CommandHandler` and encapsulate the orchestration required to execute the operation (fetch ports, invoke domain methods, persist, publish events).
- **Queries** read state without side effects. They implement `cqrs::Query`/`cqrs::QueryHandler` and typically delegate directly to repository ports, occasionally shaping return DTOs.
- **Adapters construct handlers on demand.** Nothing retains references to command/query handlers; an HTTP route or CLI command builds the handler with the dependencies it needs (`Arc` clones from `AppState`/`Application`) and immediately calls `handle`.

The core traits live in [`src/cqrs.rs`](src/cqrs.rs). They define the minimal contract every command and query follows.

## Directory Layout

```
src/
├── commands/                  # state-changing use cases
│   ├── create_statistic/      # example command slices (command.rs, handler.rs)
│   ├── delete_daily_goal/
│   ├── delete_default_goal/
│   ├── delete_statistic/
│   ├── log_value/
│   ├── set_daily_goal/
│   └── set_default_goal/
├── queries/                   # read-only use cases
│   ├── get_all_metadata/
│   ├── get_all_statistics/
│   ├── get_daily_goal/
│   ├── get_default_goal/
│   ├── get_metadata/
│   ├── get_plot_data/
│   ├── get_statistic_by_id/
│   ├── get_streak/
│   └── get_time_spans_metadata/
├── integration/               # adapter-facing integration orchestration
└── session_tracking_service/  # shared heartbeat session for logger apps
```

Each command/query slice contains three tiny files:

- `command.rs` – defines the DTO that implements `Command`/`Query`.
- `handler.rs` – implements the handler; this is where orchestration lives.
- `mod.rs` – re-exports both for ergonomic `use` statements.

## Working with Commands

Command DTOs deliberately avoid stringly-typed data. For example, `commands::create_statistic::CreateStatisticCommand` uses the `StatisticTypeDto` enum and `Vec<StatisticId>` to express intent. Adapters are responsible for parsing user input into these DTOs.

A handler typically follows this pattern:

```rust
let command = LogValueCommand {
    statistic_id: StatisticId(1),
    date,
    payload: LogValuePayload::Integer { value: 42, update_policy },
    statistic,
};

let handler = LogValueCommandHandler::new(
    app_state.statistic_repository.clone(),
    app_state.event_publisher.clone(),
);
handler.handle(command)?;
```

Handlers may:

1. Fetch additional data from ports.
2. Invoke domain methods on `AnyStatistic`/
   other aggregates.
3. Persist through the appropriate repository port.
4. Publish domain events (e.g., `Event::GoalUpdated`).

## Working with Queries

Queries expose focused read models. Most handlers are one or two lines that delegate to a repository port:

```rust
let handler = GetDefaultGoalQueryHandler::new(app_state.statistic_repository.clone());
let result = handler.handle(GetDefaultGoalQuery {
    statistic_id: StatisticId(1),
})?; // -> Option<serde_json::Value>
```

Metadata-oriented queries (`get_all_metadata`, `get_metadata`) return the value object `StatisticMetadata`. The persistence layer tags each metadata instance with `statistic_type`, allowing adapters to render presentation DTOs without touching the domain enum.

## Session Tracking & Integrations

The only remaining long-lived API is the heartbeat session (`session_tracking_service`). Logger adapters reuse this utility to buffer "stopwatch" style interactions. Integration bundles still depend on `integration::IntegrationService`—they model an adapter façade that coordinates multiple commands/queries under the hood.

## Adding a New Use Case

1. Create a new directory under `commands/` or `queries/`.
2. Define a strongly typed DTO in `command.rs`/`query.rs`.
3. Implement the handler in `handler.rs`, using repository/event ports passed via `Arc`.
4. Re-export the types in the slice `mod.rs` and, if necessary, from `commands/mod.rs` or `queries/mod.rs`.
5. Add focused tests using the generated mock ports (`MockStatisticRepository`, `MockEventPublisher`, etc.).

Handlers should remain deterministic and easy to unit test—avoid reaching into adapter details. If something feels adapter-specific, move that logic to the adapter crate.

## Example: Web API Route

The `create_statistic` HTTP route demonstrates the intended flow:

```rust
let statistic_type = StatisticTypeDto::from_str(&payload.statistic_type)?;
let command = CreateStatisticCommand {
    name: payload.name,
    category: payload.category,
    statistic_type,
    source_statistic_ids: payload.source_statistic_ids
        .map(|ids| ids.into_iter().map(StatisticId).collect()),
};

CreateStatisticCommandHandler::new(
    app_state.statistic_repository.clone(),
    app_state.event_publisher.clone(),
)
.handle(command)?;
```

The adapter builds the DTO, constructs the handler with shared infrastructure (`AppState`), and immediately calls `handle`. No singleton services are passed around, and no state is stored in the handler.

## Testing

Every handler ships with unit tests that exercise both happy-path and error scenarios using the mocked ports under `lifestats_core::statistic::ports`. Add new tests alongside the handler to capture regressions quickly. For integration-level behaviour, prefer adapter crate tests (e.g., web API contract tests).

---

Questions or follow-ups? Document assumptions next to the handler or open an issue so future contributors understand the intended flow.
