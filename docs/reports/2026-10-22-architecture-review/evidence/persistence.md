# Persistence and ingestion architecture exploration

Date: 2026-10-22. Scope: `/home/daniel/.local/share/src/lifestats`, particularly the Rust server, shared repositories, server-side integration ownership, and heartbeat sessions. Read-only exploration; the sole filesystem output is this report. No live DB, service, git, HTTP mutation, or external state was touched. No further delegation.

## Overview

The implemented central store is SQLite through Diesel, with one JSON daily value per `(statistic_id, date)`, separate daily/default goals, separate bounded-progress items/checkpoints, an ebook receipt ledger, and a Fitbit credential/status table. SQLite itself is a reasonable fit for the stated single-person system. The main architectural issue is the **scope of the write contract**, not the database engine: repository calls own individual statements or small transactions, while application operations often span independently committed calls.

`build_services()` shares one pool among statistic, progress, ebook-event, and token adapters (`lifestats-bootstrap/src/lib.rs:69-92`, `:115-117`). The HTTP server uses that container (`core/infrastructure/web_api/src/main.rs:30-54`), but this is not exclusive server ownership: the active-workspace MIDI binary constructs the same services and writes directly through a heartbeat session (`logger_applications/midi_logger/src/main.rs:25-26`, `:57-65`; membership in `Cargo.toml:3-16`). The excluded stopwatch binary also constructs services directly (`logger_applications/stopwatch/stopwatch_logger/src/main.rs:166-175`). A pool singleton is process-local, not a cross-process application lock.

There are several distinct delivery models today:

- Generic logging: read, apply a domain update policy, replace one JSON row; no command receipt or revision.
- Ebook ingestion: transactionally deduplicate a source event and increment a daily total, followed by a separate progress-item/checkpoint operation.
- Fitbit: fetch/retry remote snapshots, then independently replace metric/day rows.
- LiftLog: replace or clear every explicitly supplied day; no retained workout identity or ordering revision.
- Heartbeat sessions: process-local workers incrementally replace time-span JSON; only the flushed spans survive a crash.

These should remain distinct semantics in the design, with an explicit owner and consistency guarantee for each. Calling every operation “idempotent” would conceal important differences.

## Key Concepts

### Storage and transaction boundaries

`core/infrastructure/persistence/src/pool.rs:12-17` reads `DATABASE_URL` into a lazy process-global pool. Every acquired connection enables foreign keys, a 30-second busy timeout, and WAL (`:22-31`). The pool allows ten connections, checks connections on checkout, and runs embedded pending migrations during construction (`:35-50`). Configuration, connection construction, and migration failures use `expect`, so they can panic rather than become the builder's advertised `Result` (`:14`, `:44-50`). No environment values were read for this review.

`StatisticRepository` exposes independent JSON read/save/delete calls, with no transaction/unit-of-work, compare-and-swap, expected revision, or source-operation identity (`core/core/src/statistic/ports/statistic_repository.rs:11-76`). Each SQLite call obtains its own pooled connection (`core/infrastructure/persistence/src/repository.rs:49-55`). Explicit adapter transactions exist for metadata plus initial default goal (`:148-154`), batches of metadata (`:187-201`), and metadata deletion plus its daily values/goals (`:257-278`). Saving a daily log is an unconditional UPSERT (`:304-330`). WAL and a busy timeout handle database access; they do not make a read in one repository call and a later write in another call atomic.

Progress item creation and checkpoint insertion each have their own transaction (`core/infrastructure/persistence/src/progress_repository.rs:64-76`, `:164-200`). Ebook receipt insertion and daily increment share one transaction (`core/infrastructure/persistence/src/ebook_event_store.rs:36-104`). There is no enclosing transaction spanning ebook daily totals and progress checkpoints.

### Canonical values versus records of origin

Daily logs contain only statistic ID, date, and JSON value (`core/infrastructure/persistence/src/db/schema.rs:33-39`). They have no source record, device, revision, ingestion timestamp, deletion marker, or operation receipt. Source tags belong to statistic metadata, not individual contributions (`:83-92`). Consequently, the daily aggregate does not retain enough information to remove one logger's contribution while preserving another's.

Ebook receipts retain event ID, source, metric, statistic, date, delta, book identity/title, position, and creation timestamp (`core/infrastructure/persistence/src/ebook_event_store.rs:40-50`); the receipt key is globally `event_id`, not `(source, device, event_id)` (`core/infrastructure/persistence/migrations/2026-04-05-000000_ebook_sync_events/up.sql:1-12`). Progress checkpoint deduplication is instead scoped to `(progress_item_id, source, source_event_id)` (`core/infrastructure/persistence/migrations/2026-05-02-000000_progress_items/up.sql:43-45`). These are different deduplication domains.

### Dates and time

Daily logs use ISO date text, and queries compare it lexically as a date range (`core/infrastructure/persistence/src/repository.rs:289-292`, `:353-357`). A `TimeFrame` holds only naive wall-clock start/end times, serialized to seconds; its constructor rejects end-before-start (`core/core/src/shared/time_frame_list.rs:19-40`, `:61-65`). Offsets, time zones, and elapsed instants are absent from persisted spans.

Progress checkpoints distinguish UTC `recorded_at`, an explicit `effective_date`, and UTC `created_at` (`core/core/src/progress/checkpoint.rs:6-16`). The SQLite adapter normalizes timestamps to UTC RFC3339 seconds and orders by timestamp then row ID (`core/infrastructure/persistence/src/progress_repository.rs:203-214`, `:240-244`, `:330-337`). Ebook ingestion currently supplies server receipt time as `recorded_at`, not source occurrence time (`core/application/src/use_cases/integration/service.rs:333-340`).

## How It Works

### Generic log write: HTTP → domain calculation → SQLite

1. `POST /statistics/{id}/log/{date}` is routed to `statistics::log_value` (`core/infrastructure/web_api/src/routes/mod.rs:43`). The adapter validates the payload, fetches metadata to determine type, parses the string payload, and selects a policy (`core/infrastructure/web_api/src/routes/statistics.rs:132-173`). Integer default is `Sum`; time-span default is `Append` (`:239-251`).
2. It builds `LogValueCommand { statistic_id, date, payload, statistic }` and invokes a fresh handler (`:265-280`). Thus metadata loaded outside the command accompanies the command; it is not revalidated under the value write's transaction.
3. `LogValueCommandHandler::handle` reads existing JSON (`core/application/src/use_cases/commands/log_value/handler.rs:45-59`), deserializes the typed value, calls the pure domain method (`:68-79`, `:134-146`), and rejects derived or mismatched payload variants at runtime (`:149-160`).
4. If changed, it calls `save_log_value`, then emits `ValueUpdated(statistic_id)` (`:164-169`). The adapter replaces the complete JSON row through UPSERT (`core/infrastructure/persistence/src/repository.rs:304-330`). The default composition uses a no-op publisher (`lifestats-bootstrap/src/lib.rs:32-41`, `:74`).
5. HTTP returns 204. Parsing errors become 400; metadata not found becomes 404; command errors are wrapped in `InternalError` (`core/infrastructure/web_api/src/routes/statistics.rs:139-150`, `:278-283`), mapped to 500 (`core/infrastructure/web_api/src/error.rs:47-53`). Policy panics do not pass through that result mapping.

Failure/retry behavior: a read or save failure propagates before success; once the save commits there is no durable receipt joining the commit to the response. If the response is lost, a retry of `Sum` increments again (`core/core/src/statistic/value_objects/update_policies/amount.rs:15-24`). Two concurrent callers can both read the same old value and then replace it with independent calculations. Neither the handler nor the port supplies a lock/revision protecting that interval.

### Source provisioning: source/context → tag lookup → batch creation

`POST /integrations/{source}` calls `IntegrationService::provision_bundle` (`core/infrastructure/web_api/src/routes/integrations.rs:145-159`). A source factory supplies tagged metadata; the service finds rows containing all supplied tags, takes the first match, and batches the missing rows (`core/application/src/use_cases/integration/service.rs:206-245`). SQLite implements tag containment using `json_each`, while exact canonical fingerprint lookup is a separate application port used by Fitbit (`core/infrastructure/persistence/src/repository.rs:204-225`, `:523-542`). The batch creation is transactional, but lookup plus create is not one transaction.

Server-side source identity is currently:

- Anki: `source=anki`, `deck_name`, and metric; five actual metrics including `cards_studied` (`core/application/src/use_cases/integration/anki_bundle_factory.rs:19-24`, `:37-54`, `:111-147`). A deck rename therefore denotes a different identity under the current factory; there is no source deck ID in its contract.
- Fitbit: source and canonical metric (`core/application/src/use_cases/integration/fitbit_bundle_factory.rs:34-41`; `core/application/src/use_cases/commands/sync_fitbit_data.rs:268-277`). Credentials are one row per source name (`core/infrastructure/persistence/migrations/2025-10-07-000000_fitbit_sync_support/up.sql:2-10`).
- Ebook daily aggregate: source, metric, bundle version (`core/application/src/use_cases/integration/ebook_bundle_factory.rs:10-12`, `:28-38`); per-book progress: source, `progress_type=book`, and `reader_data_id` (`core/application/src/use_cases/integration/ebook_progress_service.rs:54-67`).
- LiftLog: source, metric, bundle version; no device/account identity (`core/application/src/use_cases/integration/liftlog_bundle_factory.rs:11-14`, `:30-40`).

Anki's metric writes use the generic server surface; no Anki-specific durable receipt path is registered in the integration routes (`core/infrastructure/web_api/src/routes/mod.rs:146-158`). Client policies, buffering, and actual Anki call frequency belong to the consumer explorer and were not inspected here.

### Ebook ingestion and replay

1. `POST /integrations/ebook/events` requires a positive numeric book ID, translates the request, and invokes the service (`core/infrastructure/web_api/src/routes/integrations.rs:170-207`).
2. The service validates event identity, supported metric, positive delta/target, nonempty book information, and nonnegative position; then provisions the aggregate statistic (`core/application/src/use_cases/integration/service.rs:251-317`).
3. `SqliteEbookEventStore::record_event` inserts a receipt with `ON CONFLICT(event_id) DO NOTHING`. A duplicate immediately returns false; a new event reads the aggregate, uses `checked_add`, and UPSERTs the result **inside the same transaction** (`core/infrastructure/persistence/src/ebook_event_store.rs:39-104`). Parsing, overflow, or database failure rolls back both the receipt and increment. Starting with the receipt INSERT avoids the generic path's unprotected read/write interval between ebook-store transactions.
4. Regardless of new/duplicate receipt, the service calls `ensure_and_log_checkpoint` using the incoming request fields (`core/application/src/use_cases/integration/service.rs:331-341`). This ensures a book progress item, potentially updates its target, then saves an absolute-position checkpoint (`core/application/src/use_cases/integration/ebook_progress_service.rs:105-131`, `:135-158`). The checkpoint adapter inserts or ignores and returns the stored duplicate (`core/infrastructure/persistence/src/progress_repository.rs:164-200`).
5. The route publishes only when `accepted=true` and responds `{accepted, duplicate}` (`core/infrastructure/web_api/src/routes/integrations.rs:239-250`).

A progress failure after step 3 returns an error **after the daily total has committed**. A retry with the identical payload can repair the missing checkpoint without recounting the daily increment, provided provisioning resolves to the same item/statistic. This is a useful repair path, but not atomic ingestion, and its preconditions are not encoded in a single persistence operation.

### LiftLog snapshot/clear

`PUT /integrations/liftlog/days` validates the whole request and calls `sync_liftlog_days` (`core/infrastructure/web_api/src/routes/integrations.rs:259-278`). RFC3339 times are compared as instants for end-before-start validation, then converted with `naive_local()`, discarding offsets (`:110-130`). Workout IDs are not retained in `LiftLogWorkout` (`core/application/src/use_cases/integration/liftlog_days.rs:4-10`).

`day_time_frames` clamps a workout into the supplied calendar date, gives it at least 60 wall-clock seconds, and merges touching/overlapping intervals (`core/application/src/use_cases/integration/service.rs:129-191`). The service provisions metadata, loops dates, deletes a daily row when frames are empty, or performs read/compare/replace (`:349-391`, `:107-125`). Each day commits separately; a later failure leaves earlier dates applied. Retrying the identical full request normally converges. Publication happens only after the entire service call succeeds (`core/infrastructure/web_api/src/routes/integrations.rs:276-300`).

The documented arrival-order limitation is real and explicitly accepted for the current one-phone setup (`docs/liftlog-integration.md:37-49`). It becomes an architecture requirement, rather than an unknown old bug, when several devices and offline requests are introduced.

### Fitbit pull/snapshot sync

`sync_handler` awaits the shared `SyncFitbitDataCommandHandler` (`core/infrastructure/web_api/src/integrations/fitbit/handlers.rs:199-216`). The handler validates the range, loads/refreshes credentials, resolves metric tags, and processes dates with concurrency three (`core/application/src/use_cases/commands/sync_fitbit_data.rs:27-33`, `:120-170`). Credential and cache mutexes are allocated **per invocation** (`:132-138`).

Fetch retries include rate-limit retry-after or jittered exponential backoff, one unauthorized refresh path, and a 15-second timeout around data fetches (`:397-450`, `:456-505`). Each transformed present metric is written directly to `save_log_value` (`:309-327`), bypassing generic logging and events. Absent optional metrics produce no write and do not clear an older value (`:536-570`, `:582-596`). `last_sync_at` is written only after the entire range succeeds (`:172-174`). A partial failure can leave some snapshots committed while status remains old; replay re-fetches/replaces them, rather than resuming a durable job.

Sleep duration is attributed to requested date; wake/fall-asleep point-in-time values use upstream local dates directly (`:536-565`). Although the handler stores a configured timezone (`:53`, `:72-82`), its transformations do not apply it. Do not infer uniform server `TIMEZONE` attribution from the bootstrap setting (`lifestats-bootstrap/src/lib.rs:122-126`).

### Heartbeat session lifecycle

HTTP session routes invoke `SessionManager`; start validates only metadata existence and positive timing configuration (`core/application/src/use_cases/session_manager.rs:69-107`). The registry is an in-memory `RwLock<HashMap<StatisticId, HeartbeatSession>>` (`:48-50`). Creating a session does not start its worker: constructor sets channel/handle to `None`; first `beat()` spawns it (`core/application/src/use_cases/session_tracking_service.rs:267-285`, `:291-295`). Thus the start route can return 201 with `is_running=false` (`core/infrastructure/web_api/src/routes/sessions.rs:61-77`).

A worker starts with an immediate flush, then handles beat/timeout/stop, persisting start/end times (`core/application/src/use_cases/session_tracking_service.rs:406-471`). `persist_time_frame` chooses **the current server-local date**, reads existing JSON, applies `UpdateEndValue`, and separately saves (`:478-509`). Worker errors are logged, not returned to beat/stop HTTP callers (`:406-415`; manager `beat` and `stop_session`, `core/application/src/use_cases/session_manager.rs:126-150`). `stop_session` removes the registry entry before joining the worker; the write lock spans the join, blocking access to other sessions in that process (`:142-149`). Graceful drop stops the worker; process kill does not preserve pending timing state (`core/application/src/use_cases/session_tracking_service.rs:361-366`).

## Prioritized Findings

Priority here is relative to the supplied architecture brief's offline/multiple-device scenarios, not a claim of observed production incidents.

### P1 — Generic writes lack atomic mutation and replay identity

**Evidence:** separate read/save in `core/application/src/use_cases/commands/log_value/handler.rs:56`, `:164-169`; unconditional UPSERT in `core/infrastructure/persistence/src/repository.rs:324-330`; default additive policy in `core/infrastructure/web_api/src/routes/statistics.rs:173`. A synthetic forced read/read/write/write sequence against the actual migrated schema produces 1 after two `+1` calculations from zero. This proves the interleaving's consequence, not that it happened in production.

**Consequence:** concurrent increments/spans can overwrite each other; a response-lost retry can count the same action twice. Direct MIDI writers can race the HTTP process even if HTTP execution is otherwise serialized.

**Design constraint:** accepted commands need durable operation identity and an atomic mutation boundary. For independent source contributions, retain source facts rather than treating a shared aggregate as the only recoverable state. Full replacement edits need expected revisions; increment operations need atomic application plus receipt. Process-local mutexes alone do not cover existing direct writers.

### P1 — Provisioning's promised idempotency is sequential, not guaranteed concurrently

**Evidence:** lookup-then-create and “first match” in `core/application/src/use_cases/integration/service.rs:217-245`; canonical uniqueness was deliberately removed by `core/infrastructure/persistence/migrations/2025-11-06-153009_remove_canonical_tags_unique_constraint/up.sql:1-4`. The actual migrated schema accepts duplicate canonical identities. Exact Fitbit lookup errors if multiple rows exist (`core/infrastructure/persistence/src/repository.rs:539-545`). Ebook progress also looks up an item and independently creates one (`core/application/src/use_cases/integration/ebook_progress_service.rs:110-131`), with only a nonunique identity index (`core/infrastructure/persistence/migrations/2026-05-02-000000_progress_items/up.sql:20-21`).

**Consequence:** two first-use requests may create separate aggregates/books; later sources may choose one unpredictably or fail exact Fitbit resolution. Duplicate ebook book items also split the scope of checkpoint deduplication.

**Design constraint:** add a uniquely constrained source binding keyed by stable source/context/metric identity, independent of arbitrary display tags. Provision under one persistence operation. Do not blindly restore uniqueness on all metadata tags: ordinary manually created statistics may legitimately share tags, and existing duplicates require reconciliation without dropping history.

### P1 — Reachable default time-span append panics on the second stored value

**Evidence:** `TimeFrameListUpdatePolicy::Append` with `Some(existing)` is `todo!()` (`core/core/src/statistic/value_objects/update_policies/time_frame_list.rs:21`). Both generic time-span logging and `log_time_frame` default to Append (`core/infrastructure/web_api/src/routes/statistics.rs:251`, `:330`); handler executes the domain policy (`core/application/src/use_cases/commands/log_value/handler.rs:140-141`; `core/core/src/statistic/types/logable_statistics/time_spans.rs:62-87`).

**Consequence:** another append to an already logged day panics instead of providing a normal append/conflict response. This is a deterministic source path, not a concurrency hypothesis. The running binary's revision and panic containment were not tested.

**Design constraint:** establish deliberate overlap, deduplication, and error semantics before exposing the default to another frontend. `UpdateEndValue` also panics on overlaps (`core/core/src/statistic/value_objects/update_policies/time_frame_list.rs:26-28`); untrusted source conflicts should have a defined outcome.

### P1 — Sessions accept the wrong statistic type and lose date/failure information

**Evidence:** start checks existence only (`core/application/src/use_cases/session_manager.rs:74-77`); persistence always writes `TimeFrameList` without a type check (`core/application/src/use_cases/session_tracking_service.rs:478-509`). If an IntegerAmount/Habit has no existing value, the first flush can store span JSON under the wrong statistic type; if it has an incompatible value, deserialization fails and the worker stops. Midnight handling discards `DateTime` dates before persistence (`:427-448`), then assigns `now().date_naive()` (`:480`); `TimeFrame::new` rejects start-after-end (`core/core/src/shared/time_frame_list.rs:61-65`). Failures are only logged (`core/application/src/use_cases/session_tracking_service.rs:413-415`).

**Consequence:** API success does not prove a successful durable session flush; wrong-type starts can corrupt the daily value's expected shape, crossing midnight can terminate tracking, restart loses registry/tail state, and duplicate activity from another process is outside the manager's exclusivity.

**Design constraint:** validate TimeSpans at the session boundary, choose durable session identity/lease and acknowledgment semantics, and split interval facts at attribution boundaries before converting to wall-clock spans. A stronger durable model is a new requirement; current sessions are explicitly in-memory code, not evidence of an existing recovery mechanism.

### P1 — Ebook receipt deduplication does not cover the complete ingestion effect

**Evidence:** the daily ledger transaction commits before progress (`core/application/src/use_cases/integration/service.rs:331-341`); duplicate receipt returns without comparing payload (`core/infrastructure/persistence/src/ebook_event_store.rs:53-61`), but progress uses the **new request's** book and position. Book target changes happen before checkpoint deduplication (`core/application/src/use_cases/integration/ebook_progress_service.rs:113-120`, `:146-158`).

**Consequence:** identical retry can repair a partial failure, but reusing an event ID with different book content can leave the original daily receipt and a checkpoint for another book. Duplicate requests can still mutate a target. Re-provisioned metadata/items can change the destination of replay. A failed progress call after a committed total also prevents the route's original event publication; its retry is now `accepted=false` and does not publish (`core/infrastructure/web_api/src/routes/integrations.rs:239-242`).

**Design constraint:** receipt identity must bind immutable payload and destination; conflicting reuse should be rejected. Commit the receipt and all authoritative effects together, or durably record pending projection work and replay the stored receipt payload. Preserve the already-correct atomic receipt+daily-increment behavior.

### P1 — LiftLog snapshot replacement has no source revision or device ownership

**Evidence:** server retains only naive timestamps (`core/application/src/use_cases/integration/liftlog_days.rs:4-10`), uses a singleton source bundle, and independently replaces dates (`core/application/src/use_cases/integration/service.rs:349-391`). No request revision check occurs. Arrival ordering is explicitly documented and accepted (`docs/liftlog-integration.md:43-49`).

**Consequence:** an old request may resurrect a cleared date or undo an edit; two phones can replace each other's days. Partial batch commit can expose a moved workout on both/neither date until a retry completes. There is no source record retained for a per-workout correction.

**Design constraint:** choose whether the authoritative unit is a per-device/source day snapshot or stable workout records. For snapshots, persist monotonic revisions and clear tombstones per binding/day, including receipt semantics. For workout facts, retain source IDs and revisions and derive days. Preserve the current explicit-day replacement/clear contract while transitioning; do not silently reinterpret it as additive.

### P1 — Progress daily deltas can omit late-arriving history based on query end date

**Evidence:** ebook checkpoint `recorded_at` is server receipt time (`core/application/src/use_cases/integration/service.rs:338`), but daily-delta query bounds recorded time to UTC end-of-day of the requested **effective-date** range (`core/application/src/use_cases/queries/get_progress_daily_deltas/handler.rs:33-45`) before grouping by effective date (`:48-58`).

**Consequence:** a checkpoint effective May 1 but uploaded May 3 is excluded from a May 1-only query, yet may contribute to May 1 when querying May 1–3. The same historical day's answer can depend on the selected range. Late older positions also become “latest” by arrival time (`core/infrastructure/persistence/src/progress_repository.rs:208-214`), although furthest position remains distinct (`core/application/src/use_cases/queries/get_progress_summary/handler.rs:33-46`). The latter may be intended; the date-filter mismatch needs an explicit decision.

**Design constraint:** distinguish source occurrence/ordering, receipt time, effective date, and optional as-of query semantics. Do not use an effective-date endpoint as an implicit receipt-time cutoff. Preserve the intended latest-versus-furthest distinction and positive frontier calculation.

### P1 — Delete/write concurrency is not guarded by the log schema; statistic IDs can be reused

**Evidence:** the rebuilt `logging_data` has no foreign key (`core/infrastructure/persistence/migrations/2024-12-22-111651_logging_data_non_null/up.sql:2-7`). Synthetic migrated-schema checks confirmed `PRAGMA foreign_key_list(logging_data)` is empty and a nonexistent statistic ID can be inserted even with foreign keys ON. Metadata deletion transaction clears existing logs and deletes the statistic (`core/infrastructure/persistence/src/repository.rs:260-277`), but late independent writes are still accepted. `statistics.id` is `INTEGER PRIMARY KEY` without AUTOINCREMENT (`core/infrastructure/persistence/migrations/2024-12-11-185406_new_tables/up.sql:2-7`); deleting the highest ID and inserting can reuse that ID, confirmed synthetically.

**Consequence:** a worker holding stale metadata can reinsert an orphan log after deletion; a cached client ID can later refer to a different statistic if reused. Deleting an ebook aggregate cascades its dedup receipts (`core/infrastructure/persistence/migrations/2026-04-05-000000_ebook_sync_events/up.sql:12`), so re-ingestion after recreation no longer remembers those receipts. These are mechanisms, not evidence of existing orphan records.

**Design constraint:** stable identities and deletion lifecycle must cover offline clients and workers; tombstone/reject stale writes explicitly. Before adding a log FK, audit and preserve/reconcile old orphans in a copy and handle legacy direct writers. Receipt retention should match the intended deletion/replay policy.

### P2 — Goal value, goal effective date, and stale-edit protection are separate writes

**Evidence:** setting a default goal updates metadata and then saves goal JSON (`core/application/src/use_cases/commands/set_default_goal/handler.rs:133-140`), using current server-local date (`:54`); the adapter commits these separately (`core/infrastructure/persistence/src/repository.rs:157-184`, `:501-507`). Neither port operation accepts an expected revision (`core/core/src/statistic/ports/statistic_repository.rs:16`, `:68-72`).

**Consequence:** a mid-operation failure can change goal applicability without changing its value; two offline edits can silently replace each other. This directly conflicts with the brief's stale-goal-edit scenario.

**Design constraint:** one atomic goal-change operation encompassing applicability metadata/value and receipt/revision. Keep SQL NULL daily suppression distinct from missing override (`core/infrastructure/persistence/src/repository.rs:389-399`, `:411-435`).

### P2 — Domain notifications currently provide no durable downstream work guarantee

**Evidence:** default publisher is a no-op wired into HTTP (`lifestats-bootstrap/src/lib.rs:32-41`, `:74`; `core/infrastructure/web_api/src/main.rs:34-40`). The separate Unix implementation opens a socket and logs publication failures without persistence (`events/event_impl/src/unix_socket.rs:30-43`, `:83-90`). Generic logging publishes after commit; heartbeat and Fitbit paths save directly; LiftLog publishes only after the whole loop succeeds.

**Consequence:** notifications cannot serve as an authoritative mobile change feed or guarantee that projections/reactors eventually catch up. Root guidance describes reactive subscribers, but that intent is not what the default runtime wiring implements.

**Design constraint:** if durable reactions are required, put an outbox/change sequence in the same transaction as accepted state. If queries remain synchronous and no background reaction is required, explicitly keep notifications best effort rather than adding event machinery for its own sake. Progress events are explicitly a v1 non-goal in the proposed ADR (`docs/plans/progress-items-architecture-adr.md:96-109`).

### P2 — Fitbit retries protect fetches, not cross-run credential ownership or deletions

**Evidence:** per-run mutex allocated after initial token loading (`core/application/src/use_cases/commands/sync_fitbit_data.rs:132-138`); unauthorized handling refreshes outside the mutex, then writes shared credentials (`:409-426`); proactive load/refresh also occurs outside a shared per-source lock (`:197-211`). Missing values skip persistence (`:324-327`, `:536-570`).

**Consequence:** simultaneous sync invocations can refresh the same credential independently; even within a run, concurrent unauthorized responses can refresh the same snapshot. Whether upstream rotation rejects/invalidates one is unverified. A remote deletion/absence leaves stale local snapshots unless the upstream response explicitly contains a replacing value.

**Design constraint:** serialize/compare credential refresh per integration binding, define absent-versus-deleted snapshot semantics, and record a sync run's success/partial failure truthfully. Keep bounded fetch retry/backoff behavior; do not equate `last_sync_at` with a completeness cursor.

## Where Things Live / Components Found

| Component | Location | Responsibility and boundary |
|---|---|---|
| Pool/migrations | `core/infrastructure/persistence/src/pool.rs:10-60` | Environment-selected SQLite, per-connection pragmas, startup migrations. |
| StatisticRepository | `core/core/src/statistic/ports/statistic_repository.rs:11-76` | Metadata, JSON values, and goals; independent operations. |
| SqliteStatisticRepository | `core/infrastructure/persistence/src/repository.rs:40-55` | Diesel adapter, local transactions, unconditional daily replacement. |
| ProgressRepository | `core/core/src/progress/ports/progress_repository.rs:21-50` | Items and append-only absolute checkpoints, duplicate result type. |
| SqliteProgressRepository | `core/infrastructure/persistence/src/progress_repository.rs:63-252` | Item storage, checkpoint receipt scope, ordered timestamp reads. |
| IntegrationService | `core/application/src/use_cases/integration/service.rs:76-102` | Bundle lookup/create, ebook orchestration, LiftLog day replacement. |
| SqliteEbookEventStore | `core/infrastructure/persistence/src/ebook_event_store.rs:35-105` | Atomic event receipt plus daily delta. |
| EbookProgressService | `core/application/src/use_cases/integration/ebook_progress_service.rs:105-163` | Resolve book, adjust target, append idempotent checkpoint. |
| SyncFitbitDataCommandHandler | `core/application/src/use_cases/commands/sync_fitbit_data.rs:46-60` | OAuth use, retry/cache/concurrency, snapshot transformation/persistence. |
| SqliteTokenStore | `core/infrastructure/persistence/src/token_store.rs:79-182` | One credential/status record per source name. |
| SessionManager / Heart | `core/application/src/use_cases/session_manager.rs:48-50`; `session_tracking_service.rs:378-384` | Process registry versus timing worker/direct span persistence. |
| Bootstrap | `lifestats-bootstrap/src/lib.rs:65-172` | Concrete adapters and no-op event wiring shared by API/local binaries. |
| LiftLog local storage | `logger_applications/LiftLog/docs/Storage.md:3-11`, `:76-85` | Documented separate phone SQLite/Drizzle database, not the server DB. Client implementation belongs to the other explorer. |

## Gotchas and implementation-versus-intent distinctions

- Anki guide says four metrics and idempotent atomic provisioning (`logger_applications/AnkiLogger/INTEGRATION.md:19-35`), while implementation has five and only the missing-row insert batch is atomic. Do not infer concurrent idempotency from that guide.
- The progress ADR is marked **Proposed** (`docs/plans/progress-items-architecture-adr.md:3-5`), yet progress schema, repositories, commands and routes are present. Treat its semantics as intent and verify each against code.
- Root guidance's zero-dependency domain and event descriptions are not a persistence verification. Actual statistic ports carry serde JSON (`core/core/src/statistic/ports/statistic_repository.rs:27-37`); notification names are `ValueUpdated` and `GoalUpdated` (`events/event_impl/src/event_port.rs:7-14`), and default runtime drops them.
- Persisting derived statistic metadata is unsupported by the current SQLite mapper: only four loggable types are recognized, and other `AnyStatistic` variants return an error (`core/infrastructure/persistence/src/db/mappers.rs:27-34`, `:54-63`, `:87-101`). SQL migration CASE branches mentioning Summing/TimeDifference are not evidence that their metadata currently round-trips.
- Generic time-span JSON deserialization bypasses `TimeFrame::new`: fields derive Deserialize (`core/core/src/shared/time_frame_list.rs:19-30`) and `TimeFrameList` transparently derives Deserialize (`:123-127`); the HTTP adapter uses `serde_json::from_str` (`core/infrastructure/web_api/src/routes/statistics.rs:239-240`). A new canonical ingestion boundary should validate actual parsed invariants, not assume JSON is a constructed domain object.
- Normal metadata reads combine metadata/default-goal queries without a read transaction (`core/infrastructure/persistence/src/repository.rs:111-122`, `:125-145`). A dashboard can therefore observe an intermediate goal update; the issue originates at the multi-write goal boundary.
- The session manager's doc says status survives stopping, but `stop_session` removes its entry (`core/application/src/use_cases/session_manager.rs:142-170`). A stopped/idle worker can remain registered until explicit stop, preventing another `start` even when not running (`:95-99`).
- LiftLog `total_weight` is still provisioned for compatibility but frozen (`core/application/src/use_cases/integration/liftlog_bundle_factory.rs:48-74`; documented `docs/liftlog-integration.md:19-20`). Do not delete it as apparently unused data or revive old backup parsing. The old backup ingest route was explicitly removed (`docs/liftlog-integration.md:71-73`).

## Preservation constraints for any migration

1. **Preserve statistic IDs and existing daily JSON.** Do not reassign source-tagged histories or recompute imported totals without a reconciled mapping. Keep dates, span wall-clock representation, integer/bool/point-time JSON and existing goal payloads readable. Mapper fallback for an invalid default-goal start date is current behavior (`core/infrastructure/persistence/src/db/mappers.rs:135-137`); altering it needs a compatibility decision.
2. **Preserve three daily goal states:** absent row = no override, NULL = suppressed, JSON = explicit goal (`core/infrastructure/persistence/src/repository.rs:389-399`). A migration that collapses NULL into missing changes past scores.
3. **Keep global ebook daily activity separate from per-book absolute progress.** The split is explicit in ADR intent (`docs/plans/progress-items-architecture-adr.md:33-51`, `:64-94`) and implemented by separate stores. Dedup receipts are data, not disposable logs; retain them across deployments and define deletion retention.
4. **Keep LiftLog's exact named-day replacement, omission, clear, clamping, overlap merge and minimum-duration rules** until a versioned API changes them. Its phone queue documents durable changed-day capture and conditional acknowledgments (`logger_applications/LiftLog/docs/LifestatsSync.md:62-79`); this is consumer intent, not a server ordering guarantee.
5. **Do not add uniqueness to generic tags indiscriminately.** Introduce/reconcile stable source bindings while retaining legitimate duplicate-tag manual statistics. Anki display deck name is current identity; adopting a stable deck ID requires an explicit mapping instead of inventing a new set of metrics.
6. **Retire direct SQLite writers deliberately.** MIDI is an active Cargo member; moving all write ownership into the server must migrate that binary as well as remote clients. A new server guarantee cannot be asserted while old binaries still UPSERT behind it.
7. **Use additive migrations and compatibility fixtures before replacing tables.** Existing migrations are embedded and automatically run when any process constructs a pool (`core/infrastructure/persistence/src/pool.rs:46-50`), so startup itself is a schema writer. Avoid deploying competing old/new writers against incompatible schema assumptions.
8. **Do not run the existing compatibility command on the original snapshot as a read-only check.** `justfile:7-8` points directly to a snapshot; the test applies pending migrations to that path (`core/infrastructure/persistence/tests/compatibility_test.rs:16-24`, `:35-44`). It tests only metadata load/nonemptiness (`:49-70`), not daily JSON, receipt replay, goal distinctions, progress ordering or deletion. Copy snapshots for future tests and expand assertions using nonprivate fixture data.

## Recommended persistence proof slice (proposal, not implementation)

Keep one server-owned SQLite store. Define an accepted-write envelope with durable operation ID, stable source binding/device context, immutable payload identity, source/effective timestamps, and expected revision where replacements are allowed. Model source record revisions and deletion explicitly for imports; keep daily aggregates as a query/projection rather than losing all source facts. One transaction must cover the receipt, authoritative state mutation, revision and any required durable reaction/change sequence. An application write port should name this atomic operation; exposing a generic transaction callback with SQL connection types to loggers would preserve the present ownership leak.

The smallest useful proof can use one additive manual statistic plus one replacement source day. It must show: retry after commit/response loss applies once; simultaneous increments both survive; stale replacement/clear cannot resurrect data; conflicting operation-ID reuse is rejected; source A correction preserves source B; and server restart resumes or safely retries required work. Preserve the legacy projection/query representation so existing clients continue reading. Compatibility adapters must have an explicit retirement path; they cannot manufacture source history that old daily JSON never retained.

For sessions, separately decide whether the source owns durable interval facts or the server owns a durable heartbeat lease. The present process registry cannot satisfy restart recovery by rearranging repository modules. For progress, decide whether recorded time means source observation or ingestion and make effective-date queries independent of accidental upload order unless an explicit as-of query is requested.

## Confidence, verification, and gaps

**High confidence:** source call chains, transaction placement, no-op runtime events, missing log FK, permitted duplicate tags, default Append panic, session date/type path, ebook split effects, and progress cutoff are directly supported by code. No claim is made that production has encountered the races or contains damaged data.

**Lightweight verification performed:** Python stdlib SQLite `:memory:` only; loaded all 14 checked-in `up.sql` migrations in filename order with foreign keys ON. Results: migrations succeeded; logging-data FK list empty; duplicate canonical source tags accepted; forced read/read/write/write `+1,+1` from zero ended at 1; orphan statistic log accepted; highest deleted statistic ID reused. This models the actual schema and statement pattern, not the Rust runtime, WAL contention timing or an observed production race. The executable reproduction is embedded below for rerun.

**Not run:** Cargo tests/build, snapshot compatibility command, live HTTP calls, DB reads, service restart, real Fitbit/OAuth calls, or mobile automation. Building/tests would create files beyond the sole assigned report, and some repository checks can mutate configured DBs. Existing persistence tests mostly use one-connection in-memory pools (`core/infrastructure/persistence/src/repository.rs:564-575`; ebook tests `core/infrastructure/persistence/src/ebook_event_store.rs:115-132`); LiftLog E2E also uses one connection without production connection customization (`core/infrastructure/web_api/tests/liftlog_days_e2e.rs:25-31`). These tests do not establish cross-process concurrency safety. Fitbit tests include sequential idempotency and token-refresh scenarios (`core/application/tests/sync_fitbit_data_tests.rs:176`, `:331`, `:420`) but were only searched here, not executed.

**Open questions:** deployed binary revision; actual simultaneous logger activity; source ID scope across ebook databases/devices; desired Anki deck-rename semantics; whether server sessions are intended to survive restart; precise business handling of DST elapsed time versus wall-clock spans; whether all needed reactive consumers still run; current snapshot schema/data diversity. No secrets, personal values, token contents, or source DB rows were inspected.

### Isolated schema/interleaving reproduction

Run from repository root; creates no files and never reads an existing database:

```python
import pathlib, sqlite3
root = pathlib.Path('core/infrastructure/persistence/migrations')
c = sqlite3.connect(':memory:', isolation_level=None)
c.execute('PRAGMA foreign_keys=ON')
for migration in sorted(root.glob('*/up.sql')):
    c.executescript(migration.read_text())
print('logging_data_foreign_keys', c.execute(
    'PRAGMA foreign_key_list(logging_data)').fetchall())
q = '''INSERT INTO statistics
(name,category,statistic_type,default_goal_valid_since,tags,canonical_tags)
VALUES ('Synthetic','test','IntegerAmount','2026-10-22',
'{"source":"synthetic"}','source=synthetic')'''
c.execute(q)
c.execute(q)
print('duplicate_source_identity_rows', c.execute(
    "SELECT COUNT(*) FROM statistics WHERE canonical_tags='source=synthetic'"
).fetchone()[0])
c.execute("INSERT INTO logging_data VALUES(1,'2026-10-22','0')")
a = int(c.execute('SELECT value FROM logging_data WHERE statistic_id=1').fetchone()[0])
b = int(c.execute('SELECT value FROM logging_data WHERE statistic_id=1').fetchone()[0])
up = '''INSERT INTO logging_data VALUES(1,'2026-10-22',?)
ON CONFLICT(statistic_id,date) DO UPDATE SET value=excluded.value'''
c.execute(up, (str(a + 1),))
c.execute(up, (str(b + 1),))
print('read_read_write_write_final', c.execute(
    'SELECT value FROM logging_data WHERE statistic_id=1').fetchone()[0])
c.execute("INSERT INTO logging_data VALUES(999,'2026-10-22','1')")
print('orphan_log_accepted', c.execute(
    'SELECT COUNT(*) FROM logging_data WHERE statistic_id=999').fetchone()[0])
c.execute('DELETE FROM statistics WHERE id=2')
c.execute(q)
print('highest_deleted_statistic_id_reused', c.execute(
    'SELECT max(id) FROM statistics').fetchone()[0])
```

### Files read

Read includes targeted sections/search results where specified, not an assertion that every file was reviewed in full. Relative paths are rooted at Lifestats.

- Guidance: root `AGENTS.md`; `logger_applications/LiftLog/AGENTS.md`; `logger_applications/LiftLog/docs/index.md`. Required external instructions: `/home/daniel/.local/share/src/pstack-t3/.pstack-t3/current/t3-adapter.md`, sibling `pstack-models.md`, `/home/daniel/.codex/skills/how/SKILL.md`, and `how/references/explorer-prompt.md`. Optional root/LiftLog `CLAUDE.local.md` checks returned no content.
- Root/build/docs: `Cargo.toml`, `justfile`, `test_data/README.md`, `docs/liftlog-integration.md`, `docs/plans/progress-items-architecture-adr.md`, `docs/reports/2026-10-22-architecture-review/brief.md`, `logger_applications/AnkiLogger/INTEGRATION.md`, `logger_applications/LiftLog/docs/LifestatsSync.md`, `logger_applications/LiftLog/docs/Storage.md`.
- Composition/events/local writers: `lifestats-bootstrap/src/lib.rs`, `events/event_impl/src/{lib.rs,event_port.rs,unix_socket.rs}`, `logger_applications/midi_logger/src/main.rs`, `logger_applications/stopwatch/stopwatch_logger/src/main.rs`.
- Domain: `core/core/src/statistic/types/logable_statistics/time_spans.rs`, `core/core/src/statistic/ports/statistic_repository.rs`, `core/core/src/shared/time_frame_list.rs`, `core/core/src/statistic/value_objects/update_policies/{amount.rs,time_frame_list.rs}`, `core/core/src/progress/checkpoint.rs`, `core/core/src/progress/ports/progress_repository.rs`.
- Application: `core/application/src/use_cases/commands/{log_value/handler.rs,set_default_goal/handler.rs,delete_statistic/handler.rs,sync_fitbit_data.rs}`, `core/application/src/use_cases/{session_manager.rs,session_tracking_service.rs}`, `core/application/src/use_cases/integration/{service.rs,liftlog_days.rs,anki_bundle_factory.rs,ebook_bundle_factory.rs,fitbit_bundle_factory.rs,liftlog_bundle_factory.rs,ebook_progress_service.rs}`, `core/application/src/shared/tags.rs`, `core/application/src/use_cases/queries/{get_progress_summary/handler.rs,get_progress_daily_deltas/handler.rs}`.
- Persistence: `core/infrastructure/persistence/src/{pool.rs,repository.rs,token_store.rs,ebook_event_store.rs,progress_repository.rs}`, `core/infrastructure/persistence/src/db/{schema.rs,queries.rs,mappers.rs}`, `core/infrastructure/persistence/tests/compatibility_test.rs`.
- All migration `up.sql` files under `core/infrastructure/persistence/migrations/`: `2024-11-23-000000_create_logging_data`, `2024-12-11-185406_new_tables`, `2024-12-22-111651_logging_data_non_null`, `2024-12-22-142054_more_columns_for_default_goal`, `2025-10-04-062139_fix_timeamount_type`, `2025-10-04-071032_add_default_goal_start_date_to_statistics`, `2025-10-04-092816_unify_default_goal_schema`, `2025-10-04-103343_unify_daily_goal_schema`, `2025-10-06-000000_add_tags_to_statistics`, `2025-10-07-000000_fitbit_sync_support`, `2025-11-06-153009_remove_canonical_tags_unique_constraint`, `2025-11-13-102621_drop_plots_table`, `2026-04-05-000000_ebook_sync_events`, `2026-05-02-000000_progress_items`.
- API: `core/infrastructure/web_api/src/{main.rs,error.rs}`, `core/infrastructure/web_api/src/routes/{mod.rs,statistics.rs,integrations.rs,sessions.rs}`, `core/infrastructure/web_api/src/integrations/fitbit/handlers.rs`, `core/infrastructure/web_api/tests/liftlog_days_e2e.rs` (setup). Search-level inspection: `core/infrastructure/web_api/tests/{integrations_api.rs,e2e.rs}`, `core/application/tests/sync_fitbit_data_tests.rs`, `core/infrastructure/fitbit_client/src/lib.rs`.
