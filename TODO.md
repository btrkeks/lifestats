# ✓ COMPLETE: Refactor Stopwatch & Heartbeat Architecture

**Status:** Implementation complete on `refactor` branch
**All tests passing:** 36/36 tests pass (including 13 new HeartbeatSession tests)

## Goal

Migrate the active session tracking logic from the `heart_beat_logger` infrastructure crate into the `lifestats-application` core crate. Convert the `stopwatch_logger` into a "thin" UI Adapter that drives this new core application service.

## Architectural Principles Applied

1.  **Single Responsibility Principle:** Core business logic (tracking time sessions) is separated from UI concerns (TUI rendering) and persistence details (HTTP vs Local).
2.  **Dependency Inversion:** The new session service will depend on the `StatisticRepository` **Port** (abstract trait), not on concrete logger implementations.

---

## Phase 1: Application Layer Refactoring ✓

**Objective:** Establish the `HeartbeatSession` service within the Application Layer (`lifestats-application`), removing dependencies on concrete logging mechanisms.

### 1. Create the Service Module ✓

- [x] Create a new file: `core/application/src/use_cases/session_tracking_service.rs`.
- [x] Register the module in `core/application/src/use_cases/mod.rs`.

### 2. Port Configuration & Data Types ✓

**Context:** Move the pure data structures that define a session's state and configuration.
**Source:** `logger_applications/heart_beat_logger/src/lib.rs`
**Destination:** The new `session_tracking_service.rs`.

- [x] Move `HeartbeatConfig` struct.
- [x] Move `HeartbeatUpdate` enum.
- [x] Move internal types `HeartMessage` enum and `TimeoutAction` enum.
- [x] Move `TotalDurationManager` struct and its implementation.

### 3. Port & Refactor Core Logic (`HeartTiming` & `Heart`) ✓

**Context:** Move the complex timing logic and apply **Dependency Inversion** to the worker thread.

- [x] **Move `HeartTiming`:** Copy the `HeartTiming` struct and `impl` block into `session_tracking_service.rs`. Ensure it retains the generic dependency on `TP: TimeProvider`.
- [x] **Refactor `Heart` (Apply Dependency Inversion):**
  - [x] Copy the `Heart` struct.
  - [x] **CRITICAL:** Remove the `TL: TimeFrameLogger` generic and field.
  - [x] Add fields for `statistic_id: StatisticId` and `repository: Arc<R>` where `R: StatisticRepository + Send + Sync`.
- [x] **Implement Domain Logging in `Heart::main_loop`:** Replace all calls to `self.logger.log(start, end)` with direct use of the `StatisticRepository` port.
  - [x] _Guidance:_ Create a private helper method in `Heart`: `fn persist_time_frame(&self, start: NaiveTime, end: NaiveTime) -> Result<()>`.
  - [x] _Implementation:_ Inside this helper:
    1. Fetch current value using `self.repository.find_log_value(...)`.
    2. Construct the new `TimeFrame` and `TimeFrameList` domain objects.
    3. Apply `TimeFrameListUpdatePolicy::UpdateEndValue` logic (similar to `LoggingService::log_time_frame`, but implemented specifically for this worker to avoid circular dependencies with `LoggingService`).
    4. Save the result using `self.repository.save_log_value(...)`.

### 4. Create the Public Application Service (`HeartbeatSession`) ✓

**Context:** This is the public face of the service that the UI adapters will interact with.

- [x] Port `HeartbeatLogger` from `heart_beat_logger/src/lib.rs` to `session_tracking_service.rs` and rename it to `HeartbeatSession`.
- [x] Refactor its `new` constructor:
  - [x] Remove `TL: TimeFrameLogger` generic.
  - [x] Accept `statistic_id: StatisticId` and `repository: Arc<R: StatisticRepository + Send + Sync>`.
- [x] Update `spawn_heart_thread` to initialize the refactored `Heart` worker with the `statistic_id` and `repository`.

### 5. Add Testing ✓

- [x] Tests integrated into `session_tracking_service.rs` (13 tests).
- [x] Implemented tests using `MockStatisticRepository` and `MockTimeProvider`. Verified that `beat()`, `stop()`, and timeouts correctly result in calls to `repository.save_log_value` with the expected `TimeFrameList` data.

---

## Phase 2: Adapter Layer Refactoring (`StopwatchLogger`) ✓

**Objective:** Convert the `stopwatch_logger` binary into a "dumb" UI adapter that depends on the new Application Layer service.

### 1. Update Dependencies ✓

- [x] In `logger_applications/stopwatch_logger/Cargo.toml`:
  - [x] Remove dependency on `heart_beat_logger`.
  - [x] Ensure dependency on `lifestats-application` is present and correct.

### 2. Refactor `main.rs` ✓

**Context:** Wire the UI directly to the new application service.

- [x] Remove `mod stopwatch;` and `use crate::stopwatch::...`.
- [x] Import `lifestats_application::use_cases::HeartbeatSession` (and related config/update types).
- [x] In `run_stopwatch`:
  - [x] Replace `Stopwatch::new(...)` with `HeartbeatSession::new(...)`.
  - [x] Pass `metadata.id()`, configuration, and `app_context.statistic_repository` to the constructor.
  - [x] Update the event loop to handle the imported `HeartbeatUpdate` variants.

---

## Phase 3: Cleanup ✓

**Objective:** Remove the now-obsolete infrastructure code.

- [x] Delete `logger_applications/stopwatch_logger/src/stopwatch.rs`.
- [x] Delete the entire directory `logger_applications/heart_beat_logger/`.
- [x] `heart_beat_logger` was not in workspace members (legacy code).
- [x] Run `cargo build` and `cargo test` from the workspace root - **ALL TESTS PASS** (36/36)
