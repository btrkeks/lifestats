# Candidate B: capability ownership, one authoritative server

## Problem and evidence

**Design proposal; no implementation performed.** One user needs several disconnected clients, without making phones replicas of server calculations. Preserve pure rules, existing IDs, integrations and aggregate history; replace ambiguous write ownership.

Current writes read then independently save JSON ([handler:56](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/commands/log_value/handler.rs:56), [repository port:32](/home/daniel/.local/share/src/lifestats/core/core/src/statistic/ports/statistic_repository.rs:32)). Ebook receipt/daily increment commits before progress effects ([integration:331](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/service.rs:331)); LiftLog replaces/clears named days without revisions ([integration:366](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/service.rs:366)). MIDI directly opens server composition and uses repository-backed sessions ([MIDI:26](/home/daniel/.local/share/src/lifestats/logger_applications/midi_logger/src/main.rs:26), [MIDI:58](/home/daniel/.local/share/src/lifestats/logger_applications/midi_logger/src/main.rs:58)). These are semantic boundaries to repair, not evidence against SQLite.

Completed probes reproduce second-append panic, reversed-interval acceptance and achievement 30/10 becoming 44%; schema probing demonstrates a forced statement schedule, **not concurrent Rust runtime safety**. SDK probing uses Zod3 compatibility exports, **not a complete SDK build** ([domain-probe](domain-probe.json), [schema-probe](schema-probe.json), [sdk-probe](sdk-probe.json)). Runtime evidence does not identify the deployed binary revision ([runtime](runtime.md)).

## Usage: caller first

The phone saves an immutable command to SQLite before displaying “pending.” Reconnection submits it unchanged; ambiguous delivery retries the same identity. A logger publishes a versioned snapshot of its own stream/day. A frontend reads resolved goals and edits against the returned revision. None loads repositories or supplies a preloaded statistic.

```text
// Mobile manual observation, transaction persists pending row + immutable outbox bytes.
local.enqueue(AddObservation(id="op-7", statistic=42, day="2026-10-22",
  value=Amount(3, "count"), sourceRecord="manual-9"));
// Delivery may repeat after restart; accepted and duplicate both reconcile the receipt.
sync.flush(transport, outbox);

// Source collection owns this complete snapshot, including an explicit empty clear.
client.submit(ReplaceDays(op="op-8", binding="liftlog-main", stream="workouts",
  epoch=2, days=[{day:"2026-10-22", revision:17, intervals:[]} ]));

view = client.readDaily([42], "2026-10-22");
client.submit(EditGoal(op="op-9", statistic=42, expected=view.goalRevision,
  change=DailyOverride(day="2026-10-22", state=Suppressed)));
```

## Two viable whole-system shapes

**A — Strengthened horizontal layers.** Keep domain/application/infrastructure/bootstrap crates. Domain owns validated values, pure reducers and evaluation. Application owns one write coordinator, read evaluator and typed persistence ports. Infrastructure owns transactions/codecs; HTTP and integration translators call the coordinator. Bootstrap exposes command/query façades instead of concrete repositories. Replace JSON repository mutation with typed load/commit contracts; move concrete rendering outside application. All consistency rules below apply unchanged.

This is viable with least movement and concentrated transaction coordination. Its deep façade hides replay, provisioning, corrections and calculations. Its cost is ownership spread: adding a source observation can require application orchestration, domain variants and infrastructure codecs in separate trees. Gates must prevent old handler/repository paths remaining equally attractive.

**B — Capability-owned modular monolith, preferred.** Statistics owns definitions, contributions, goals and all daily evaluation. Progress owns targets/checkpoints/frontier calculations. Sessions owns leases; ingestion owns source bindings and cross-capability import operations. Each capability owns its typed state, operation planning and read semantics; a small shared commit mechanism provides atomicity, not product policy. An ebook operation explicitly composes statistics and progress plans inside one commit. There is no generic plugin registry or distributed saga.

```text
lifestats-domain                 pure statistics/progress/interval calculations
lifestats-server (binary)
  product/statistics             definitions, facts, goals, canonical daily reads
  product/progress               target/checkpoint invariants, summaries
  product/sessions               durable live leases -> interval contributions
  product/ingestion              binding authority, ebook/LiftLog import operations
  product/changes                receipt and scoped invalidation protocol
  product/commit                 typed read sets, closed mutation plans
  sqlite                        private codecs + atomic plan implementation
  http                          versioned DTO parsing / legacy translators
  bootstrap                     construct adapters; expose product façades
clients                         generated contract + plain sync library
mobile                          SQLite outbox/cache adapter + presentation
```

Product modules and ports are `pub(crate)` inside the binary; SQL models remain private. Existing pure domain code remains a library, without network/database/rendering dependencies. HTTP imports only capability façades and validated inputs; capability modules cannot import HTTP/SQLite. Enforce those edges with Cargo dependency checks plus module-import linting; compile-fail fixtures prove external imports of server internals fail. Logger manifests lose bootstrap/persistence dependencies. This changes decision ownership, not merely directory names.

**Independently deployed ingestion service.** It could isolate credential refresh failures or slow collectors, but a new authoritative service would need cross-service receipt/effect transactions or expose partial acceptance. No independent authoritative lifecycle is established. Keep device collectors and scheduled remote fetchers independently deployable HTTP clients; isolate their permissions/timeouts there. Select another service only if a demonstrated resource/security lifecycle requires it, with durable handoff and explicit completion semantics. One-user scale alone supplies no such reason.

## Preferred shape: types and atomicity

Per boundary-discipline, wire/storage decoders construct validated domain values. Per model-the-domain, commands express intent rather than generic update-policy strings. Compact illustrative Rust signatures follow; bodies are intentionally unspecified.

```rust
struct Operation { id: OperationId, actor: AuthorizedActor, intent: Intent }
enum Intent {
    AddObservation(NewObservation), CorrectObservation(ConditionalCorrection),
    ReplaceDays(SourceDayBatch), EditGoal(ConditionalGoalEdit),
    RecordReading(ReadingFact), EditProgress(ConditionalProgressEdit),
    Session(SessionCommand), DeleteStatistic(ConditionalDeletion),
}
enum Value { Amount(Quantity), Habit(bool), Clock(LocalClock), Intervals(IntervalSet) }
enum Override { Inherit, Suppressed, Explicit(Goal) }
enum Outcome {
    Applied(Receipt), Duplicate(Receipt), Stale(CurrentVersion),
    Conflict(ConflictDetail),
}
enum WriteError { Invalid(FieldErrors), Unsupported, Forbidden, Gone, Busy, Internal }
impl Product {
    fn submit(&self, op: Operation) -> Result<Outcome, WriteError>; // unimplemented
    fn daily(&self, scope: DailyScope) -> Result<Snapshot<DailyView>, ReadError>;
    fn changes(&self, scope: Scope, after: Cursor) -> Result<ChangePage, CursorExpired>;
}
// Server-private persistence boundary; no SQL connection or JSON in these types.
trait AtomicStore {
    fn load(&self, intent: &Intent) -> Result<OperationState, StoreError>;
    fn commit(&self, plan: MutationPlan) -> Result<CommitResult, StoreError>;
}
struct MutationPlan { receipt: BoundOperation, read_set: VersionSet, effects: Effects }
// Effects is closed and typed: fact upserts/tombstones, goal versions,
// progress changes, session lease changes, affected read scopes.
```

`submit` authenticates authority, binds operation identity, loads canonical state, computes a pure plan, and commits. The adapter starts an immediate transaction, checks receipt first, verifies the entire read set (including metadata, tombstones, binding epoch and range versions), then writes effects, final receipt and change cursor together. A changed internal read set causes bounded re-planning; a caller's failed expected revision remains a conflict. Unique binding keys prevent concurrent provisioning duplicates without imposing uniqueness on arbitrary display tags. Foreign keys protect new fact rows; statistic deletion retains a non-reused identity tombstone and rejects late writes. Aggregate-scope versions detect phantom contributions. Concurrent increments therefore recompute rather than overwrite. Receipts for terminal stale/conflict outcomes commit without product effects; transient failures roll back and remain retryable.

Receipt uniqueness is `(installation, operation_id)` across devices. Bind canonical versioned semantic payload, destination, actor/binding and preconditions; retain canonical bytes alongside digest. Identical retry returns stored outcome without executing effects. Different payload under the same identity returns `identity_reused`. Retain compact identity records for the installation lifetime; pruning detailed responses must never make old identities executable again. Restoring a server creates a new installation epoch and demands explicit reconciliation.

### Authority, correction and time

Enroll installation-scoped device credentials with grants to specific bindings/streams; derive the authorized actor at the transport boundary, never from a caller-asserted source name. This is single-user device authorization, not tenancy. A device is a sender, not automatic authority. A binding identifies an external dataset; stream identifies its metric/partition. Snapshot streams have one authorized writer epoch; counters increase per stream/day and survive client restart, acknowledgement and outbox deletion. Backup restore enrolls a new epoch rather than resetting a counter. Handoff increments epoch, fencing old devices. Multiple devices mirroring one dataset share its authority; independent datasets use separate bindings. Equal revision/different payload conflicts; lower revision is stale; higher revision applies. Multi-day replacements are bounded, all-or-nothing batches. Empty means clear; omitted means untouched. Clear retains revision/tombstone so delayed writes cannot resurrect it.

Manual observations use stable record IDs; correction/deletion requires record revision. Source records key `(binding, stream, record_id)` and carry source version, occurrence time, attributed day and receipt time. Moving a record invalidates both days atomically. A correction replaces only that source's contribution. Statistics declare aggregation: checked sum, interval union, or single-authority register; habits/clock values never acquire accidental last-arrival semantics. Snapshot and record modes cannot overlap the same stream partition.

Day attribution is an explicit business date, distinct from offset-aware occurrence and receipt timestamps. Bedtime evening rules and LiftLog named-day clamping remain source policies. New interval facts retain instants plus timezone/attribution policy; elapsed duration and wall-clock display are separate. Legacy naive spans remain explicitly legacy rather than receiving invented offsets.

### Reads, goals, units and sessions

One canonical daily reader serves dashboards, totals, scores, progress and render data. Cache entries key server installation, scope, query/schema version and parameters; include revision/cursor and fetched-at. A snapshot and its watermark share a read transaction. Changes enumerate affected statistic/day ranges, goals, progress and deletions through a fixed watermark, including dependency invalidation. Mobile atomically installs page updates and cursor. Uncached scopes obtain their own snapshot; consuming another scope's feed never advances them. Expired cursors trigger replacement snapshots, retaining pending commands and drafts. Pending presentation is labelled provisional; it does not compute authoritative achievement offline.

Goals retain `inherit | suppressed | explicit`; default schedules are effective-dated goal-or-none entries. Edits require schedule/override revision and explicit effective date. Past schedule entries remain queryable; retrospective edits are explicit, versioned corrections. Migration preserves surviving applicability and overrides but cannot recover overwritten defaults. Progress remains separate, preserving latest/furthest semantics; effective-date queries do not impose implicit receipt-time cutoffs.

Metadata advertises units, supported intents, source policy and implemented read capabilities. Amounts have checked arithmetic and explicit signed/nonnegative semantics; wire integers outside JavaScript’s exact range use validated decimal strings; efficiency is bounded 0–100, achievement uses a wider non-wrapping ratio allowing overachievement. Interval decoding validates order, bounds, timezone and overlap normalization consistently. Unsupported derived statistics stay unavailable: a future gate must implement persisted references, type/unit validation, cycle detection, missing-data policy and canonical computation before advertising them. Existing mapper/evaluator gaps are documented in [domain evidence](domain.md#5-p1p2--derived-statistics-are-an-advertised-but-nonfunctional-capability).

Server sessions are durable online leases with session ID, fencing revision, acknowledged heartbeat sequence and last confirmed instant. Restart expires/reconstructs leases; it invents no unacknowledged activity. Source-owned offline intervals are submitted as facts, never replayed heartbeats. Both use validated interval contribution plans. Keep existing HTTP session behavior behind compatibility translation until MPV/MIDI consumers migrate.

### Wire and mobile boundary

Versioned wire DTOs own serialization, separately from domain types. Generate OpenAPI/schema, validators and language bindings from Rust contract declarations; CI regenerates and checks server/consumer fixtures, including legacy formats. Do not generate wire definitions from private persistence entities.

```json
{"operation_id":"op-9","kind":"edit_goal","statistic_id":42,"expected_revision":6,
 "change":{"kind":"daily_override","day":"2026-10-22","state":{"kind":"suppressed"}}}
{"kind":"duplicate","receipt":{"operation_id":"op-9","result":"applied","cursor":"c19"}}
{"kind":"stale","current_source_revision":18}
{"kind":"conflict","code":"revision_mismatch","current_revision":7}
{"kind":"error","code":"invalid_interval","retryable":false,"fields":["end"]}
```

The plain client library accepts transport and durable-storage interfaces; it owns envelope validation, retry classification and reconciliation, with no React/Expo imports. Mobile SQLite owns immutable outbox bytes, drafts, attempt state, receipts and cached views. Coalescing is allowed before sealing a snapshot; an attempted operation never changes payload. New edits create new operations. Acknowledgement matches identity/generation. HTTP maps invalid input to 422, forbidden to 403, deleted targets to 410, stale/conflict to distinct 409 codes, and busy to 503 with retry guidance; extractor failures share this envelope. Network/busy/429/5xx retry; conflicts await user resolution; invalid/forbidden/gone remain visible terminal failures. This builds on LiftLog's transactional dirty dates and sequence-matched acknowledgements ([sync guide:62](/home/daniel/.local/share/src/lifestats/logger_applications/LiftLog/docs/LifestatsSync.md:62)), adding immutable delivery identity.

## Synthesis decision and tradeoffs

Runner recommendation: use B as base; parent owns final synthesis. Adopt A's typed transaction boundary and centralized composition. B places source correction and evaluation ownership beside their state, making the nearest example safer. Both expose a small, deep intent/query surface while hiding storage, replay and evaluation; callers still choose attribution, source completeness and conflict resolution.

We accept optimistic re-planning and retained identities for atomic multi-capability writes. We accept capability migration work for fewer competing writers. We reject full replication, business event sourcing and generic plugins. The durable change log is bounded cache invalidation, not a replayable product history. Add background jobs only for an identified durable consumer; synchronous queries and pull synchronization need none.

Red-flag screening: no caller-coordinated repository workflow; no public wire/storage re-exports; no load/validate/save service chain; no passthrough handler layers; private internals and import gates; one mutation path after retirement; generated contract lists. Remaining risk is a central commit module accumulating policy: constrain it to version checking and typed effects, leaving decisions in capabilities.

## Migration and acceptance gates

1. **Contract baseline:** copied synthetic/snapshot fixtures preserve IDs, value JSON, units, goal tri-state, progress ordering, receipts, bundle keys and frozen LiftLog weight metrics. Capture legacy requests/responses; audit orphan/duplicate bindings on copies. Acceptance: explicit unchanged results or reviewed bug-correction deltas.
2. **Additive storage:** add bindings, receipts, source facts, tombstones, revisions, schedules and changes. Preserve old aggregates as opaque legacy baselines, never fabricated source facts. Known-source takeover requires explicit scoped reconciliation; do not add a replacement snapshot atop its unknown historical contribution. Acceptance: copied fixtures round-trip, old reads remain compatible, no ID reuse.
3. **Future proof slice:** isolated production-like SQLite with two independent consumers; concurrent increments, response loss/retry, identity misuse, stale goal, snapshot-clear-reorder, source correction/move preserving another source, restart at commit boundaries, cursor expiry. Acceptance: receipt/effects/change atomicity and deterministic convergence. This gate is specified, not completed.
4. **Writer migration:** legacy routes translate to the sole product path; retain wire formats, policies and named-day rules. Legacy clients without identities cannot gain exactly-once guarantees magically. Migrate MIDI and other DB writers; remove bootstrap dependencies and DB write credentials before claiming exclusive server ownership. Acceptance: deployment inventory confirms all writers migrated; old/new consumer contract fixtures pass.
5. **Mobile adoption/retirement:** generated client plus SQLite outbox/cache, offline restart/conflict/freshness acceptance; update API README and agent skill together. Retire compatibility only after actual caller migration. Rollback preserves additive schema and receipt identities; old binaries cannot resume direct writes after cutover.

## Open questions and first step

Which datasets are mirrored across devices, which live-session restart behavior is desired, and which historical aggregates can be reconciled to source snapshots? These determine bindings and cutover policy, not a need for tenancy. Which deployed clients still use legacy routes remains an inventory gate.

Next implementation step, after the design milestone: implement the isolated manual-observation/source-day proof slice through the typed commit boundary before restructuring remaining handlers or building mobile screens.
