# Proposed application boundary

Status: design proposal, not implemented. This sketch selects Candidate A's
in-transaction execution model within capability-owned modules. It adopts
Candidate B's scope-aware cache protocol and installation/authority fencing.
The independent judge's findings and the final response to them are retained
under `evidence/`.

Review status, October 23: the [Claude review](evidence/claude-review.md)
challenges this draft's sync complexity and incomplete storage/API specification.
The [parent assessment](evidence/claude-review-response.md) identifies changes to
evaluate and qualifications. Protocol choices below remain under review.

Prototype follow-up: the [executed comparisons](prototypes/2026-10-23/README.md)
and [working decisions](prototypes/2026-10-23/prototype-decisions.md) now favor
explicit contributor kinds, stable client operation identity, and complete
cached-scope refreshes initially. These revise the generation-bound receipt,
mandatory change-feed and blanket decimal-string choices below. Controlled
restore must still mint a new generation before serving, and numeric product
ranges and API compatibility remain explicit design decisions. The original
sections are retained as the earlier proposal, not an implemented contract.

## Caller's usage

A phone stores a typed intent and an operation ID atomically with pending UI
state. Transport translates the intent into versioned DTOs. Retrying a send
uses the same ID and immutable semantic payload. A logger submits only its own
authorized dataset. A frontend reads an authoritative goal view and edits
against its returned version.

```text
local.transaction(tx => enqueue(tx, AddObservation {
    operationId, factId, statisticId, effectiveDate,
    value: Amount { decimal: "5", unit: Count }
}));

submit(ReplaceSourceDays {
    operationId, streamId, authorityEpoch,
    days: [{ date, sourceRevision: "42", intervals: [] }]
});

view = dailyView(statisticId, date);
submit(EditGoal {
    operationId, statisticId, date,
    expectedGoalVersion: view.goalVersion,
    override: Suppressed
});
```

These are illustrative calls, not the current API. An integer amount is a
validated decimal string on the wire to preserve all signed 64-bit values in
JavaScript. All 64-bit wire fields, including goal values and revisions, follow
that encoding. Cursors remain opaque. Internally it is a checked numeric type with a declared unit and
signed/nonnegative policy.

## Ownership and imports

- Domain capability models retain pure value, interval, goal, and checkpoint
  decisions. Statistics owns definitions, source contributions, goal history,
  merge policy, and canonical daily evaluation. Progress owns bounded objects,
  checkpoints, latest/furthest, and frontier deltas.
- Application capability modules expose intents and views. A closed ingestion
  coordinator composes real cross-capability operations such as ebook reading.
  Shared delivery code owns receipts and invalidation, not scoring rules.
- An application-owned storage contract exposes only the typed state/delta and
  transaction vocabulary required by the SQLite adapter. An import allowlist
  permits application, SQLite implementation, and bootstrap access. It is not
  a public interface for loggers or HTTP handlers.
- SQLite rows, Diesel, JSON storage codecs, and connections remain adapter
  internals. HTTP imports public operations and transport codecs. Rendering
  consumes canonical views. Bootstrap alone wires concrete implementations.
- Capability internals are private. Compile-fail/import checks reject clients
  importing repositories, forbidden adapter dependencies, and application
  imports of concrete plotting. All independently deployed loggers lose local
  bootstrap/database access before server-exclusive writes are claimed.

This can begin inside the current crates. It does not require a new crate for
every capability or handler, or moving every trait into the domain.

## Plot images are an optional presentation frontend

The user clarified that Obsidian at `/home/daniel/main/obsidian` is the main
current frontend. It embeds the polished plot images originally made for a
desktop wallpaper. React Native will use interactive charts, and image plotting
may eventually be removed. This clarification strengthens the presentation
boundary; it does not introduce another product capability.

Separate three responsibilities:

1. Product queries return typed daily values, units, normalized intervals,
   applicable goals, achievement results, and missing/no-goal distinctions.
   Aggregation and goal calculations stay authoritative here. Queries accept
   semantic filters such as date range, not chart types or image dimensions.
2. Each frontend converts these results into its own chart model. Colors, axis
   bounds, ticks, labels, dimensions, chart type, zoom, tooltips, and visual
   downsampling belong to presentation. A presentation model may be shared
   within one frontend, but it is not a universal graphics contract for all
   clients.
3. An optional static-image frontend retains the existing Plotters styling and
   PNG output for Obsidian and any wallpaper caller. It reads the same public
   product queries as other presentation clients. It has no repositories or
   independent achievement evaluator.

The static frontend can remain in the server process. Mount its existing image
routes in the composition root, with its own presentation state and renderer
dependencies. It does not require a new microservice or an HTTP loopback call.
The product domain/application packages neither import plotting nor define a
`PlotRenderer` port. Any rendering interface belongs inside the optional
presentation package. Core API construction/tests must work without that
package enabled.

The current `/statistics/{id}/plot/data` response includes colors, chart type,
and axis configuration. Treat it as part of the image frontend's compatibility
API, not the canonical data contract for React Native. Preserve existing
`/statistics/{id}/plot/image` and `/plots/timeline/{date}` embeds while moving
their implementation. Obsidian's `scripts/refreshableImage.js:38` and
`scripts/showTimeline.js:13` confirm these live consumers. No vault files or
routes change in this design milestone.

Retirement is successful when deleting the image frontend, its routes, assets,
and rendering dependencies requires no product storage migration and no changes
to mobile queries or business rules. Reuse genuine units/time/value types from
the current plotting code in product models where appropriate; keep drawable,
RGB, axis, and image configuration types in presentation. The existing
`plot-domain` name does not establish product ownership.

Before migration, capture representative image fixtures to preserve the current
polish. Review any visual changes caused by deliberate corrections to product
calculations separately. This is a proposed migration gate, not completed image
regression verification.

## Type and signature sketch

### App-wide wire contract and the Yomikata reference

The user identified `/home/daniel/.local/share/src/lifestats-yomikata-api` as a
recent API redesign. The October 22 comparison inspected its working-tree
changes without editing that checkout or running its tests. Since that
comparison, the redesign landed on main in commit `ea68f0c`, after the original
review baseline. On October 23 the separate checkout is absent. The paths below
now refer to main. This establishes source integration, not deployment.

The new `PUT /integrations/yomikata/days/{day}` accepts typed
`cards_reviewed` and exact `study_time_seconds`, rejects invalid/unknown fields,
returns a structured code/field/message error, and succeeds only after the
atomic operation commits. Provisioning, legacy adoption, the source ledger,
and both statistic projections share that transaction. Source replacement
preserves other aggregate residuals rather than blindly overwriting them.

Its `utoipa` declarations generate `/openapi.json` from the transport DTOs and
handler annotation. That document explicitly covers only Yomikata. Source tests
check required fields, types/bounds, unknown-field policy, response codes, and
error shape; separate tests exercise concurrent persistence. These tests were
inspected, not rerun here. This is stronger grounding for the schema-generation
plan than selecting another tool without a need.

Generalize these rules across the public API:

- Define explicit requests/responses per operation. Replace stringly typed
  value/policy and goal maps with typed variants at the new wire boundary.
- Use transport DTOs and handler declarations as the schema authority, isolated
  from database rows and domain implementation types. Compose per-capability
  documents into one app-wide contract and keep legacy APIs distinguishable.
- Use one public error envelope and shared framework-rejection mapping, with
  operation-specific codes. Clients branch on codes and retry metadata rather
  than parsing message text.
- Generate TypeScript types and runtime codecs from that contract. Migrate the
  web UI and shared SDK onto those artifacts, then build RN on the same client.
- Check route/schema coverage, generated-output drift, and real adapter/consumer
  fixtures. Schema generation alone does not prove parser or behavior agreement.

Generalize the contract discipline, not Yomikata's exact endpoint shape. Source
snapshots, additive observations, conditional goal edits, progress checkpoints,
and read queries keep their own explicit semantics. The internal `submit(Intent)`
sketch does not require a single catch-all HTTP endpoint or arbitrary
client-authored transaction language.

Two app-wide choices remain beyond the reference endpoint. First, replaying the
same daily PUT is idempotent, but different snapshots still need the planned
source revision/authority policy to reject late stale content. Second, Yomikata
currently accepts JSON integers through full i64 range; the proposed mobile
contract uses decimal strings for 64-bit values. Resolve this wire difference
explicitly, including schema generation and client round-trip fixtures, rather
than silently changing the existing endpoint.

Reference paths on main:

- `core/infrastructure/web_api/src/routes/yomikata.rs:22` - request and validation.
- `core/infrastructure/web_api/src/routes/yomikata.rs:62` - structured errors.
- `core/infrastructure/web_api/src/routes/yomikata.rs:163` - endpoint schema and documentation.
- `core/infrastructure/web_api/src/routes/yomikata.rs:208` - scoped OpenAPI document.
- `core/infrastructure/persistence/src/yomikata_day_store.rs` - atomic source replacement.
- `core/infrastructure/web_api/tests/yomikata_days_e2e.rs:401` - schema contract assertions.

### Public application operations

The following is pseudocode for interfaces, not a compiled implementation.
Newtypes have private fields and validating constructors. The HTTP adapter
derives `AuthorizedActor` from the deployment identity boundary; callers cannot
self-assert permission by sending a source string.

```rust
pub struct Operation {
    id: OperationId,
    actor: AuthorizedActor,
    installation: InstallationGeneration,
    intent: Intent,
}
pub enum Intent {
    AddObservation(NewFact),
    CorrectObservation(ConditionalFactEdit),
    ReplaceSourceDays(AuthorizedSourceDays),
    EditGoal(ConditionalGoalEdit),
    RecordCheckpoint(NewCheckpoint),
    RecordReading(SourceReading),
    DeleteStatistic(ConditionalDeletion),
}
pub enum GoalOverride<G> { Inherit, Suppressed, Explicit(G) }
pub enum Outcome {
    Applied(Receipt), Duplicate(StoredOutcome), Unchanged(Receipt),
    Stale(CurrentSourceRevision), Conflict(ConflictDetail),
}
pub enum Failure {
    Invalid(FieldErrors), Unsupported(Capability), Forbidden,
    NotFound, Gone, Retryable(RetryHint), Internal(IncidentId),
}
impl Application {
    pub fn submit(&self, operation: Operation) -> Result<Outcome, Failure>;
    pub fn daily_view(&self, scope: DailyScope) -> Result<Snapshot<DailyView>, Failure>;
    pub fn progress_view(&self, scope: ProgressScope) -> Result<Snapshot<ProgressView>, Failure>;
    pub fn changes(&self, scope: ReadScope, after: Cursor) -> Result<ChangePage, SyncError>;
}

// Internal storage vocabulary, implemented by a server-owned adapter.
trait AtomicStore {
    type Tx: ProductTransaction;
    fn write<T>(&self, operation: impl FnOnce(&mut Self::Tx) -> Result<T>) -> Result<T>;
}
trait ProductTransaction: StatisticsTransaction + ProgressTransaction + DeliveryTransaction {}
trait StatisticsTransaction {
    fn load(&mut self, id: StatisticId) -> Result<StatisticsState>;
    fn apply(&mut self, delta: StatisticsDelta) -> Result<()>;
}
trait ProgressTransaction {
    fn load(&mut self, id: ProgressItemId) -> Result<ProgressState>;
    fn apply(&mut self, delta: ProgressDelta) -> Result<()>;
}
trait DeliveryTransaction {
    fn find_receipt(&mut self, key: OperationKey) -> Result<Option<BoundReceipt>>;
    fn finish(&mut self, receipt: BoundReceipt, changes: ChangedScopes) -> Result<()>;
}
```

The public operation hides target loading, type checks, source binding,
normalization, replay, atomicity, and invalidation. It never accepts a preloaded
statistic or asks a caller to coordinate persistence methods. Cross-capability
operations are explicit enum variants, not a plugin registry or arbitrary
client-authored transaction plan.

## One atomic execution mechanism

Use a bounded SQLite immediate write transaction. Inside it, check a matching receipt,
load canonical target/binding/revision state, run pure capability decisions,
apply typed changes, persist the result receipt, and allocate change records.
No network access occurs inside the transaction. Busy retries rerun the entire
transaction. Failed commits leave neither authoritative effects nor acceptance
receipts. Bounded source batches commit all named days or none.

The first implementation does not load state outside the transaction and then
validate a versioned read set. Candidate B's optimistic planning is viable, but
requires range/phantom versions and re-planning without an established benefit
for this workload. Reconsider only after real transaction pressure is measured.

## Replay, replacement, and conflict semantics

Use `(installation generation, operation ID)` as the durable operation key.
Bind the authenticated actor, destination, source scope, schema version,
preconditions, and canonical semantic payload. Store canonical payload identity
alongside its digest. Matching identity/payload returns the original outcome;
changed-content reuse returns `operation_payload_mismatch`. Duplicate is receipt
replay, not merely equal current content.

Keep compact receipt identities for the installation lifetime initially.
Cache-change retention is separate. Any future receipt retirement explicitly
rejects expired identities rather than treating them as new. The restore procedure must mint a fresh installation generation, even when
restored metadata contains the old generation. Restoring a backup
therefore creates an installation generation change: clients pause ambiguous writes and
reconcile. They do not blindly mint IDs and replay work whose prior acceptance
may have been lost with the backup. Restoration cannot recover missing receipts
or overwritten source history by protocol design alone.

Snapshot authority is a server-granted `(stream, authority epoch)` with one
publisher. Each day has a persistent source revision. A higher source revision
applies, a lower one is stale, equal revision/equal payload is unchanged, and
equal revision/different payload conflicts. A clear retains its revision and
tombstone. Omitted days stay untouched. A failed member rejects the bounded
batch. Counters survive local restart and acknowledgement. Device restore or
publisher transfer requires a new granted epoch that fences prior publishers.
Before enabling a restored or transferred publisher, reconcile accepted server
days and clear tombstones. A new epoch fences senders but does not prove the
new publisher content is current. An intentional authoritative reset requires
an explicit recorded decision. Mirrored copies of the same dataset cannot
become additive sources by inventing different device IDs.

Manual record corrections, deletions, and user goal edits use expected server
versions. Goal versions bind the relevant override and applicable default
schedule, including inheritance. A stale user edit remains a visible conflict.
Source snapshot revision and expected server version are deliberately different
protocols; clients never interchange them.

## Reads, history, and ownership after migration

Snapshot data and watermark share a read transaction. Caches key installation
generation, schema/query version, normalized scope, and parameters. Each scope
has its own cursor. Changes include all affected old/new days, goals, progress,
deletions, and implemented derived dependents, plus a scan-through cursor even
when no matching change is returned. Page scans fix their upper watermark and advance on empty matching pages.
Start with bounded complete scope snapshots. Larger scopes split explicitly;
a future paginated snapshot needs a pinned snapshot token or restart rule.
Install updates and cursor atomically, and discard older racing responses.

Cursor expiry requires a replacement snapshot that reconciles deletions while
preserving the separate command queue. Scope registration first obtains a
snapshot; consuming another scope's feed cannot advance an uncached scope.
Time-sensitive views also carry server time/expiry and refresh at day rollover.
An invalidation log is not business event sourcing and needs no message broker.

New source facts or snapshots remain distinct enough to correct source A while
retaining B. Metric definitions select checked sum, interval union, or one
authoritative register. Source adapters choose activity attribution; canonical
product code owns validation, units, and calculation. Do not combine complete
snapshots of the same dataset as independent contributions.

Keep existing aggregates as opaque legacy baselines. A known exclusive source
may take over a reconciled scope atomically. An ambiguous mixed baseline stays
opaque until explicitly reconciled. Never add a full source backfill atop its
unknown prior contribution. Fence old aggregate writers before a scope begins
new contribution merging. Do not enable those live scopes until old direct
database writers retire. Legacy requests without stable operation IDs retain
their existing replay limitations even when routed through the new coordinator.

Default goals become effective-dated goal-or-none history, with explicit
retroactive corrections. Preserve surviving defaults/overrides and label missing
history unknown. Separate efficiency from achievement and validate interval
decoders. Unsupported derived capabilities remain unavailable until storage,
references, cycles, units, missing-data rules, and canonical reads work together.

Server heartbeat leases are a separate follow-up product decision. The new
offline contract submits durable source interval facts; it does not replay old
heartbeats. Before promising durable live capture, specify session identity,
fencing, acknowledged endpoint sequence, expiry, and restart behavior that
never credits unobserved downtime. Existing session consumers need explicit
migration evidence before their protocol changes.

## Tradeoffs and next implementation step

Accept synchronous bounded transactions and retained operation identities for
clear correctness. Accept visible stale-edit resolution instead of silent
last-arrival wins. Keep independently scheduled collectors, but do not split
authoritative product state into microservices without an isolation need.

The next implementation unit fixes the reproduced core defects with behavioral
regressions and establishes transport fixtures. The next architecture proof
implements one observation and one source-day replacement using two consumers
against isolated storage, including commit/response loss, concurrent writes,
stale clear, source correction, and restart.
