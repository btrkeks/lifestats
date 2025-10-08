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

## Architecture: Ports & Adapters (Hexagonal)

The codebase strictly separates business logic from external concerns. **Dependencies always point inward toward the domain.**

### Workspace Structure

```
core/
├── core/                # Pure business logic, zero external dependencies (domain layer)
│   └── src/
│       ├── statistic/
│       │   ├── types/          # SourceStatistic & DerivedStatistic types
│       │   │   ├── logable_statistics/
│       │   │   └── derived_statistics/
│       │   ├── ports/          # Trait definitions (e.g., StatisticRepository)
│       │   ├── traits/         # Logable, Computable
│       │   ├── goals/
│       │   ├── update_policies/
│       │   └── value_objects/
│       └── shared/
│
├── application/         # Use case orchestration layer (CQRS pattern)
│   └── src/
│       ├── use_cases/
│       │   ├── commands/       # Command handlers (write operations)
│       │   │   ├── create_statistic/
│       │   │   ├── log_value/
│       │   │   ├── set_daily_goal/
│       │   │   ├── set_default_goal/
│       │   │   ├── delete_statistic/
│       │   │   ├── delete_daily_goal/
│       │   │   └── delete_default_goal/
│       │   ├── queries/        # Query handlers (read operations)
│       │   │   ├── get_all_statistics/
│       │   │   ├── get_statistic_by_id/
│       │   │   ├── get_metadata/
│       │   │   ├── get_all_metadata/
│       │   │   ├── get_plot_data/
│       │   │   ├── get_streak/
│       │   │   ├── get_daily_goal/
│       │   │   ├── get_default_goal/
│       │   │   └── get_time_spans_metadata/
│       │   └── integration/
│       ├── integrations/
│       │   └── fitbit/
│       ├── repositories/
│       ├── dto/
│       ├── statistic/
│       └── shared/
│
└── infrastructure/      # Implementations of domain ports (Driven Adapters)
    ├── persistence/     # SQLite implementations of repository traits
    ├── web_api/         # HTTP API adapter (Axum-based REST API)
    │   └── src/
    │       ├── routes/
    │       ├── dto/
    │       └── integrations/
    │           └── fitbit/
    ├── fitbit_client/   # Fitbit API client
    ├── plotting/        # Plot generation adapter
    └── system_clock/    # System clock adapter

events/                  # Event infrastructure
├── event_impl/
└── event_sender/

logger_applications/     # Various logging applications
├── cli_logger/
├── fitbit_logger/
├── midi_logger/
├── stopwatch_logger/
├── mpv_logger/
├── AnkiLogger/
├── ebook-reader/
└── web-activity-time-tracker/

ui/                      # User interface components
├── wallpaper/           # Desktop wallpaper display
└── web-ui/              # Web-based UI

lifestats-bootstrap/     # Application bootstrap and wiring

client/                  # HTTP client library
```

### Critical Architectural Constraints

1. **`core/core` (domain layer) has ZERO dependencies** on other workspace crates. It defines all ports as traits.
2. **`core/application` depends only on `core/core` (domain)**. It coordinates domain logic but contains no business rules.
3. **`core/infrastructure/*` implements domain ports**. The domain never depends on infrastructure.
4. **Driving adapters** (CLI tools, web servers, loggers in `logger_applications/`, `lifestats-bootstrap`) wire everything together by injecting concrete implementations into application services.

## Development Guidelines

### Handling Uncertainty with External Dependencies

**When uncertain about correct usage of a library, API, or framework:**

1. **PAUSE** - Do not make assumptions or attempt to brute-force a solution
2. **ASK** - Explicitly request clarification, documentation, or additional context from the user
3. **VERIFY** - Use official documentation, examples, or the `apilookup` tool to confirm correct usage

This applies to:
- Unfamiliar Rust crates and their APIs
- HTTP client/server frameworks
- Database query builders
- Testing frameworks
- Any external dependency where the correct usage pattern is unclear
