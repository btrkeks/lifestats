# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lifestats is a personal observability platform for tracking daily habits and wellness metrics. Built in Rust using **Ports & Adapters (Hexagonal) Architecture**. Currently mid-refactor on the `refactor` branch.

## Architectural Constraints

Dependencies always point inward toward the domain.

1. **Domain purity**: `core/core` has ZERO external dependencies (except std library traits like Clone, Debug)
2. **Port definitions**: All ports (traits) are defined in the domain layer; infrastructure provides concrete implementations
3. **Composition root**: `lifestats-bootstrap` wires everything together via dependency injection (`build_services() -> Result<Application>`)
4. **CQRS**: Application layer splits commands (writes) and queries (reads) under `core/application/src/use_cases/{commands,queries}/`. Handlers stay thin; domain logic lives in domain entities as pure methods.

Most `logger_applications/*`, `events/*`, and `ui/*` crates are excluded from the workspace (legacy/WIP). Check the root `Cargo.toml` for the active set.

## Domain Model (Critical Concepts)

**Two statistic categories enforced at compile-time** — you cannot log directly to a derived statistic:

1. **Logable** (`TimeSpans`, `IntegerAmount`, `Habit`, `PointInTime`): implement `Logable` with pure `calculate_new_log_value`
2. **Derived** (`SummingStatistic`, `TimeDifference`): implement `Computable` with pure `compute_value`

**Goals**: daily goals override default goals. Each statistic type carries its own goal metadata (e.g., "is higher better?", efficiency percentages).

**Events**: `ValueLogged` / `GoalSet` are published when data or goals change; subscribers (wallpaper updater, plot renderer) react.

## Local Development

The web API server is running locally at `http://localhost:3000` — query it directly to inspect live behavior. See `core/infrastructure/web_api/README.md` for the full route reference, DTO shapes, error mapping, and handler patterns.

### Android emulators and device

This machine is shared with other projects' agents.

- Start emulators only through `~/.local/bin/agent-emulator` (read its header for usage), never with bare `emulator` or `systemd-run`. It holds a machine-wide slot and caps memory and runtime. Exit code 75 means every slot is busy: wait and retry, never bypass it.
- Use the AVD `ont-widget` (Pixel 8, Android 37) for the Android apps here, started `-read-only` with `-memory 3072`. Stop it when you are done or idle for more than about 10 minutes.
- The owner's phone is often attached over USB. Always pass `-s emulator-<port>` to `adb`, and never install to or modify the phone without the owner's explicit go-ahead.
- Do not build or run tests while the emulator boots. Run heavy builds and test suites under `nice`, and cap Gradle with `-Porg.gradle.workers.max=2`.

## Data & Persistence

- **Engine**: SQLite via Diesel ORM; WAL mode configured in `core/infrastructure/persistence/src/pool.rs`
- **Migrations**: `core/infrastructure/persistence/migrations/`; `web_api` runs them automatically on startup
- **Compatibility test** (`just test-db-compat`) ensures new code can read older DB snapshots (requires `test_data/test_snapshot.db`)

## Adding New Features

**Documentation Maintenance**: When making changes to the web API (routes, DTOs, error handling, etc.), always update `core/infrastructure/web_api/README.md` **and** the agent skill at `.claude/skills/lifestats/SKILL.md` to keep both in sync with the code. (The skill is symlinked into `~/.claude/skills/` so Claude Code can use the API from outside the repo.)

### Adding Anki Metrics

See `logger_applications/AnkiLogger/INTEGRATION.md` for the Anki bundle architecture and step-by-step instructions (both Rust backend and Python add-on changes).

### Adding a New Statistic Type

1. Add type under `core/core/src/statistic/types/{logable,derived}_statistics/`
2. Implement `Logable` or `Computable`
3. Update `AnyStatistic` enum if needed
4. Add domain tests (pure business logic, no mocking)
5. Wire the variant through every layer. `docs/plans/timed-amounts-statistic/touchpoints.md` lists the files `TimedAmounts` touched; the compiler finds most of them, but not the router, bootstrap injection, docs, or the web UI's and SDK's type lists.

### Adding a New Use Case

1. Add command/query struct under `core/application/src/use_cases/{commands,queries}/`
2. Handler orchestrates domain types via repository/event ports — keep it thin
3. Domain logic stays in domain entities (pure methods)
4. Unit-test with mock ports (mockall generates these)

### Adding a New Logger Application

New binaries depend on `lifestats-bootstrap` + `lifestats-application`, call `build_services()` in `main`, then instantiate handlers on demand. Mirror an existing logger under `logger_applications/` (e.g. `stopwatch_logger`).

### Adding a New HTTP Route

1. Add handler under `core/infrastructure/web_api/src/routes/`
2. Register in `routes::create_router`
3. Add integration test; handler should instantiate command/query handlers on-demand
4. **Update `core/infrastructure/web_api/README.md`** in the API Routes Reference section, and `.claude/skills/lifestats/SKILL.md` if the route is relevant for interactive agent use
