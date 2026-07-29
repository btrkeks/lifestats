# Progress Items Architecture ADR

## Status

Proposed

## Date

2026-05-02

## Context

LifeStats currently models statistics as daily log values. The central persistence shape is:

```text
logging_data(statistic_id, date) -> JSON value
```

This works well for daily observations such as pushups, sleep duration, habits, time spans, and
point-in-time values. It is not a natural source-of-truth model for progress through a bounded
object, for example:

- current page in a book
- current character offset in an ebook
- lesson number in a course
- level or mission progress in a game
- page count in a long document

For those use cases, the primary fact is not "what value belongs to this date?" The primary fact is
"where was this tracked object at this moment?" Daily progress, pace, streaks, and completion
percentages are useful derived views, but they should not be the canonical representation.

The existing ebook integration already records a useful global daily metric:

```text
[Ebook] Characters Read -> IntegerAmount, summed per day
```

That path should remain. It answers "how much did I read today?" A progress item answers a
different question: "where am I in this specific book?"

## Decision

Introduce a new bounded context named `progress`, centered on `ProgressItem` and
`ProgressCheckpoint`.

Do not add `AnyStatistic::Progress` in v1.

Progress items are sibling domain objects beside statistics. They have their own repository port,
application commands, application queries, SQLite adapter, and HTTP routes. Existing daily
statistics remain unchanged.

Implementation should use one small enabling refactor before feature work, then proceed vertically:

1. Move the generic repository error to a neutral shared port module and re-export it from the old
   statistic path.
2. Implement the progress feature through domain, application, persistence, and API layers.
3. Refactor after implementation only where the new code reveals concrete duplication or awkward
   boundaries.

Do not start with a broad cleanup of `AnyStatistic`, `LogValueCommand`, goals, dashboards, or
plotting. Progress should route around the daily statistic path rather than generalizing it upfront.

The source of truth for a progress item is an append-only sequence of absolute checkpoints:

```text
ProgressItem: The Left Hand of Darkness
Target: 304 pages

Checkpoints:
  2026-05-01T21:14:00Z, effective_date=2026-05-01, position=17
  2026-05-02T22:03:00Z, effective_date=2026-05-02, position=42
  2026-05-04T18:30:00Z, effective_date=2026-05-04, position=88
```

The system derives daily deltas from checkpoints when needed. The daily delta algorithm should use
a furthest-frontier model by default:

```text
frontier = highest position observed before the current checkpoint
delta = max(0, checkpoint.position - frontier)
frontier = max(frontier, checkpoint.position)
attribute delta to checkpoint.effective_date
```

This means:

- logging page 50 after page 40 adds 10 pages of new progress
- logging page 30 after page 50 records the current position but does not subtract progress
- logging page 45 after page 50 adds no new progress
- logging page 60 after page 45 adds 10 pages of new progress

The query layer should expose both latest position and furthest position. UI surfaces can choose
which one is appropriate for the workflow.

## Non-Goals

The v1 implementation should not:

- replace existing daily statistics
- migrate existing `logging_data`
- add progress into `AnyStatistic`
- add daily goals, default goals, or streak evaluation for progress items
- make progress items appear in `/statistics`
- rewrite the current ebook character-sync path
- build a complex progress dashboard before the domain and persistence model are stable
- add delete endpoints for progress items or checkpoints
- support changing a progress item's unit after creation
- publish progress events

## Core Semantics

### Source of Truth

The canonical value is an absolute checkpoint:

```rust
position_value: i64
```

The value must be non-negative. It is absolute within the item, not a delta.

For a book, "I am on page 73" is canonical. "I read 31 pages today" is derived.

### Target

Every v1 progress item has a required bounded target:

```rust
target_value: i64
```

The target must be positive.

Examples:

- `304 pages`
- `581233 characters`
- `100 percent`
- `18 chapters`

If the target is initially unknown, the caller should wait to create the progress item or create it
with the best known target and update it later. Supporting unbounded progress can be considered in
a later version, but v1 should keep the model bounded.

### Unit

Progress units are typed enough for common UI and query semantics, while still allowing custom
units:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ProgressUnit {
    Pages,
    Characters,
    Percent,
    Chapters,
    Custom { label: String },
}
```

For `Percent`, the recommended target value is `100`.

This serde representation is part of the public API contract. It produces JSON such as:

```json
{ "type": "Pages" }
```

and:

```json
{ "type": "Custom", "label": "lessons" }
```

Persist the same semantic shape as `unit_kind` and optional `unit_label` in SQLite. Do not invent a
second custom-unit encoding in the web API or persistence adapter.

### Position Bounds

Commands should validate:

- `position_value >= 0`
- `target_value > 0`
- `position_value <= target_value`

If the target was wrong, callers should update the target before logging a checkpoint beyond it.
This keeps invalid states out of the core model while still allowing correction.

### Time and Dates

Each checkpoint stores both:

- `recorded_at`: an absolute UTC timestamp for ordering
- `effective_date`: the local LifeStats date used for daily grouping

This mirrors the existing system's explicit date-based logging while avoiding timezone ambiguity.
The adapter, client, or application command caller is responsible for choosing `effective_date`.

For local manual logging, `effective_date` will usually be the user's current local date. For
imports, it should be the date that the source system attributes the progress to.

### Latest vs Furthest

Progress summary queries must distinguish:

- `latest_position`: position from the most recent checkpoint by `recorded_at`
- `furthest_position`: maximum position ever observed
- `percent_latest`: `latest_position / target_value`
- `percent_furthest`: `furthest_position / target_value`
- `is_complete`: `furthest_position >= target_value`

This matters because a user may move backward in a book to reread a section. The latest checkpoint
should preserve that fact without erasing completed progress.

## Domain Design

Add a new module under the core crate:

```text
core/core/src/progress/
  mod.rs
  error.rs
  item.rs
  checkpoint.rs
  unit.rs
  ports/
    mod.rs
    progress_repository.rs
```

Expose it from:

```text
core/core/src/lib.rs
```

### Identifiers

Add a new ID type in `core/core/src/shared/ids.rs`:

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProgressItemId(pub u32);
```

Implement `From<u32>` and `Display`, matching `StatisticId`.

### Value Objects

Recommended domain structs:

```rust
pub struct ProgressItem {
    id: ProgressItemId,
    metadata: ProgressMetadata,
    target: ProgressTarget,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct ProgressMetadata {
    name: String,
    category: String,
    tags: HashMap<String, String>,
}

pub struct ProgressTarget {
    value: i64,
    unit: ProgressUnit,
}

pub struct ProgressCheckpoint {
    id: Option<ProgressCheckpointId>,
    item_id: ProgressItemId,
    recorded_at: chrono::DateTime<chrono::Utc>,
    effective_date: chrono::NaiveDate,
    position: i64,
    source: Option<String>,
    source_event_id: Option<String>,
    note: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}
```

If adding `ProgressCheckpointId` feels too much for v1, the repository can keep the row ID internal.
However, exposing it will make future editing/deletion/debugging easier.

### Constructors and Validation

Keep validation in pure domain constructors:

```rust
impl ProgressItem {
    pub fn new(
        id: ProgressItemId,
        metadata: ProgressMetadata,
        target: ProgressTarget,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> DomainResult<Self>;
}

impl ProgressCheckpoint {
    pub fn new(
        item_id: ProgressItemId,
        recorded_at: DateTime<Utc>,
        effective_date: NaiveDate,
        position: i64,
        source: Option<String>,
        source_event_id: Option<String>,
        note: Option<String>,
        created_at: DateTime<Utc>,
    ) -> DomainResult<Self>;
}
```

Validation rules:

- names must not be empty after trim
- categories must not be empty after trim
- target value must be positive
- checkpoint position must be non-negative
- source and source event IDs must be non-empty after trim when present
- `source_event_id` requires `source`
- note may be empty, but empty notes should be normalized to `None`

The command handler should additionally validate `checkpoint.position <= item.target.value`, because
that requires loading the item.

## Shared Repository Error

The existing repository error type is generic, but it currently lives at:

```text
core/core/src/statistic/ports/error.rs
```

Before adding `ProgressRepository`, move the canonical definition to a neutral path:

```text
core/core/src/shared/ports/error.rs
```

Then re-export it from the old statistic path to avoid broad churn:

```rust
pub use crate::shared::ports::error::RepositoryError;
```

Progress ports should import the neutral path:

```rust
use crate::shared::ports::error::RepositoryError;
```

This keeps the new progress bounded context from depending on statistic internals while preserving
the public contract used by existing statistic repository code.

## Repository Port

Define the port in the domain layer:

```rust
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait ProgressRepository {
    fn create_item(&self, item: NewProgressItem) -> Result<ProgressItem, RepositoryError>;

    fn update_item(&self, item: ProgressItem) -> Result<(), RepositoryError>;

    fn find_item_by_id(
        &self,
        id: ProgressItemId,
    ) -> Result<Option<ProgressItem>, RepositoryError>;

    fn find_all_items(&self) -> Result<Vec<ProgressItem>, RepositoryError>;

    fn find_items_by_tags(
        &self,
        tags: &HashMap<String, String>,
    ) -> Result<Vec<ProgressItem>, RepositoryError>;

    fn save_checkpoint(
        &self,
        checkpoint: NewProgressCheckpoint,
    ) -> Result<SaveCheckpointResult, RepositoryError>;

    fn find_latest_checkpoint(
        &self,
        item_id: ProgressItemId,
    ) -> Result<Option<ProgressCheckpoint>, RepositoryError>;

    fn find_checkpoints(
        &self,
        item_id: ProgressItemId,
        range: ProgressCheckpointRange,
    ) -> Result<Vec<ProgressCheckpoint>, RepositoryError>;
}
```

The exact `New*` structs can either live in the domain module or application module. Prefer the
domain module if they encode invariant-checked creation requests.

`save_checkpoint` should be idempotent when both `source` and `source_event_id` are present.

```rust
pub enum SaveCheckpointResult {
    Inserted(ProgressCheckpoint),
    Duplicate(ProgressCheckpoint),
}
```

`ProgressCheckpointRange` should support optional `recorded_at` bounds:

```rust
pub struct ProgressCheckpointRange {
    pub recorded_at_start: Option<DateTime<Utc>>,
    pub recorded_at_end: Option<DateTime<Utc>>,
}
```

Use this single method for both timeline inspection and "all checkpoints through date X" daily-delta
calculation. Do not add a second overlapping `find_checkpoints_through` method.

Manual logs should use `source = None` and `source_event_id = None`, so they always insert a new
checkpoint. Integrations should provide both fields for idempotency.

## Application Layer

Add progress use cases using the existing flat command/query layout:

```text
core/application/src/use_cases/commands/create_progress_item/
  mod.rs
  command.rs
  handler.rs
core/application/src/use_cases/commands/update_progress_target/
  mod.rs
  command.rs
  handler.rs
core/application/src/use_cases/commands/log_progress_checkpoint/
  mod.rs
  command.rs
  handler.rs
core/application/src/use_cases/queries/get_progress_item/
  mod.rs
  query.rs
  handler.rs
core/application/src/use_cases/queries/list_progress_items/
  mod.rs
  query.rs
  handler.rs
core/application/src/use_cases/queries/get_progress_summary/
  mod.rs
  query.rs
  handler.rs
core/application/src/use_cases/queries/get_progress_checkpoints/
  mod.rs
  query.rs
  handler.rs
core/application/src/use_cases/queries/get_progress_daily_deltas/
  mod.rs
  query.rs
  handler.rs
```

The important decision is that progress use cases should not be implemented by extending
`LogValueCommand`.

### Commands

#### CreateProgressItemCommand

```rust
pub struct CreateProgressItemCommand {
    pub name: String,
    pub category: String,
    pub target_value: i64,
    pub unit: ProgressUnit,
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}
```

Returns:

```rust
ProgressItem
```

Responsibilities:

- validate and construct the domain object
- persist through `ProgressRepository`
- return the created item

#### UpdateProgressTargetCommand

```rust
pub struct UpdateProgressTargetCommand {
    pub item_id: ProgressItemId,
    pub target_value: i64,
    pub updated_at: DateTime<Utc>,
}
```

Responsibilities:

- load the item
- validate new target
- reject target values below the current furthest checkpoint
- update item target and `updated_at`

Do not support changing the unit in v1. Changing from pages to characters is a semantic edit that
can invalidate historical checkpoint meaning. If a unit was created incorrectly, create a new
progress item and import corrected checkpoints there.

#### LogProgressCheckpointCommand

```rust
pub struct LogProgressCheckpointCommand {
    pub item_id: ProgressItemId,
    pub recorded_at: DateTime<Utc>,
    pub effective_date: NaiveDate,
    pub position: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
}
```

Responsibilities:

- load the progress item
- validate the item exists
- validate `0 <= position <= item.target.value`
- construct `ProgressCheckpoint`
- save idempotently through `ProgressRepository`
- return whether the checkpoint was inserted or deduplicated

Do not compute or save daily deltas in this command.

### Queries

#### GetProgressSummaryQuery

`ProgressSummary` is an application-layer query projection. Do not put it in
`core/core/src/progress`; define it next to `GetProgressSummaryQuery`.

```rust
pub struct ProgressSummary {
    pub item: ProgressItem,
    pub latest_checkpoint: Option<ProgressCheckpoint>,
    pub latest_position: Option<i64>,
    pub furthest_position: Option<i64>,
    pub percent_latest: Option<f64>,
    pub percent_furthest: Option<f64>,
    pub is_complete: bool,
}
```

`percent_*` should be `None` only when no checkpoint exists. It should otherwise be clamped to
`0.0..=1.0` for presentation safety, even though command validation prevents positions above target.

#### GetProgressDailyDeltasQuery

```rust
pub struct GetProgressDailyDeltasQuery {
    pub item_id: ProgressItemId,
    pub date_range: DateRange,
}
```

Returns:

```rust
Vec<DatedValue<i64>>
```

Algorithm:

1. Load all checkpoints up to the end of the requested date range, ordered by `recorded_at`,
   then checkpoint ID as a stable tie-breaker.
2. Initialize `frontier` to `0`.
3. For each checkpoint:
   - `delta = max(0, checkpoint.position - frontier)`
   - `frontier = max(frontier, checkpoint.position)`
   - if `checkpoint.effective_date` is inside the requested range, add `delta` to that date
4. Return one value per date that has a positive delta.

This query intentionally needs checkpoints before the requested range to calculate the correct
starting frontier.

#### GetProgressCheckpointsQuery

Supports inspection/debugging and UI timeline rendering.

```rust
pub struct GetProgressCheckpointsQuery {
    pub item_id: ProgressItemId,
    pub recorded_at_start: Option<DateTime<Utc>>,
    pub recorded_at_end: Option<DateTime<Utc>>,
}
```

Return checkpoints ordered by `recorded_at`, then checkpoint ID.

## Persistence Design

Add a SQLite adapter:

```text
core/infrastructure/persistence/src/progress_repository.rs
```

Export it from:

```text
core/infrastructure/persistence/src/lib.rs
```

### Migration

Add a migration under:

```text
core/infrastructure/persistence/migrations/2026-05-02-000000_progress_items/
  up.sql
  down.sql
```

If another migration with that timestamp already exists when implementation starts, choose the
actual creation timestamp using the existing `YYYY-MM-DD-HHMMSS_name` convention so migrations sort
chronologically.

Suggested schema:

```sql
CREATE TABLE progress_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    unit_kind TEXT NOT NULL,
    unit_label TEXT,
    target_value BIGINT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    canonical_tags TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (length(trim(name)) > 0),
    CHECK (length(trim(category)) > 0),
    CHECK (target_value > 0)
);

CREATE INDEX idx_progress_items_category
    ON progress_items(category);

CREATE INDEX idx_progress_items_canonical_tags
    ON progress_items(canonical_tags);

CREATE TABLE progress_checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    progress_item_id INTEGER NOT NULL,
    recorded_at TEXT NOT NULL,
    effective_date TEXT NOT NULL,
    position_value BIGINT NOT NULL,
    source TEXT,
    source_event_id TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(progress_item_id) REFERENCES progress_items(id) ON DELETE CASCADE,
    CHECK (position_value >= 0)
);

CREATE INDEX idx_progress_checkpoints_item_recorded_at
    ON progress_checkpoints(progress_item_id, recorded_at, id);

CREATE INDEX idx_progress_checkpoints_item_effective_date
    ON progress_checkpoints(progress_item_id, effective_date);

CREATE UNIQUE INDEX idx_progress_checkpoints_source_event
    ON progress_checkpoints(progress_item_id, source, source_event_id)
    WHERE source IS NOT NULL AND source_event_id IS NOT NULL;
```

Use ISO 8601 strings for timestamps, matching the style already used in persistence code.

`canonical_tags` should follow the existing statistic tag canonicalization style so integration
bundle provisioning can eventually be idempotent for progress items too.

### Diesel Schema and Models

Update:

```text
core/infrastructure/persistence/src/db/schema.rs
core/infrastructure/persistence/src/db/models.rs
```

Recommended raw models:

```rust
pub struct ProgressItemRow {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub unit_kind: String,
    pub unit_label: Option<String>,
    pub target_value: i64,
    pub tags: String,
    pub canonical_tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProgressCheckpointRow {
    pub id: i32,
    pub progress_item_id: i32,
    pub recorded_at: String,
    pub effective_date: String,
    pub position_value: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
```

Keep row-to-domain mapping in the persistence adapter or a dedicated mapper module. Do not leak raw
Diesel rows into the application layer.

### Idempotency

When `source` and `source_event_id` are present:

- first insert returns `SaveCheckpointResult::Inserted`
- duplicate insert returns `SaveCheckpointResult::Duplicate(existing_checkpoint)`
- duplicates must not create another checkpoint row or any downstream side effects

For SQLite, implement this with the partial unique index and an insert-or-ignore/select-existing
transaction.

## Bootstrap and Dependency Injection

Update the composition root:

```text
lifestats-bootstrap/src/lib.rs
```

Add a progress repository field to the application struct:

```rust
pub progress_repository: Arc<SqliteProgressRepository>,
```

or, if the existing application struct prefers trait objects:

```rust
pub progress_repository: Arc<dyn ProgressRepository + Send + Sync>,
```

The web API should receive the progress repository through app state, parallel to
`statistic_repository`.

Implementation touchpoints:

- add the repository to the bootstrap `Application`
- instantiate `SqliteProgressRepository` from the existing SQLite pool
- add the repository to the web API app state struct
- clone it into progress route handlers using the same pattern as statistic routes
- register progress routes in `routes::create_router`

## HTTP API Design

Add routes under:

```text
core/infrastructure/web_api/src/routes/progress.rs
```

Register them from:

```text
core/infrastructure/web_api/src/routes/mod.rs
```

Recommended v1 routes:

```text
POST /progress-items
GET  /progress-items
GET  /progress-items/:id
PATCH /progress-items/:id/target
POST /progress-items/:id/checkpoints
GET  /progress-items/:id/checkpoints
GET  /progress-items/:id/summary
GET  /progress-items/:id/daily-deltas?start=YYYY-MM-DD&end=YYYY-MM-DD
```

### Create Item Request

```json
{
  "name": "The Left Hand of Darkness",
  "category": "Reading",
  "target_value": 304,
  "unit": {
    "type": "Pages"
  },
  "tags": {
    "media_type": "book",
    "author": "Ursula K. Le Guin"
  }
}
```

For custom units:

```json
{
  "unit": {
    "type": "Custom",
    "label": "lessons"
  }
}
```

### Log Checkpoint Request

```json
{
  "recorded_at": "2026-05-02T20:15:00Z",
  "effective_date": "2026-05-02",
  "position": 73,
  "source": null,
  "source_event_id": null,
  "note": "Finished chapter 5"
}
```

For clients that do not supply `recorded_at`, the API may default it to server time. However, the
application command should require it explicitly so adapters make the defaulting decision at the
edge.

Manual API logs should map to `source = None` and `source_event_id = None`. Reserve non-empty
`source` values for integrations, imports, or automated clients that can also provide stable source
event IDs.

Two identical manual submissions, such as logging page 73 twice, will create two checkpoints. This
is acceptable in v1 because checkpoints are absolute positions; the second checkpoint does not add
derived daily progress under the furthest-frontier algorithm.

### Summary Response

```json
{
  "item": {
    "id": 1,
    "name": "The Left Hand of Darkness",
    "category": "Reading",
    "target_value": 304,
    "unit": { "type": "Pages" },
    "tags": { "media_type": "book" }
  },
  "latest_position": 73,
  "furthest_position": 73,
  "percent_latest": 0.2401,
  "percent_furthest": 0.2401,
  "is_complete": false,
  "latest_checkpoint": {
    "recorded_at": "2026-05-02T20:15:00Z",
    "effective_date": "2026-05-02",
    "position": 73
  }
}
```

## Events

V1 ships with no progress events.

Do not add `ProgressItemCreated` or `ProgressUpdated` variants until there is a concrete subscriber
that needs them. This avoids creating accidental event semantics before the progress model has a
consumer beyond request/response queries.

The progress design must not publish `ValueUpdated(StatisticId)` because progress items are not
statistics.

## Relationship to Existing Statistics

Progress items can produce statistic-like query projections, but they are not daily statistics.

Allowed:

- show progress items in a separate UI section
- add a progress-specific plot endpoint
- derive daily deltas for a progress item
- manually create a separate `IntegerAmount` statistic for global daily reading volume

Not allowed in v1:

- storing progress checkpoints in `logging_data`
- making `ProgressItemId` interchangeable with `StatisticId`
- using `LogValueCommand` to save progress checkpoints
- treating progress targets as default goals

## Relationship to Ebook Integration

Keep the existing ebook daily metric:

```text
[Ebook] Characters Read -> IntegerAmount
```

Progress items may later be used for per-book state:

```text
ProgressItem: The Left Hand of Darkness
Target: 581233 characters
Checkpoints: current confirmed character offsets
```

Do not make ebook progress depend on progress items in the first implementation unless the user
explicitly wants that integration. The immediate goal is to introduce the clean model and API.

A later ebook integration can:

- create one progress item per book
- log confirmed position checkpoints with `source = "ebook"`
- use stable source event IDs for idempotency
- continue logging global daily `characters_read` as the analytics sink

## Testing Plan

### Domain Tests

Add tests for:

- valid progress item construction
- rejection of empty names/categories
- rejection of non-positive targets
- valid checkpoint construction
- rejection of negative positions
- note normalization
- custom unit validation

### Application Tests

Use mock `ProgressRepository` tests for:

- creating a progress item
- logging a valid checkpoint
- rejecting checkpoint for missing item
- rejecting checkpoint above target
- duplicate source event does not create another checkpoint or derived side effect
- target update rejects a new target below furthest known position
- summary with no checkpoints
- summary with forward-only checkpoints
- summary with backward movement
- daily deltas with backward movement
- daily deltas where the requested range starts after existing progress

### Persistence Tests

Use in-memory SQLite tests for:

- create and fetch progress item
- update target
- insert and fetch latest checkpoint
- checkpoint ordering by timestamp and ID
- range queries
- source event idempotency
- JSON tag round trip
- canonical tag lookup

### Web API Tests

Add integration tests for:

- `POST /progress-items`
- `POST /progress-items/:id/checkpoints`
- duplicate source event checkpoint
- `GET /progress-items/:id/summary`
- `GET /progress-items/:id/daily-deltas`
- validation errors for invalid target/position/date

## Implementation Phases

### Phase 1: Domain and Port

1. Move `RepositoryError` to the neutral shared port module and re-export it from the old statistic
   path.
2. Add `ProgressItemId` and optionally `ProgressCheckpointId`.
3. Add `core/core/src/progress`.
4. Add domain constructors and validation.
5. Add `ProgressRepository` port.
6. Add domain tests.

Exit criteria:

- `cargo test -p lifestats-core progress` passes
- existing statistic repository imports still compile through the compatibility re-export
- no changes to statistic behavior

### Phase 2: Application Use Cases

1. Add progress commands and queries.
2. Add mock-based application tests.
3. Add summary and daily-delta calculation.

Exit criteria:

- application tests cover backward movement and range-start frontier behavior
- no progress use case depends on `StatisticRepository`

### Phase 3: Persistence

1. Add migration.
2. Add Diesel schema/models.
3. Implement `SqliteProgressRepository`.
4. Add persistence tests.
5. Wire repository into `lifestats-bootstrap`.
6. Add the repository to the bootstrap `Application` struct.

Exit criteria:

- progress persistence tests pass against in-memory SQLite
- existing `just test-db-compat` remains unaffected

### Phase 4: HTTP API

1. Add `routes/progress.rs`.
2. Register routes.
3. Add request/response DTOs in the web API adapter.
4. Add the progress repository to the web API app state.
5. Clone the repository into progress route handlers.
6. Add API tests.

Exit criteria:

- manual API flow can create a book, log checkpoints, retrieve summary, retrieve daily deltas
- validation errors are user-readable

### Phase 5: UI or Integration Follow-Up

This is intentionally outside the first backend implementation.

Possible follow-ups:

- add a small web UI section for progress items
- add progress plots
- add per-book ebook progress items
- add import/export for progress checkpoints

## Rejected Alternatives

### Add `AnyStatistic::Progress`

Rejected for v1.

The current statistic abstraction expects a daily log value, default goal behavior, and integration
with daily goal/streak/query code. A progress item has different semantics:

- checkpoints are timestamped, not date-keyed
- daily values are derived, not canonical
- targets are object bounds, not daily goals
- latest position and furthest position are both meaningful

Adding progress directly to `AnyStatistic` would force many existing statistic paths to either
special-case progress or pretend it is daily-shaped.

### Store Current Page as `IntegerAmount` with Overwrite Policy

Rejected as the primary model.

This can store one value per date, but it loses intra-day history and cannot cleanly distinguish:

- current position
- furthest position
- duplicate source events
- backward movement
- daily new progress

It is acceptable as a temporary manual workaround, not as the code design.

### Store Daily Pages Read as `IntegerAmount` with Sum Policy

Rejected as the source of truth.

Daily pages read is a useful projection, but it is brittle as canonical state. Missing or duplicated
events corrupt the reconstructed current position. Absolute checkpoints are more robust.

### Make One Statistic Per Book

Rejected for progress state.

One daily statistic per book can answer "how much did I read in this book per day" but still does not
model "where am I now?" naturally. It also pollutes the statistics list with object instances rather
than reusable metric definitions.

Progress items are object instances by design, so one progress item per book is appropriate.

## Compatibility and Migration Impact

This feature is additive.

It should not require:

- modifying existing rows in `statistics`
- modifying existing rows in `logging_data`
- changing existing statistic type names
- changing existing daily goal behavior
- changing the ebook daily character sync path

Database backward compatibility risk is low because the new tables are independent. The main risk is
bootstrap or migration wiring, so keep the migration simple and add repository-level tests.

## Engineering Notes

- Keep domain logic pure and dependency-light, matching `core/core`.
- Do not parse HTTP strings in the application layer; adapters should convert request DTOs into
  typed commands.
- Use `rg` to find all composition root and router registration points before implementing.
- Prefer small mappers over ad hoc JSON/string manipulation in handlers.
- Keep progress DTOs out of the statistic DTO module unless there is a deliberate shared boundary.
- Use deterministic ordering for checkpoints: `recorded_at ASC, id ASC`.
- Avoid adding dashboard behavior until the backend semantics are proven.

## Definition of Done

The feature is implementation-complete when:

- a user can create a progress item for a book
- a user can log absolute page/character checkpoints
- the latest and furthest positions are queryable
- completion percentage is queryable
- daily new progress can be derived from checkpoints
- backward movement does not subtract daily progress
- duplicate imported checkpoints are idempotent
- existing statistics and ebook character sync continue to behave unchanged
- domain, application, persistence, and API tests cover the behavior above
