# lifestats

LifeStats is a personal observability platform for daily habits and wellness metrics. It collects data from many touchpoints, keeps a consistent history, and produces ambient visualisations that make trends easy to spot at a glance. The codebase is geared toward developers (including automation tooling and LLM agents) who want a clear picture of how the system hangs together.

## Why It Exists

- Track recurring activities such as sleep, practice sessions, study time, or workouts.
- Capture events the moment they happen—manually, via sensors, or by syncing third-party data.
- Surface engaging feedback through live dashboards, desktop wallpapers, and simple scores.
- Experiment with routines by adding new statistics without rewriting core logic.

## Core Capabilities

- Multi-channel logging (CLI, web UI, HTTP API, stopwatch, heartbeat feeds, MIDI input, Fitbit sync, Anki add-on).
- Goal management with custom targets per statistic and day.
- Automated plot generation with a gallery UI and wallpaper updater for multi-monitor setups.
- Event-driven updates: whenever new data or goals arrive, subscribers refresh themselves.
- Extensible Rust workspace with focused crates for logging, persistence, plotting, utilities, and integration glue.

## Logging Interfaces

The system deliberately offers multiple ways to capture data so that logging can happen wherever the user already is.

- **CLI logger (`logging/cli_logger`)** – quick terminal logging for any statistic, including date overrides and update-policy hints.
- **Web UI (`interfaces/web/server`)** – a browser-based form for logging values, plus goal management pages and an API under `/api`.
- **HTTP client crate (`interfaces/web/api`)** – reusable API client for other services or scripts that want to push data remotely.
- **Stopwatch logger (`logging/stopwatch_logger`)** – interactive terminal UI that toggles play/pause, surfaces live durations, and writes directly to statistics.
- **Heartbeat logger (`logging/heart_beat_logger`)** – a generic engine that converts periodic “beats” into time-span logs, with pluggable backends (local DB or HTTP).
- **MIDI logger (`logging/midi_logger`)** – connects an instrument to the heartbeat logger to track practice sessions automatically.
- **Fitbit sync (`logging/third_party/fitbit`)** – pulls recent Fitbit metrics (sleep duration, wake times, steps, etc.) and maps them into statistics.
- **Anki add-on (`AnkiLogger`)** – monitors study sessions inside Anki, logging focused time via the heartbeat mechanism.
- **Event sender (`interfaces/event_sender`)** – emits synthetic events such as “New Day” to force recalculations or wallpaper refreshes.
  Every channel converges on the same domain model, so once data lands it is treated uniformly regardless of its origin.

## Plots, Scores, and Feedback Loops

- **Wallpaper service (`interfaces/wallpaper`)** – renders plot images for one or more monitors, keeps them up to date, and sets them as wallpapers. Plots re-render when events arrive (goal changes, new values, or daily rollovers).
- **Web gallery** – the root page of the web server serves a masonry grid of the latest plots for a quick overview.
- **Scoring CLI (`interfaces/cli`)** – calculates per-statistic scores and aggregated totals for a chosen day, useful for quick retrospectives.

## Goals & Habit Tracking

Goals live alongside statistics. The goal editor in the web interface supports:

- Setting daily or default targets for different statistic types (numbers, durations, time spans, habits, point-in-time targets).
- Supplying extra metadata (e.g., “is higher better?”, efficiency percentages) depending on the statistic type.
- Deleting daily or default goals with instant feedback.
  When goals change, the event bus notifies subscribers so wallpapers or other clients can recalculate progress indicators.

## Data & Persistence

- SQLite is the default backing store. `persistence/pool.rs` configures pooled connections, WAL mode, and foreign-key enforcement.
- Environment variable `DATABASE_URL` points to the live database. Tests use in-memory SQLite.
- Repositories translate domain objects to DTOs and vice versa, keeping storage concerns outside of the core.

## Roadmap & Vision

- **Refactor branch** – bring the ongoing refactor up to production quality.
- **Unified logging transport** – converge all loggers on the HTTP API so they behave like micro-services rather than direct DB writers.
- **Web UI refresh** – modernise the browser experience for logging, reviewing history, and managing goals.
- Continued exploration of architectural refinements to keep the onion layers clean while adding new integrations.
