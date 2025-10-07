# Repository Guidelines

## Project Structure & Module Organization
LifeStats is a Rust workspace anchored in `Cargo.toml`. Core domain logic sits in `core/core`, application services in `core/application`, and adapters under `core/infrastructure/{persistence,plotting,web_api}`. Plot-specific types live in `core/plot-domain`. Client-facing crates (`client`, `ui/*`, `events/*`, `logger_applications/*`) consume services but many are legacy; focus new work inside `core/*`. Shared assets and design notes are under `docs/`, while SQLite snapshots for regression tests live in `test_data/`.

## Build, Test, and Development Commands
- `cargo build` – compile all workspace crates (Rust 2021).
- `cargo test --workspace` – run unit/integration tests across crates; preferred before any push.
- `just test` – shortcut for the same test sweep.
- `just test-db-compat` – exercises backward-compatibility suite; requires `test_data/test_snapshot.db`.
- `cargo fmt` / `cargo clippy --all-targets --all-features` – format and lint prior to review.

## Coding Style & Naming Conventions
Use Rust defaults: 4-space indentation, `snake_case` for modules/functions, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep modules small; domain traits belong under `core/core/src/.../ports` while concrete adapters stay in infrastructure crates. Run `cargo fmt --check` in CI locales and fix clippy warnings locally. When authoring HTTP handlers, mirror existing Axum route naming (`*_route.rs`) and re-export via `mod.rs`.

## Testing Guidelines
Write focused unit tests alongside source files using `#[cfg(test)]` modules. Integration tests belong in each crate’s `tests/` directory. Name tests after observable behavior (`logs_event_on_goal_change`) to match existing patterns. For database changes, add coverage to `core/infrastructure/persistence/tests` and run `just test-db-compat`. Capture expected event flows with mocks when touching `core/application`. Document new test data in `test_data/README.md`.

## Commit & Pull Request Guidelines
Commits in this repo are short, imperative summaries (e.g., `Refactor derived stats to use id`). Limit scope to one concern and include follow-up notes in the body if needed. PRs should describe motivation, outline architectural touchpoints, and reference issues or TODO items. Always list the commands you ran, attach screenshots for UI-facing changes (`ui/web_ui`), and call out migration impacts or new environment variables.

## Configuration & Environment
`.cargo/config.toml` pins a shared `target-dir` and optional `sccache`; adjust locally rather than deleting. Set `DATABASE_URL` when exercising live persistence and `COMPAT_DB_URL` for compatibility tests. Avoid committing `.env` files; document secrets in `docs/` or mention them inline in PR discussion.
