# LifeStats

LifeStats is a personal observability platform for tracking daily habits and performance metrics. It is built in Rust using **Ports & Adapters (Hexagonal) Architecture** to ensure a clean separation between core business logic and external concerns (persistence, HTTP, plotting, etc.). This repository is currently mid-refactor (see `refactor` branch) and is geared toward developers, not yet end-users.

**Status:** Active development; architecture stabilization in progress.

---

## At a Glance

- Multi-channel logging: CLI, stopwatch, MIDI input, Fitbit sync, Anki add-on, HTTP API
- SQLite persistence with Diesel ORM and migrations
- HTTP API (Axum) serving statistic/goal CRUD and plotting endpoints
- Goal management: daily & default targets for various statistic types
- Derived statistics (e.g., sum multiple time-spans, compute time deltas)
- Event-driven updates (publish-subscribe for data changes)
- Plotting/visualization infrastructure (wallpaper, gallery—in progress)
- Session tracking with heartbeat mechanism
- Backward-compatible database reads (verified via compatibility tests)

For third-party applications integrating with a running `lifestats-web-api.service`,
see the [Web API Integration Guide](docs/web-api-integration.md).

---

## Repository Layout

```
lifestats/
├── core/
│   ├── core/                     # Domain model (zero dependencies)
│   ├── application/              # CQRS orchestration (commands, queries, heartbeat utilities)
│   ├── infrastructure/
│   │   ├── persistence/          # SQLite repository implementations + migrations
│   │   ├── plotting/             # Plot rendering (plotters crate)
│   │   └── web_api/              # HTTP server (Axum, port 3000)
│   └── plot-domain/              # Plot-specific domain types
├── client/                       # HTTP client (in progress)
├── events/
│   ├── event_impl/               # Event types & publisher trait
│   └── event_sender/             # CLI to emit synthetic events
├── logger_applications/          # Legacy loggers (excluded from workspace)
│   ├── AnkiLogger/               # Anki add-on (Python)
│   ├── cli_logger/               # Terminal logger (Rust, old)
│   ├── stopwatch_logger/         # Interactive stopwatch (Rust, old)
│   ├── fitbit_logger/            # Fitbit sync (Rust, old)
│   ├── midi_logger/              # MIDI input tracker (Rust, old)
│   ├── ebook-reader/             # E-reader integration
│   ├── mpv_logger/               # MPV video tracker
│   └── web-activity-time-tracker/# Browser activity tracker
├── ui/
│   ├── wallpaper/                # Desktop wallpaper updater
│   └── web_ui/                   # Browser-based logging/gallery
├── docs/                         # Architecture & design docs
├── test_data/                    # DB snapshots for compatibility tests
├── .cargo/config.toml            # Shared target dir, sccache, build profiles
├── justfile                      # Task shortcuts (just test, just test-db-compat)
└── Cargo.toml                    # Workspace root
```

**Workspace members** (actively maintained):

- `core/core` (domain)
- `core/application`
- `core/infrastructure/{persistence,plotting,web_api}`
- `core/plot-domain`
- `client`

**Excluded** (legacy or WIP): `logger_applications/*`, `events/*`, `ui/*`

---

## Prerequisites

- **Rust** 1.70+ (2021 edition): `rustup` recommended
- **sccache** (optional): configured in `.cargo/config.toml` for faster builds
- **SQLite**: bundled via `diesel` (no separate install required)
- **just** (optional): for running recipes in `justfile`

---

## Quick Start

### 1. Clone & Build

```bash
git clone <repo-url>
cd lifestats
cargo build
```

Release build:

```bash
cargo build --release
```

### 2. Configuration

Set the database URL (required for live usage; tests use in-memory DB):

```bash
export DATABASE_URL=/path/to/your/lifestats.db
```

Alternatively, create a `.env` file in the project root:

```env
DATABASE_URL=/path/to/your/lifestats.db
```

The HTTP server (`web_api`) will automatically run migrations on startup if `DATABASE_URL` is set.

**Environment Variables:**

| Variable        | Purpose                             | Default/Required                      |
| --------------- | ----------------------------------- | ------------------------------------- |
| `DATABASE_URL`  | Path to SQLite DB for live usage    | Required for live                     |
| `COMPAT_DB_URL` | Snapshot DB for compatibility tests | Required for compat test              |
| `RUST_LOG`      | Log level (tracing-subscriber)      | `info,web_api=debug,tower_http=debug` |

### 3. Run Tests

```bash
# All workspace tests
cargo test --workspace

# Or via just
just test

# Database backward compatibility test (requires test_data/test_snapshot.db)
just test-db-compat
```

### 4. Start HTTP API

```bash
cargo run -p lifestats-web-api
```

The server binds to `0.0.0.0:3000`.

**Available routes:**

- `GET /statistics` – list all statistics
- `POST /statistics/:id/log/:date` – log a value for a statistic on a given date
- `POST /statistics/:id/log_time_frame/:date` – log a time frame
- `GET /statistics/:id/plot/image` – retrieve plot image for a statistic (stateless on-the-fly generation)
- `GET /statistics/:id/plot/data` – retrieve plot data as JSON (stateless on-the-fly generation)
- `GET /plots/timeline/:date` – retrieve timeline plot for specific date
- `POST /integrations/provision` – provision integration (e.g., Anki bundles)

Example (requires a running server):

```bash
curl http://localhost:3000/statistics
```

---

## Development Workflows

### Linting & Formatting

```bash
cargo clippy             # Lint
cargo fmt                # Format
cargo check              # Quick compile check
```

### Running Specific Tests

```bash
# Domain tests only
cargo test -p lifestats-core

# Application tests
cargo test -p lifestats-application

# Persistence tests
cargo test -p lifestats-persistence
```

### Shared Target Directory

Builds use `/tmp/cargo-target-shared` (see `.cargo/config.toml`). To change:

```toml
[build]
target-dir = "./target"  # Use local target dir instead
```

If `sccache` is unavailable, remove or comment out:

```toml
rustc-wrapper = "sccache"
```

---

## Domain Model (Key Concepts)

The domain is defined in `core/core` and has **zero external dependencies**.

### Statistic Types

**Logable Statistics** (accept direct input):

- `TimeSpans` – tracks intervals (start/end times)
- `IntegerAmount` – numeric counts
- `Habit` – boolean occurrences
- `PointInTime` – specific timestamps

Each implements the `Logable` trait with pure methods like `calculate_new_log_value`.

**Derived Statistics** (computed from other statistics):

- `SummingStatistic` – aggregates multiple `TimeSpans`
- `TimeDifference` – delta between two `PointInTime` stats

Each implements the `Computable` trait with a pure `compute_value` method.

**You cannot log directly to a derived statistic.** This separation prevents logical errors at compile time.

### Goals

- **Daily goals:** targets for specific dates
- **Default goals:** fallback targets when no daily goal exists
- Each statistic type supports different goal metadata (e.g., "is higher better?", efficiency percentages)

### Events

When data or goals change, the system publishes events (e.g., `ValueLogged`, `GoalSet`). Subscribers (wallpaper updater, plot renderer) listen and react.

### Ports (Traits)

- `StatisticRepository` – load/save statistics and goals
- `PlotRenderer` – generate plot images
- `EventPublisher` – publish domain events

Infrastructure crates provide concrete implementations.

---

## Data & Persistence

- **Engine:** SQLite via Diesel ORM
- **Connection pooling:** configured with WAL mode in `core/infrastructure/persistence/src/pool.rs`
- **Migrations:** located in `core/infrastructure/persistence/migrations/`
- **Schema:** statistics, goals (daily & default), time frames, plots, integrations, tags

### Migrations

Migrations run automatically when starting `lifestats-web-api` with `DATABASE_URL` set. To run manually:

```bash
cd core/infrastructure/persistence
diesel migration run
```

### Backward Compatibility

The compatibility test (`just test-db-compat`) ensures new code can read older database snapshots. Requires a snapshot DB in `test_data/test_snapshot.db` (not committed; see `test_data/README.md`).

---

## Extensibility

### Adding a New Statistic Type

1. Add type to `core/core/src/statistic/types/{logable,derived}_statistics/`
2. Implement `Logable` or `Computable` trait
3. Update `AnyStatistic` enum if needed
4. Add domain tests

### Adding a New Use Case

1. Add a command or query slice under `core/application/src/use_cases/commands|queries`
2. Keep handlers thin: orchestrate domain types via repository/event ports
3. Add unit tests using the generated mock ports

### Adding a New Logger

1. Create a binary crate (or add to `logger_applications/`)
2. Build the infrastructure you need (`SqliteStatisticRepository`, event publisher, etc.)
3. Instantiate command/query handlers on demand (e.g., `LogValueCommandHandler`)
4. See `CLAUDE.md` lines 185-235 for detailed guidance

### Adding a New HTTP Route

1. Add handler function to `core/infrastructure/web_api/src/routes/`
2. Register route in `routes::create_router`
3. Add integration test

---

## Architecture

LifeStats follows **Hexagonal (Ports & Adapters) Architecture**:

- **Domain (`core/core`):** Pure business logic, zero external dependencies. Defines ports as traits.
- **Application (`core/application`):** Use case orchestration. Depends only on domain.
- **Infrastructure (`core/infrastructure/*`):** Concrete implementations of ports (repositories, renderers, publishers). Depends on domain & application.
- **Driving Adapters:** CLI tools, HTTP server, loggers—wire everything together via dependency injection.

**Critical constraint:** Dependencies always point inward toward the domain.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for detailed diagrams and layer descriptions.

---

## Troubleshooting

### Build Issues

- **Disk space:** The shared target directory (`/tmp/cargo-target-shared`) can grow large. Clear with `cargo clean` or adjust `target-dir` in `.cargo/config.toml`.
- **sccache errors:** If `sccache` is not installed, remove `rustc-wrapper = "sccache"` from `.cargo/config.toml`.

### Runtime Issues

- **`DATABASE_URL` not set:** Tests will pass (they use in-memory SQLite), but live usage requires setting this variable.
- **Migration errors:** Ensure migrations have run. The `web_api` binary runs them automatically on startup.
- **Port 3000 in use:** The HTTP server binds to `0.0.0.0:3000`. Change the address in `core/infrastructure/web_api/src/main.rs` if needed.

### Logger Applications

The `logger_applications/` directory contains legacy loggers (excluded from the workspace). These may use outdated APIs or have missing dependencies. Refer to individual logger README files (if present) or check `CLAUDE.md` for architectural guidance.

---

## Further Reading

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) – detailed architecture diagrams and layer descriptions
- [`docs/statistics.md`](docs/statistics.md) – statistic types and their behaviors
- [`docs/IDEAS.md`](docs/IDEAS.md) – design ideas and future explorations
