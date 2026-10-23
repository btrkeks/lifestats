# Candidate A: capability ownership, one transactional server

## Problem and evidence boundary

**Proposal:** retain one authoritative Rust server and SQLite database; organize product ownership by capability. Mobile owns durable delivery and cached presentation, not replicated product calculations. This report ends at design; no proof slice is implemented.

Current generic logging separates read/save [1]; ebook commits its daily effect before checkpoint work [2]; LiftLog replaces/clears named days without revisions [3]. MIDI constructs a separate SQLite-backed writer [4]. These are different consistency boundaries, not merely folder problems. The completed [domain probe](/tmp/lifestats-architecture-20261022/domain-probe.json) reproduced second-append panic, reversed-interval acceptance, and 30/10 scoring as 44%. The [schema probe](/tmp/lifestats-architecture-20261022/schema-probe.json) demonstrated a forced SQL schedule, **not concurrent Rust execution**. The [SDK probe](/tmp/lifestats-architecture-20261022/sdk-probe.json) used Zod3's compatibility export, **not a complete SDK build**. Source baseline is `06e4e708`; deployed revision remains unknown ([runtime evidence](/tmp/lifestats-architecture-20261022/runtime.md)).

## Usage: caller first

Proposed application calls below use domain/client objects; transport DTOs stay behind codecs.

```text
// Phone manual entry, including process death after enqueue:
phoneDb.transaction(tx => sync.enqueue(tx, ManualObservation {
  operation: freshId(), fact: freshId(), statistic: steps,
  day: chosenDay, value: Count(12), occurredAt: optionalInstant
}));
await sync.flush(transport); // same persisted operation AND payload on every retry

// Logger replaces its authorized dataset's named days:
logger.submit(SourceDays {
  operation: savedId, stream: authorizedStream, epoch: grantedEpoch,
  days: [{day: monday, revision: 42, facts: []}] // retained clear
});

// Frontend reads authoritative goal resolution, then conditionally edits:
view = await client.dailyView(statistic, day); // cache + freshness + revision
sync.enqueue(tx, GoalEdit {
  operation: freshId(), statistic, day, expected: view.goalRevision,
  override: Suppressed
}); // conflict leaves an actionable draft; no automatic overwrite
```

A source record correction uses its stable record ID and expected revision; moving its activity day invalidates both days. The caller never loads repositories, builds aggregates, or coordinates checkpoint and daily-total writes.

## Two viable whole-system shapes

**H — strengthen horizontal layers.** Keep domain → application → adapters. Add a private application command coordinator, one canonical daily reader/evaluator, and a typed transactional persistence port. HTTP, source translators, and session commands call that coordinator; bootstrap exposes only the facade. Domain owns value/goal algorithms; application owns source reconciliation and cross-aggregate orchestration; persistence implements atomic storage. A manual write traverses adapter → coordinator → typed transaction, which loads canonical metadata and invokes domain rules. Existing CQRS handlers can remain internal helpers. This has the smallest migration footprint and a deep public facade, but source/goal/read invariants remain distributed across horizontal owners. A contributor adding a capability must change several layers and understand their shared protocol.

**C — capability-owned modular monolith, preferred.** Statistics owns definitions, source contributions, merge policy, goals/history, and canonical daily evaluation together. Progress owns bounded items, checkpoints, latest/furthest, and frontier deltas. Capture owns server session leases. Ingestion translates source vocabulary; its closed ebook command composes statistics and progress in one transaction. Delivery owns operation receipts and change cursors, without owning scoring rules. Each capability owns its commands, queries, pure decisions, and typed storage vocabulary; SQLite implements those vocabularies. Adding a new logger changes its translator/binding rather than domain repository choreography. This changes ownership and visibility, not just directories: only Statistics can construct its accepted state changes, and all daily views use its evaluation.

Both shapes support the semantics below with one database. C has greater internal interface depth: its daily-view boundary hides merging, temporal goals, missing-value treatment, and evaluation together. H is viable if minimizing immediate movement dominates; C is preferable because the next contributor can locate an invariant's complete owner.

## Preferred shape and boundaries

Proposed module map:

- `lifestats-core`: pure validated value objects and capability models; no infrastructure imports. Retain useful existing types and finite enums.
- `lifestats-application::{statistics,progress,capture,ingestion,delivery}`: private implementation modules; public `api` exports intentions and views only. No public repository fields. A capability change stays inside its owner plus the SQLite implementation of its typed port; shared delivery code does not acquire new scoring branches.
- Application-owned `storage_contract`: typed capability state/deltas and transaction interfaces; only SQLite adapter and bootstrap may depend on it. Transactions never cross into clients.
- `persistence`: private Diesel rows/codecs; `web_api`: versioned codecs/auth/error mapping; `plotting`: renders canonical views; bootstrap constructs them once.
- Remote logger clients and RN depend on generated contracts/client APIs, never bootstrap/persistence.

Crate dependency checks enforce that graph, including banning application → concrete plotting (currently present [5]). Rust privacy prevents importing capability internals; compile-fail probes exercise representative forbidden imports. The storage-contract dependency allowlist closes its deliberately public adapter seam. Typed deltas expose read-only accessors; only their capability owner constructs them. No blanket relocation of all traits into domain is needed.

```rust
// Proposed compact Rust sketch; private fields require validating constructors.
pub struct Operation { id: OperationId, actor: Actor, intent: Intent }
pub enum Intent {
    Observe(NewFact), Correct(ConditionalFactEdit),
    ReplaceDays(SourceDays), EditGoal(ConditionalGoalEdit),
    Checkpoint(NewCheckpoint), Reading(SourceReading), Session(SessionCommand),
}
pub enum Outcome {
    Applied(Receipt), Duplicate(Receipt), Unchanged(Receipt),
    Stale(CurrentVersion), Conflict(ConflictDetail),
}
pub enum Failure { Invalid(FieldErrors), Forbidden, NotFound, Gone,
    Unsupported(Capability), Retryable(RetryHint), Internal(IncidentId) }
pub enum Override<G> { Inherit, Suppressed, Explicit(G) }
pub enum Quantity { Count(CheckedCount), Duration(Seconds),
    Habit(bool), Clock(LocalClock) }
impl Lifestats {
    pub fn submit(&self, op: Operation) -> Result<Outcome, Failure>;
    pub fn daily_view(&self, q: DailyQuery) -> Result<DailyView, Failure>;
    pub fn progress_view(&self, q: ProgressQuery) -> Result<ProgressView, Failure>;
}
// Application-private workflow; adapter implements the typed port, no SQL exposed.
trait WriteStore {
    type Tx: ProductTx;
    fn transaction<T>(&self, f: impl FnOnce(&mut Self::Tx) -> Result<T>) -> Result<T>;
}
trait ProductTx: StatisticsTx + ProgressTx + DeliveryTx {}
trait StatisticsTx {
    fn load(&mut self, id: StatisticId) -> Result<StatisticsState>;
    fn apply(&mut self, delta: StatisticsDelta) -> Result<()>;
}
trait ProgressTx {
    fn load_item(&mut self, id: ItemId) -> Result<ProgressState>;
    fn apply_progress(&mut self, delta: ProgressDelta) -> Result<()>;
}
trait DeliveryTx {
    fn receipt(&mut self, key: OperationKey) -> Result<Option<BoundReceipt>>;
    fn finish(&mut self, receipt: BoundReceipt, changes: ChangeScopes) -> Result<()>;
}
fn execute(tx: &mut impl ProductTx, op: Operation) -> Result<Outcome> {
    // Verify identity/payload receipt, authority, versions; load canonical target.
    // Pure capability decisions -> typed deltas; persist all effects and receipt.
    // Allocate change cursor with affected scopes; commit before replying.
    todo!()
}
```

### Delivery, authority, and corrections

Operation identity is `(actor, operation_id)` with immutable schema version and canonical payload digest, including preconditions, source scope, and epoch. A matched receipt returns the original outcome even after later edits. Same ID/different payload is `operation_payload_mismatch`, never success. Committed terminal stale/conflict outcomes are receipted too; transient failures are retried unchanged. Compact receipts and deletion high-water marks survive indefinitely initially; any future epoch retirement must explicitly reject old operations rather than replay them as new.

Bindings uniquely identify `(source, dataset, metric)` and map to preserved statistic IDs, independently of display tags. Authorized device identity does not establish dataset authority. A snapshot dataset has one publisher per granted epoch; transfer fences the former publisher. Distinct device datasets can contribute independently. Shared replicated source datasets use common record identities or conditional shared revisions, avoiding double counting under invented per-device identities. Boundary credentials map to allowed bindings; this is single-person authorization, not tenancy.

Each named snapshot day carries a monotonic revision. Lower revisions are **stale**; equal revision/equal content is **unchanged**; equal revision/different content is **conflict**. A duplicate specifically means replaying the same operation receipt. Empty days retain revisioned tombstones; omission means untouched. Bounded multi-day commands commit all or nothing, including moves and clears. Any failed member rejects the batch. New payloads require new operation IDs; a queued but unattempted snapshot may be coalesced, an attempted operation may not.

Manual facts use stable IDs; edits/deletes require expected fact revisions. Goals require expected goal revisions, including inherited states: the version binds both override and applicable default schedule. Different source facts remain separate: correcting A rebuilds affected views from A's replacement plus B's retained contributions. Metric definitions choose additive counts, interval union, or exclusive-source scalar authority; arbitrary overwrite/arrival time is not a merge rule. Deleted statistics retain nonreusable identity/tombstones and return `Gone` to stale writers.

### Transactions and durable state

Add typed tables for bindings, facts/snapshots and revisions, tombstones, goal periods/overrides, operation receipts, capture leases, and cache changes. Preserve legacy tables as compatibility projections. Unique binding/record keys, foreign keys, and conditional version writes enforce invariants. Reconcile existing duplicates/orphans on copies before constraints.

One SQLite write transaction encloses receipt lookup, target/authority validation, capability decisions, source mutations, affected aggregate/progress projections, revision changes, receipt, and cursor allocation. Cross-capability ebook ingestion becomes atomic rather than acknowledging only its first effect. SQLite serializes writers; bounded busy retries rerun the whole transaction. No network fetch occurs inside it. Reads use a consistent snapshot. Unchanged/rejected operations persist outcomes without inventing data invalidations.

The durable change log contains invalidation scopes/deletions, not replayable business history. It is not event sourcing. Compute views synchronously initially. Add no background projection/event queue without an actual required consumer; existing notifications are no-op in bootstrap [6]. Existing Fitbit/Clef timers can remain collectors. Fitbit credential refresh needs per-binding lease/CAS ownership, and fetched absence must distinguish unavailable from an authoritative deletion.

### Time, goals, quantities, and capabilities

Keep activity day separate from source occurrence instant, offset/zone, and server receipt time; reject inconsistent attribution according to the binding's declared policy rather than trusting arbitrary device claims. Explicit source attribution wins; timezone conversion cannot silently move bedtime evenings or LiftLog days. New intervals require ordered instants and a declared day-splitting/elapsed policy; compatibility codecs preserve existing wall-clock/clamping/minimum-duration behavior [3]. Constructors and storage decoders validate ordering, bounds, checked arithmetic, and normalized interval union consistently across every read. Separate bounded efficiency `0..100` from nonwrapping achievement ratios permitting overachievement; retain zero-target at-least success and zero-observation at-most success. Nonnegative metrics reject negative observations/targets; signed quantities require explicit metric policy. Use checked wide intermediates, never byte casts.

Preserve goal `Inherit | Suppressed | Explicit` [7]. Default changes create effective-dated periods with explicit start day; clearing closes a period rather than erasing prior applicability. Backdated edits require conditional revisions and invalidate the affected interval. Preserve type-specific missing-value rules, no-goal exclusions, and progress latest/furthest/frontier semantics. Progress effective-day queries must not implicitly filter late imports by receipt date; expose `as_of` separately.

Definitions expose units, supported writes/goals, and read capabilities. Derived types are currently incomplete [8]; advertise unavailable until definitions round-trip, references/units/cycles/missing-data policies are validated, and canonical reads actually compute them. Frontends never fill that gap themselves.

### Contract and client sync

A versioned Rust wire-contract module is authoritative for generated OpenAPI/JSON Schema, TS types/runtime validators, and client fixtures; it is separate from domain types. Encode int64 quantities as decimal strings with explicit units to avoid JavaScript precision loss. CI regenerates and checks drift plus real adapter response conformance. One error codec covers extractors and application outcomes: invalid/unsupported, forbidden, not-found/gone, stale/conflict, retryable with retry-after. For example:

```json
{"operation_id":"o17","kind":"edit_goal","statistic_id":12,"day":"2026-10-22","expected_revision":7,"override":{"kind":"suppressed"}}
{"kind":"conflict","code":"revision_mismatch","expected":7,"actual":8,"retryable":false}
{"operation_id":"o18","kind":"replace_days","binding_id":"liftlog-phone","epoch":3,"days":[{"day":"2026-10-22","revision":42,"facts":[]}]}
{"kind":"applied","receipt":{"operation_id":"o18","cursor":"g4:91","revisions":{"2026-10-22":42}}}
```

Plain transport-neutral sync code accepts transport/storage/clock adapters; it owns retry classification, immutable envelopes, receipt handling, and reconciliation. RN SQLite owns transactional enqueue, payloads, retry state, blocked conflicts, cache rows/watermarks, and acknowledgments. UI drafts/pending overlays remain explicitly provisional. No authoritative score calculations run offline.

Cache keys include server generation, schema, normalized query scope, statistic IDs, and date range. Snapshot reads carry watermarks. A scoped changes feed reports invalidations for old/new days, goals, definitions, deletions, and derived dependents, plus a scan-through cursor even when filtering yields nothing. Fetch fresh affected views, then atomically install cache updates and cursor; discard older racing responses. Later read watermarks may cause harmless repeated invalidations. Expired cursor/generation mismatch forces scope refetch and deletion reconciliation, preserving the independent write outbox. Time-dependent views carry server time/expiry and refresh on activity-day rollover. Show last refresh and pending state. Cache-log retention never determines receipt retention.

## Migration and acceptance gates

1. **Specify/freeze:** capture copied historical fixtures and legacy response examples, IDs, units, receipts, goal tri-state, progress order, and frozen LiftLog `total_weight` [9]. Gate: reproducible baseline plus explicit expected changes for reproduced defects.
2. **Add storage/write boundary:** additive migrations, typed validation, receipts/versions/change log, shadow comparisons. Import daily aggregates as opaque legacy baselines, not invented source facts. Preserve existing JSON/output; historical corrections require explicit baseline reconciliation where provenance is lost. Import only known goal applicability; label missing older history unknown. Fence cutover by scope: legacy aggregate replacement and new source merging cannot both own a cell. A verified exclusive snapshot binding can adopt/replace an opaque baseline atomically; ambiguous mixed histories remain opaque until explicit reconciliation. Compatibility adapters preserve replace/clear semantics before that cutover. Gate: copied fixtures preserve data/IDs; defects have checked corrected outcomes.
3. **Future proof slice:** isolated migrated copies, two independent clients, real Rust concurrent transactions/processes; response-loss retry, crash before/after commit, payload mismatch, stale goal/day clear, authority transfer, A correction preserving B, ebook rollback, cursor expiry, mobile/server-session restart. Gate: demonstrated durable outcomes and unchanged compatibility fixtures. This is specified work, not completed verification.
4. **Adopt consumers:** generated SDK and web UI first, RN durable outbox, versioned logger adapters. Keep existing route/wire shapes and integration metric keys. Legacy routes enter the same boundary but cannot promise replay/order safety absent client identities. Gate: installed caller inventory, contract checks, and explicit legacy limitations; no unsupported multi-device guarantee.
5. **Retire competing writers:** migrate MIDI and any deployed embedded writers to transport, verify deployed artifacts, then remove raw/bootstrap access and schema-migration privileges from loggers. Gate: dependency/visibility failures for bypass imports and deployment evidence of server-exclusive writes. Retire compatibility paths only after all actual callers migrate.

## Rationale, alternatives, and screening

**Synthesis decision:** within this independent package choose C, retaining H's incremental adapter compatibility; parent owns cross-candidate synthesis.

Accept capability reorganization for locally comprehensible ownership; accept synchronous bounded transactions for simple atomicity; accept explicit stale-write resolution for honest offline behavior.

An independently deployed ingestion service with its own queue/database could isolate vendor outages and collection schedules, but would expose acceptance-versus-application states and split ebook transactions. Existing independently scheduled collectors already isolate collection; no demonstrated independent authoritative lifecycle justifies the extra service. Reconsider for sustained vendor processing or mandatory crash isolation. Full local replication/event sourcing adds conflict/computation responsibilities outside the confirmed requirement.

Red-flag screen: no caller repository choreography, transport re-exports, public raw mutations, parallel legacy/new authoritative paths after cutover, or hand-maintained policy lists. Private capability modules prevent import shortcuts; generated contracts and dependency gates catch drift. Keep the closed coordinator for actual cross-capability operations, not a generic plugin framework.

**Unknowns:** Which installed legacy writers remain? Which datasets are shared across devices? What historical baseline reconciliation is acceptable? Should server timing survive restart? Proposal: persist bounded server heartbeat leases and recover only through last durable beat, never count downtime; offline capture submits completed source interval facts, never replays delayed heartbeats as live activity. These are separate APIs/entities, preserving existing session compatibility during migration. Capture leases require an authorized interval-capable target, session ID, actor, generation, deadline, and increasing beat sequence. Each accepted beat atomically persists its endpoint and projected interval. Expired beats return `session_expired`; stop is idempotent. Neither delayed traffic nor restart credits unobserved downtime.

**First implementation step:** after parent synthesis, build the isolated manual-observation/source-day transaction proof against copied fixtures, before moving folders or connecting RN.

## Source citations

Paths are rooted at `/home/daniel/.local/share/src/lifestats`; claims above concern inspected source, not deployed incidence.

[1] [log handler:56,164](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/commands/log_value/handler.rs:56).
[2] [ebook effects:331–341](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/service.rs:331).
[3] [LiftLog replace/clear:349–390](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/service.rs:349); [normalization:129](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/service.rs:129); [compatibility contract](/home/daniel/.local/share/src/lifestats/docs/liftlog-integration.md:12).
[4] [MIDI bootstrap and session:26,58](/home/daniel/.local/share/src/lifestats/logger_applications/midi_logger/src/main.rs:26).
[5] [concrete rendering dependency](/home/daniel/.local/share/src/lifestats/core/application/Cargo.toml:10).
[6] [no-op publisher](/home/daniel/.local/share/src/lifestats/lifestats-bootstrap/src/lib.rs:32).
[7] [goal tri-state](/home/daniel/.local/share/src/lifestats/core/infrastructure/web_api/src/dto/goal.rs:49); [separate default saves](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/commands/set_default_goal/handler.rs:133).
[8] [derived persistence gap](/home/daniel/.local/share/src/lifestats/core/infrastructure/persistence/src/db/mappers.rs:97); [grounded computation search](/tmp/lifestats-architecture-20261022/domain.md).
[9] [frozen weight metric](/home/daniel/.local/share/src/lifestats/core/application/src/use_cases/integration/liftlog_bundle_factory.rs:48).
