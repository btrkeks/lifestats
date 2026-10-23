# Consumers, contracts, and mobile readiness

Read-only architecture exploration, 2026-10-22. Scope: `/home/daniel/.local/share/src/lifestats`, including nested LiftLog. All citations are relative to that root; line numbers refer to the inspected working files. No repository files, DBs, services, Git state, or external state were changed. Only this report was created. No delegates or external messages were used.

## Overview

**Implementation:** Lifestats already has an HTTP application boundary, but it is not yet one dependable contract for all consumers. Web UI, the shared JS client, Python, extensions, Android apps, ebook reader, and LiftLog maintain independent request/response knowledge. Rust MIDI and stopwatch instead instantiate the SQLite-backed server composition root inside their own processes. These are materially different deployment and delivery models.

The highest-value preparation for a new React Native frontend is to make the existing HTTP boundary authoritative and safe, explicitly distinguish observations/events from replaceable snapshots, and define identity/time/order rules. Replacing the domain wholesale is not required by this slice. There are useful existing abstractions and proven client queue patterns to retain.

**Intent versus implementation:** Root `AGENTS.md` describes hexagonal/CQRS boundaries and recommends bootstrap for new Rust loggers. That is documentation of intended layering, not proof of remote-client suitability. The running application assembled by `lifestats-bootstrap/src/lib.rs:65` always opens the local SQLite pool, builds rendering, integration and Fitbit components, and constructs an in-process session manager. `core/infrastructure/persistence/src/pool.rs:14` requires `DATABASE_URL`; pool creation runs embedded migrations at `:49`. The MIDI logger is an active Cargo workspace member (`Cargo.toml:13`), so this coupling is not confined to abandoned code.

## Key concepts

- **Statistic identity and type:** API metadata exposes `id`, `name`, `category`, `statistic_type`, and nullable `default_goal` (`core/infrastructure/web_api/src/dto/statistic.rs:56`). It omits source IDs, tags, units, revisions, and provenance. Creation separately accepts source IDs (`:8`). IntegerAmount means an integer with caller-defined units: extension sends seconds, screen-time and Clef send minutes. Clients cannot recover those units from this DTO.
- **Logging command:** HTTP value is a string and policy is an optional string (`core/infrastructure/web_api/src/dto/statistic.rs:29`). Adapter looks up statistic type and parses it into a typed command (`core/infrastructure/web_api/src/routes/statistics.rs:138`). Policies differ by type; there is no universal policy enum matching the SDK's enum.
- **Goal override is tri-state:** No override, explicit suppression, or concrete override; deleting a suppression restores default applicability. Server response enum is `not_set | suppressed | goal{goal_data}` (`core/infrastructure/web_api/src/dto/goal.rs:49`). An unset default is `200 null`, not a missing-resource response (`core/infrastructure/web_api/src/routes/goals.rs:41`, `:60`).
- **Activity sessions versus workout sessions:** Lifestats heartbeat sessions are server-process objects keyed by statistic ID (`core/application/src/use_cases/session_manager.rs:48`). LiftLog sessions are persistent on-device workouts, with sets/exercises and a finished/active lifecycle. They are not the same entity and should not be unified merely because both are named session.
- **Event versus snapshot delivery:** Ebook submits UUID events for deduplication. LiftLog replaces whole named days. Screen-time/Clef overwrite daily totals. Anki and extension sometimes send additive deltas. A retry has different meanings in these protocols.

## Where things live and contract sources

| Concern | Actual source and boundary |
|---|---|
| Route inventory | `core/infrastructure/web_api/src/routes/mod.rs:29` registers routes directly; no `/v1` prefix or generated API description found in the inspected core/client/web-UI trees. |
| Statistic/log/goal wire DTOs | `core/infrastructure/web_api/src/dto/statistic.rs:8`, `core/infrastructure/web_api/src/dto/goal.rs:15`; values and goal fields cross as strings. |
| Integration DTOs | Defined directly in route module, not the DTO directory: `core/infrastructure/web_api/src/routes/integrations.rs:19`, `:35`, `:54`. |
| Progress DTOs | `core/infrastructure/web_api/src/dto/progress.rs:10`; checkpoints include effective date, recorded timestamp, source and source-event ID (`:29`). This is a useful richer existing model. |
| Some response DTOs | Application types leak directly onto HTTP: logged-value response (`core/infrastructure/web_api/src/routes/statistics.rs:882`), dashboard (`core/infrastructure/web_api/src/routes/dashboard.rs:27`), daily summary (`core/infrastructure/web_api/src/routes/daily_summary.rs:24`); these are additional contract sources outside web API DTOs. |
| Error mapping | `core/infrastructure/web_api/src/error.rs:27`, plus per-handler string wrapping and Axum extractor rejections. |
| Shared JS package | Independently authored interfaces (`clients/lifestats-js/src/types.ts:3`) and Zod schemas (`clients/lifestats-js/src/schemas.ts:12`); no generation/check against Rust found. Package declares Zod 3 (`clients/lifestats-js/package.json:34`). |
| Web UI contract | Own interfaces (`ui/web-ui/src/types/index.ts:1`), Axios layer (`ui/web-ui/src/lib/api.ts:19`), and goal codecs/form schemas (`ui/web-ui/src/lib/schemas.ts:23`, `:96`, `:187`). It does not import the shared SDK. |
| Human/agent references | `core/infrastructure/web_api/README.md` and `.claude/skills/lifestats/SKILL.md`. These already disagree with code on some errors/session behavior. |
| LiftLog schemas | LiftLog `logger_applications/LiftLog/docs/index.md:40` describes generated local model/worker schemas; these are not schemas for the Rust Lifestats API. Its sync wire type is handwritten at `logger_applications/LiftLog/app/src/services/lifestats-sync.ts:8`. |

### Routes available to a mobile UI

- Statistics: list/create, loggable list, get/delete, string log, time-frame log, one day's logged value; `core/infrastructure/web_api/src/routes/mod.rs:31` through `:51`.
- Analysis: range totals, consistency, score, achievements, performance, streak; `core/infrastructure/web_api/src/routes/mod.rs:52` through `:76`. Dashboard aggregates multiple statistics (`:100`).
- Default and daily goals: get/put/delete (`core/infrastructure/web_api/src/routes/mod.rs:102`).
- Progress items: create/list/get, target patch, checkpoints, summary, daily deltas (`core/infrastructure/web_api/src/routes/mod.rs:78`).
- Plots: image/data/timeline (`core/infrastructure/web_api/src/routes/mod.rs:115`); daily summary (`:174`).
- Heartbeat sessions: list/start/beat/stop/status (`core/infrastructure/web_api/src/routes/mod.rs:129`).
- Integration provisioning, ebook events, LiftLog days, Fitbit OAuth and sync (`core/infrastructure/web_api/src/routes/mod.rs:147`).

**Observed omissions relevant to design:** The complete router has no metadata update registration, generic log deletion, change-feed/cursor, sync conflict endpoint, or client/device registration. `UpdateStatisticRequest` exists but is explicitly dead code (`core/infrastructure/web_api/src/dto/statistic.rs:20`). Bulk UI refresh and offline cache reconciliation need a deliberate contract; presence of lots of read endpoints does not provide synchronization semantics.

## How it works

### 1. Ordinary HTTP logging

`POST /statistics/{id}/log/{date}` → Axum `Path<(u32, NaiveDate)>` and `Json<LogValueRequest>` (`core/infrastructure/web_api/src/routes/statistics.rs:109`) → validator (`:133`) → repository metadata lookup (`:139`) → parse according to actual statistic type (`:155`) → typed `LogValueCommand` (`:266`) → `LogValueCommandHandler::handle` (`core/application/src/use_cases/commands/log_value/handler.rs:45`) → read current JSON (`:56`) → deserialize current value → pure `calculate_new_log_value` (`:73`, `:140`) → save if changed and publish `ValueUpdated` (`:165`) → 204 (`core/infrastructure/web_api/src/routes/statistics.rs:283`).

Defaults: IntegerAmount sums (`core/infrastructure/web_api/src/routes/statistics.rs:173`); Habit/PointInTime overwrite (`:197`, `:228`); TimeSpans appends (`:251`). Derived statistics reject writes with 400 (`:258`). This is already a good adapter-to-typed-command shape to retain.

Failure behavior differs by stage: missing statistic lookup becomes 404, bad primitive/policy becomes 400, DTO validator becomes 422 through `ApiError` conversion, handler errors are flattened into 500 by `core/infrastructure/web_api/src/routes/statistics.rs:280`. Panics from policies do not pass through that Result mapping. Native Axum extraction errors are not consistently converted to `{error:...}`.

Dedicated `log_time_frame` parses and constructs one validated `TimeFrame` (`core/infrastructure/web_api/src/routes/statistics.rs:313`, `:358`), unlike raw list logging. It shares the same command handler and default append policy (`:330`, `:382`).

### 2. Goal editing in web UI

`useSetDailyGoal` → `ui/web-ui/src/lib/api.ts:91` → PUT with discriminated request → `core/infrastructure/web_api/src/routes/goals.rs:277` → load metadata (`:288`) → parse string-map into typed `GoalPayload` or suppression (`:293`, `:320`) → command handler (`:308`) → 204. React Query invalidates default/daily goal, achievements and metadata caches after success (`ui/web-ui/src/features/goals/hooks/useGoals.ts:27`, `:79`).

On reads, response goal shapes differ from request shapes: integer `target_amount` becomes form `target`, Habit `target` becomes `occurred`, PointInTime `target` becomes `deadline` (`ui/web-ui/src/lib/schemas.ts:96`). This translation is protocol knowledge duplicated in UI, not a recalculation of goal achievement. Actual achievement is server-provided; `classifyDay` only maps it to display state (`ui/web-ui/src/features/goals/dayStatus.ts:14`).

### 3. Rust logger and heartbeat HTTP consumers

MIDI `main` → `build_services` (`logger_applications/midi_logger/src/main.rs:26`) → query local repository metadata (`:46`) → construct its own `HeartbeatSession` with repository (`:58`) → hardware events call `beat` (`:89`) → worker reads and writes repository directly (`core/application/src/use_cases/session_tracking_service.rs:478`). It does not use HTTP, server `SessionManager`, or `LogValueCommandHandler`. Stop joins worker (`core/application/src/use_cases/session_tracking_service.rs:298`), but worker `heart.run(receiver)` result is discarded (`:356`). Persisting failures can stop the worker without a failure returned by logger `beat`/stop APIs.

Stopwatch similarly bootstraps and receives `app.statistic_repository` (`logger_applications/stopwatch/stopwatch_logger/src/main.rs:169`, `:175`). Its manifest's core/bootstrap paths are only two parents up despite the extra `stopwatch/` directory (`logger_applications/stopwatch/stopwatch_logger/Cargo.toml:8`); resolve to nonexistent sibling paths. It is not in root active workspace. Treat as legacy/WIP rather than the primary new-logger template without verification.

MPV uses HTTP `/sessions/{statistic_id}/start`, `/beat`, `/stop` with manually configured ID (`logger_applications/mpv_logger/mpv_logger.lua:64`, `:79`, `:94`). Start succeeds only at 201; failure leaves local inactive flag, heartbeat requires that flag (`:67`, `:77`). HTTP `start` creates a dormant session object; first beat starts worker (`core/application/src/use_cases/session_tracking_service.rs:267`, `:291`). `/beat` requires an existing map entry (`core/application/src/use_cases/session_manager.rs:126`), contrary to docs saying it creates a session. HTTP stop removes the map entry (`core/application/src/use_cases/session_manager.rs:145`), so subsequent status is 404.

### 4. Python Anki and browser extension

Anki provisions by stable deck ID and deck name through `/integrations/anki` (`logger_applications/AnkiLogger/api_client.py:101`); validates five expected response keys (`:116`). Its own timer/session state machine owns idle timeout/flush (`logger_applications/AnkiLogger/session_tracker.py:58`, `:436`). It sends time with `updateendvalue` (`:240`), disruption increments with `sum` (`:264`), backlog/cards as `overwrite` (`:290`, `:374`), finish time with `ignore` (`:405`). The finish-time docstring says overwrite/last finish while implementation preserves first (`:383`).

API calls run in executor jobs that catch/log HTTP failures, with no durable retry state in these paths (`logger_applications/AnkiLogger/api_client.py:245`, `:266`). Thus these jobs are best-effort and are not a mobile offline pattern. UI notification of success can precede actual delivery (`logger_applications/AnkiLogger/session_tracker.py:411`).

Extension `Tab.incSummaryTime` → increments global interval counter → resets after 20 seconds → POST constant statistic ID 16, local date, string seconds, `sum` (`logger_applications/web-activity-time-tracker/src/entity/tab.ts:16`, `:40`, `:49`). Failure is caught/logged after the counter resets (`:35`, `:44`); measured time is not queued for redelivery. This is per-installation ID coupling and best-effort delivery despite independent local per-tab/day totals.

### 5. Ebook event flow

Reader builds event with `crypto.randomUUID` and book data (`logger_applications/ebook-reader/apps/web/src/lib/data/lifestats/lifestats-sync-adapter.ts:39`) → atomically persist tracker state and queue entry through `commitEbookSyncDelta` (`:66`) → provision ebook bundle (`:78`) → POST queued event (`:112`) → route constructs application request (`core/infrastructure/web_api/src/routes/integrations.rs:196`) → `IntegrationService::ingest_ebook_event` validates (`core/application/src/use_cases/integration/service.rs:251`) → event store `record_event` (`:331`) → ensure/log progress checkpoint (`:333`) → `{accepted,duplicate}` (`core/infrastructure/web_api/src/routes/integrations.rs:245`).

Client deletes queue on any successful HTTP status without validating accepted/duplicate body (`logger_applications/ebook-reader/apps/web/src/lib/data/lifestats/lifestats-sync-adapter.ts:129`). Failures retain event ID and schedule backoff (`:146`, `:165`). All error statuses retry alike. Server retry can repair checkpoint work after event already recorded: checkpoint call still occurs when record_event reports duplicate (`core/application/src/use_cases/integration/service.rs:331`). Preserve stable event identity and the distinction between event acceptance and completing downstream progress work.

### 6. LiftLog offline day replacement

Session finish/edit/delete/import → `queueDaysChangedBy` in same SQLite transaction as write (`logger_applications/LiftLog/app/src/store/stored-sessions/effects.ts:139`, `:163`, `:173`, `:188`) → queue dates before and after write (`logger_applications/LiftLog/app/src/db/lifestats-sync.ts:31`) → Redux wake → debounce/single in-flight push/backoff (`logger_applications/LiftLog/app/src/store/lifestats-sync/effects.ts:30`) → `pushQueuedDays` reads queue and current finished workouts (`logger_applications/LiftLog/app/src/services/lifestats-sync.ts:75`) → `sessionSpan` takes earliest/latest completed set, cardio duration included (`:31`) → batch PUT `/integrations/liftlog/days` (`:84`).

Server validates whole request, RFC3339 timestamps and bounds (`core/infrastructure/web_api/src/routes/integrations.rs:80`, `:86`), preserves device wall-clock timestamps via `naive_local` (`:127`) → integration provisions workout statistic (`core/application/src/use_cases/integration/service.rs:353`) → clamps/merges intervals within attributed date, minimum one minute (`:134`) → replaces/clears each specified day (`:366`) → counters response (`core/infrastructure/web_api/src/routes/integrations.rs:303`). Workouts' wire IDs are not carried into application `LiftLogWorkout` (`core/infrastructure/web_api/src/routes/integrations.rs:127`); server stores only daily frames, not workout identities.

Phone timeout is 15 seconds; network/408/429/5xx are retryable, other failures remain queued without timed retry (`logger_applications/LiftLog/app/src/services/lifestats-sync.ts:101`, `:111`; effects `:67`). A 2xx must at least contain numeric `statistic_id` (`logger_applications/LiftLog/app/src/services/lifestats-sync.ts:137`). Acknowledgement deletes only matching `(date, seq)` so edits during a send survive (`logger_applications/LiftLog/app/src/db/lifestats-sync.ts:51`). Retries run in foreground, not background (`logger_applications/LiftLog/app/src/store/lifestats-sync/effects.ts:92`; docs `logger_applications/LiftLog/docs/LifestatsSync.md:30`). Server day writes are sequential without a batch transaction (`core/application/src/use_cases/integration/service.rs:366`), so a partial server failure is repaired by whole-batch resend, provided replacement remains idempotent.

### 7. Other concrete snapshot consumers

Android screen-time posts today's total and optional goal-filtered minutes with `overwrite` (`logger_applications/screen_time_logger/app/src/main/java/dev/lifestats/screentimelogger/LifestatsApiClient.kt:14`, `:36`). Worker retries 408/429/5xx/network errors (`logger_applications/screen_time_logger/app/src/main/java/dev/lifestats/screentimelogger/ScreenTimeSyncWorker.kt:45`) but recomputes `readToday()` on each attempt (`:17`): a retry after midnight does not necessarily resend yesterday. This differs from a persisted dated observation.

Watch bedtime writes local state before WorkManager (`logger_applications/bedtime_watch/app/src/main/java/dev/lifestats/bedtime/BedtimeLogger.kt:16`), uses per-evening unique replace work (`:32`), POSTs overwrite and retries (`logger_applications/bedtime_watch/app/src/main/java/dev/lifestats/bedtime/UploadBedtimeWorker.kt:24`). Date attribution before noon belongs to previous evening (`logger_applications/bedtime_watch/app/src/main/java/dev/lifestats/bedtime/Bedtime.kt:15`). That rule belongs to product/source semantics, not generic timezone conversion.

Clef sync validates target statistic type (`logger_applications/clef_sync/clef-lifestats-sync:69`), validates a separately versioned upstream export (`:93`), and posts dated minutes with overwrite (`:114`). No direct Lifestats integration references were found in OpenNutriTracker's `lib/`; its AGENTS describes encrypted local Hive and Supabase food data (`logger_applications/OpenNutriTracker/AGENTS.md:7`). That is a negative source search, not proof of no runtime/external integration. Yomikata provisioning is wired and tested, but no in-repo Yomikata app caller was identified (`core/infrastructure/web_api/tests/yomikata_bundle_e2e.rs:56`).

## Client-owned state versus server product state

| Owner | Actual responsibility | Boundary to preserve/design |
|---|---|---|
| Web UI | Query cache; form drafts, selection and display labels | Server owns goal applicability/achievement. Cache invalidation is already explicit; do not copy evaluation into RN. React Query cache is in memory with 60s stale time (`ui/web-ui/src/lib/providers.tsx:6`). No durable mutation queue configured. |
| Lifestats server | Statistics, daily values/goals, derived/analysis results, progress items/checkpoints, integration provisioning | Metadata/read DTOs need a stable public schema; current server internals also serve as wire types. |
| Server process | Heartbeat session objects and clocks | Not durable multi-device workout state. A server restart loses map state; HTTP clients must re-establish session. |
| LiftLog | Exercises/plans/weights/sets, ongoing/finished workouts, local SQLite, settings, dirty-day queue | Sends only daily workout time. Server never receives full workout model. Local DB open is visible in `logger_applications/LiftLog/app/src/components/smart/services-provider.tsx:13`; finished sessions read in `logger_applications/LiftLog/app/src/db/sessions.ts:419`. |
| Ebook reader | Source reading-position confirmation/jump detection state and durable event queue | Server owns applied character totals/progress; retain source position logic on reader, event identity across retries. |
| Sensor/source loggers | Source-specific observations, device permissions, activity qualification, bedtime day attribution | Shared ingestion and retry semantics need consistency, but source detection should not be pulled into generic domain UI code. |

LiftLog's full local model is a bounded product, not duplicated Lifestats domain. Its workout-span calculation and server normalization deliberately sit on opposite sides: phone knows completed sets; server knows how those spans fit a Lifestats day. The avoidable duplication is handwritten wire types/codecs, generic session timing, error classification and delivery machinery—not the mere existence of rich client models.

## Prioritized findings

### P1 — Shared SDK is currently an unsafe foundation for RN

**Evidence:** `clients/lifestats-js/src/types.ts:18` and `clients/lifestats-js/src/schemas.ts:21` allow sum/overwrite/max/min, but server enums allow sum/overwrite/ignore for amounts, ignore/overwrite for fixed values, append/updateendvalue/overwrite for frames (`core/core/src/statistic/value_objects/update_policies/amount.rs:7`, `core/core/src/statistic/value_objects/update_policies/fixed_value.rs:6`, `core/core/src/shared/time_frame_list.rs:7`). `getDailyGoal` models a nullable `GoalResponse` (`clients/lifestats-js/src/client.ts:227`), not the server tri-state. `getDefaultGoal` handles 404 absence (`:181`) while server returns null. Metadata schema lacks `default_goal` (`clients/lifestats-js/src/schemas.ts:23`). TimeSpans goal builder makes efficiency optional (`clients/lifestats-js/src/schemas.ts:113`) while server requires it (`core/infrastructure/web_api/src/routes/goals.rs:165`).

**Verified cheaply:** Transpiled actual SDK sources in memory with installed Zod v3 compatibility export (SDK's own node_modules absent), synthetic fetch, no HTTP/files. Valid `updateendvalue` fails SDK schema; unsupported `max` passes. Metadata parsing drops `default_goal`. Suppressed daily response becomes `{}` because `z.unknown()` accepts absent goal_data in Zod v3 and object parsing strips kind. Unset default throws ZodError. This is not just type staleness.

**Consequence/design:** A new RN consumer inherits silent information loss and rejected valid writes. Define one public wire contract and generated/schema-derived SDK, migrate web UI onto it, and exercise synthetic/isolated response fixtures for null/tri-state/policies. Repository source search found no application imports of shared SDK outside its own package, so current in-repo migration scope is favorable; external consumers are unknown.

### P1 — Accepted HTTP time-frame payloads can panic or bypass invariants

**Evidence:** Both HTTP routes default TimeSpans to Append (`core/infrastructure/web_api/src/routes/statistics.rs:251`, `:330`). Policy is `todo!()` when any current value exists (`core/core/src/statistic/value_objects/update_policies/time_frame_list.rs:21`). UpdateEndValue can also panic on overlap (`:27`), with an existing should-panic test (`:156`). Raw list route calls serde directly (`core/infrastructure/web_api/src/routes/statistics.rs:239`); `TimeFrame` and `TimeFrameList` derive Deserialize (`core/core/src/shared/time_frame_list.rs:19`, `:123`), bypassing `TimeFrame::new` ordering check (`:61`) and append overlap checks (`:143`). Overwrite policy returns submitted list unchanged (`core/core/src/shared/time_frame_list.rs:17`).

**Consequence/design:** Second normal default append is a panic path; JSON-encoded lists can persist reversed/overlapping intervals despite the dedicated endpoint rejecting some of them. Make parsing construct validated domain collections and make update failure a typed Result at the ingestion boundary. This must precede claiming the API safe for varied mobile/logger inputs. Source trace is high confidence; no production invocation or Rust execution was done.

### P1 — Offline/mobile replay has no general identity or ordering contract

**Evidence:** Ordinary log request only value/policy (`core/infrastructure/web_api/src/dto/statistic.rs:29`); sum blindly adds again (`core/core/src/statistic/value_objects/update_policies/amount.rs:19`). LiftLog has only days/workouts, drops workout IDs into unnamed local timestamps (`core/infrastructure/web_api/src/routes/integrations.rs:54`, `:127`) and replaces values (`core/application/src/use_cases/integration/service.rs:366`). Phone seq is local acknowledgement protection, never transmitted (`logger_applications/LiftLog/app/src/db/lifestats-sync.ts:51`, `logger_applications/LiftLog/app/src/services/lifestats-sync.ts:84`). Docs explicitly accept requests arriving after timeout and overwriting newer data (`logger_applications/LiftLog/docs/LifestatsSync.md:77`). Router has no reconciliation/change-cursor surface (`core/infrastructure/web_api/src/routes/mod.rs:29`).

**Consequence/design:** Retrying uncertain additive delivery double counts; retrying snapshots is idempotent only for identical content, not safe against stale/newer ordering or multiple devices. Select explicit semantics: stable source/device/event IDs for additive observations; monotonic source revision or conditional replacement for snapshots; scoped provenance; durable client outbox and server dedupe; read synchronization/deletions. Progress checkpoint source/event identity already exists and is worth reusing conceptually (`core/infrastructure/web_api/src/dto/progress.rs:29`). Do not mandate one universal endpoint before deciding ownership semantics.

### P1 — Rust logger bootstrap requires local schema ownership and bypasses common ingestion

**Evidence:** Active MIDI uses concrete full `build_services` and local repository (`logger_applications/midi_logger/src/main.rs:26`, `:58`). Bootstrap exposes `Arc<SqliteStatisticRepository>` (`lifestats-bootstrap/src/lib.rs:47`); pool requires env, WAL and migration privileges (`core/infrastructure/persistence/src/pool.rs:14`, `:24`, `:49`). Heartbeat directly saves (`core/application/src/use_cases/session_tracking_service.rs:478`) rather than using common log command/event path (`core/application/src/use_cases/commands/log_value/handler.rs:165`).

**Consequence/design:** New logger deployment inherits DB schema/version coupling, unrelated rendering/Fitbit construction, process-local session state, and separate ingestion behavior. Separate server composition from logger transport composition. Prefer remote ingestion for independently deployed loggers; if local embedded mode is a real requirement, make it explicit and route both adapters through shared application semantics. Do not mistake inward dependency direction for process isolation.

### P1 — Error/status contract does not reliably tell clients what to fix or retry

**Evidence:** README claims validator errors are 400 (`core/infrastructure/web_api/README.md:880`); code returns 422 (`core/infrastructure/web_api/src/error.rs:36`) and test expects 422 (`core/infrastructure/web_api/tests/statistics_api.rs:201`). Central `ApplicationError` mapping exists (`core/infrastructure/web_api/src/error.rs:130`) but goals and creation wrap failures into InternalError (`core/infrastructure/web_api/src/routes/goals.rs:98`, `:314`; `core/infrastructure/web_api/src/routes/statistics.rs:432`). Missing required derived source IDs produces `ApplicationError::InvalidInput` (`core/application/src/use_cases/commands/create_statistic/handler.rs:44`), then 500 in HTTP creation. Most handlers use raw Json/Path extractors, while LiftLog alone explicitly converts JsonRejection (`core/infrastructure/web_api/src/routes/integrations.rs:261`, `:269`).

**Consequence/design:** User mistakes can look retryable; clients depend on status to retain/retry/drop work (LiftLog, screen-time, bedtime). Standardize typed public errors with stable code and field details, consistent extractor mapping, and one application-to-HTTP conversion; preserve retry metadata such as existing Retry-After (`core/infrastructure/web_api/src/error.rs:66`). No need to expose internal error strings to establish diagnosis.

### P2 — Validation and interpretation rules disagree across boundaries

**Evidence:** UI constrains efficiency 0–100 (`ui/web-ui/src/lib/schemas.ts:47`), server parses any u8 then `Percentage::new` (`core/infrastructure/web_api/src/routes/goals.rs:165`, `:174`), whose constructor has no bound check (`core/core/src/shared/percentage.rs:17`). Default and daily goal handlers duplicate type parsing (`core/infrastructure/web_api/src/routes/goals.rs:101`, `:320`). SDK formatDate uses UTC while formatTime uses local time (`clients/lifestats-js/src/schemas.ts:143`, `:150`). Anki sends start/end clock with today's date (`logger_applications/AnkiLogger/session_tracker.py:230`); Rust heartbeat similarly discards endpoint dates and chooses current date during persistence (`core/application/src/use_cases/session_tracking_service.rs:430`, `:478`). Dedicated TimeFrame rejects end before start (`core/core/src/shared/time_frame_list.rs:61`).

**Consequence/design:** Same user input can be rejected in one client and accepted in another; cross-midnight activity can fail or be misattributed. Enforce the efficiency-specific 0–100 bound (do not cap achievement percentages, which can exceed 100; `core/infrastructure/web_api/src/helpers.rs:15`), centralize goal parsing, and define effective business date separately from offset-aware observation timestamps. Preserve explicit bedtime-evening and LiftLog attributed-day behavior while fixing generic time handling.

### P2 — Deployed-client documentation and recovery behavior already drift

**Evidence:** API session docs say first beat creates session (`core/infrastructure/web_api/README.md:291`; route comment `core/infrastructure/web_api/src/routes/sessions.rs:83`), but manager beat returns SessionNotFound (`core/application/src/use_cases/session_manager.rs:129`). Start returns a nonrunning object until first beat (`core/application/src/use_cases/session_manager.rs:101`; `core/application/src/use_cases/session_tracking_service.rs:283`), while the route describes a newly started session. MPV relies on its own local active flag (`logger_applications/mpv_logger/mpv_logger.lua:67`, `:77`) and has no automatic re-start in its heartbeat failure branch (`:84`). Anki finish docstring says last-finish overwrite but sends ignore (`logger_applications/AnkiLogger/session_tracker.py:383`, `:409`).

**Consequence/design:** Protocol consumers can follow docs and lose activity on restarts or change metric meaning. Add behavioral consumer-contract examples covering restart/reconnect, first beat, repeat stop, first/last finish. Establish which semantics are intended before adapting callers.

### P2 — Broad network reach is configured without an application identity boundary

**Evidence:** Production main binds `0.0.0.0` and applies permissive CORS (`core/infrastructure/web_api/src/main.rs:55`, `:59`); router applies no auth middleware (`core/infrastructure/web_api/src/routes/mod.rs:177`). OAuth concerns Fitbit authorization, not caller access to Lifestats. No user/device identity appears in generic log or LiftLog snapshot DTOs.

**Consequence/design:** Current code depends on deployment network trust and one shared data namespace. For mobile, explicitly retain trusted-network single-user operation or add caller/source identity and access policy at the HTTP boundary. Source inspection cannot establish actual reverse-proxy/tailnet protections, so this is a deployment-contract decision, not a claim the live deployment is publicly exposed.

## Preservation constraints

1. Keep goal tri-state and historical date applicability; suppression is not deletion or null. Retain server-owned achievement evaluation and UI display classification separation.
2. Preserve stable integration bundle identities, logical metric keys and deck identity; Anki expects five `_id` keys (`logger_applications/AnkiLogger/api_client.py:116`), generic provisioning appends `_id` (`core/infrastructure/web_api/src/routes/integrations.rs:164`). Renaming keys breaks existing consumers.
3. Keep existing integer units and policy defaults until an explicit migration: seconds and minutes coexist; changing labels alone would corrupt interpretation.
4. Preserve LiftLog atomic session+dirty-date writes, before/after date queueing, matching-seq acknowledgements, explicit empty-day clears, and no background-send promise. Queue is device/destination metadata and deliberately excluded from backup (`logger_applications/LiftLog/app/src/store/settings/util.ts:31`). Retain workout ownership on phone and server normalization of spans.
5. Preserve ebook event IDs across retries and checkpoint repair on duplicates. Do not collapse event acceptance and downstream completion into an unretryable acknowledgement.
6. Preserve source-specific observation semantics such as bedtime's evening date, ebook confirmed reading, and completed sets. Centralize generic transport/contract/delivery semantics, not every source's domain detection.
7. Maintain legacy HTTP routes while migrating real callers; no usage telemetry was examined, and hard-coded scripts/extensions exist. A versioned replacement needs an explicit compatibility window based on actual deployment inventory.

## API readiness assessment and design sequence

**Ready foundations:** Typed application commands behind HTTP adapters, central error enum, server-side analysis/achievements, richer progress checkpoint metadata, idempotent integration provisioning, ebook event identity, and LiftLog durable transactional outbox with safe local acknowledgements.

**Not yet ready as a general offline multi-device frontend API:** SDK correctness; total validated ingestion; uniform public error handling; source/device attribution; ordered/replay-safe mutations; stable metadata with units/capabilities; reconciliation/change cursor; explicit trusted-network/auth deployment story. An online RN MVP could consume existing read/goal endpoints after fixing contract errors, but needs to state connectivity constraints rather than silently promise offline writes.

Suggested order (design recommendation, not repository change):

1. Freeze an executable public contract from actual server behavior, correcting unsafe behavior first; cover nullable/tri-state/policies/errors/time boundaries in isolated fixtures.
2. Choose observation/event versus source-owned snapshot semantics, identity, day/time handling, and conflicts. Extend existing idempotent paths deliberately.
3. Generate/share wire types and codecs, migrate SDK and web UI; RN builds on that package with storage/outbox independent of React presentation.
4. Move independently deployed Rust loggers behind transport adapters; preserve embedded option only if required. Unify application ingestion semantics so one path cannot bypass events/validation.
5. Add version/capability and reconciliation support driven by concrete mobile flows. Keep rich workout/reader state in their products unless Lifestats explicitly takes ownership.

## Confidence, verification, and gaps

- High confidence in cited call chains and exact SDK mismatches. The isolated SDK check ran actual source with synthetic responses and Zod v3 compatibility API supplied by installed UI dependency; it did not install SDK dependencies, contact HTTP, or create build artifacts.
- Rust findings are source-verified, not executed. Existing append `todo!`, overlap should-panic test, constructor bypass, error mappings and direct persistence are explicit. No Cargo/npm suites ran, because build caches/artifacts would exceed this explorer's write authorization. No live API calls or personal DB reads occurred.
- Source search found SDK references only within package source, not in consuming app source. External package users, installed MPV/Anki copies, mobile APK versions, deployment proxy/auth, scheduler/service configs and actual traffic remain unverified.
- Did not comprehensively inspect plotting representation, every UI component, Fitbit internals, full ebook DB transaction implementation, or persistence concurrency/atomic dedupe. Other reviewers should own those slices. This report does not claim lost-update safety of read-modify-write repository operations.
- No in-repo Yomikata caller found; no OpenNutriTracker `lib/` Lifestats references found. These are bounded negative searches, not proof that external integration does not exist.

## Files read

All cited implementation files were read wholly or in bounded line ranges. Additional inventory/search-only files are not represented as full reviews.

- Guidance: root `AGENTS.md`; LiftLog `AGENTS.md`, `docs/index.md`, `docs/Storage.md`, `docs/LifestatsSync.md`; OpenNutriTracker `AGENTS.md`; required external T3 adapter/model policy, how skill/explorer template, TypeScript skill and type-system principle.
- Root/assembly: `Cargo.toml`, `lifestats-bootstrap/src/lib.rs`, `core/infrastructure/persistence/src/pool.rs`.
- API: `core/infrastructure/web_api/src/routes/{mod,statistics,goals,sessions,integrations,dashboard,daily_summary,progress}.rs`, `src/{main,error,helpers}.rs`, `src/dto/{statistic,goal,progress}.rs`, inventory/validation search of other DTOs; README and `.claude/skills/lifestats/SKILL.md` bounded excerpts.
- API tests: `core/infrastructure/web_api/tests/{helpers,statistics_api,liftlog_days_e2e}.rs` excerpts; route-pattern searches in `{integrations_api,yomikata_bundle_e2e,e2e}.rs`.
- Application/domain: `core/application/src/use_cases/commands/{log_value,create_statistic}/handler.rs`, `use_cases/{session_manager,session_tracking_service}.rs`, `use_cases/integration/{service,liftlog_days}.rs`; `core/core/src/shared/{percentage,time_frame_list}.rs`; `statistic/types/logable_statistics/time_spans.rs`; `statistic/value_objects/update_policies/{amount,fixed_value,time_frame_list}.rs`.
- Clients/UI: `clients/lifestats-js/src/{client,schemas,types}.ts`, package.json; `ui/web-ui/src/lib/{api,schemas,providers,goalFormat}.tsx/ts` as applicable, `src/types/index.ts`, `features/goals/{dayStatus,hooks/useGoals}.ts`, `features/statistics/hooks/useStatistics.ts`, package.json.
- Rust/Python/extension: MIDI `src/main.rs`, Cargo.toml; stopwatch `stopwatch_logger/src/main.rs`, Cargo.toml; Anki `api_client.py`, `session_tracker.py` bounded excerpts; extension `src/entity/tab.ts`; MPV Lua logger.
- Other HTTP sources: ebook reader `apps/web/src/lib/data/lifestats/lifestats-sync-adapter.ts`; screen-time `LifestatsApiClient.kt`, `ScreenTimeSyncWorker.kt`; bedtime `BedtimeLogger.kt`, `UploadBedtimeWorker.kt`, `Bedtime.kt`; Clef sync script.
- LiftLog: `app/src/services/lifestats-sync.ts`, `db/lifestats-sync.ts`, `store/lifestats-sync/effects.ts`, `store/stored-sessions/effects.ts`, `store/settings/util.ts`, `db/sessions.ts` excerpts, `components/smart/services-provider.tsx`.
