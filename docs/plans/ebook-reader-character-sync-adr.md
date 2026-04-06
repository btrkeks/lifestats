# Ebook Reader Character Sync ADR

## Status

Proposed

## Date

2026-04-05

## Context

The ebook reader currently tracks local reading statistics and is primarily optimized around
reading time. The new goal is to log daily reading progress to lifestats as net new characters
advanced through a book.

This metric should reflect new text read on a given day and should exclude rereading by default.
The main product risk is accidental or low-confidence position changes:

- The user may scroll far forward or backward by mistake.
- The user may read on another device that does not report progress to lifestats.
- The app should recover from suspicious jumps without making the lifestats metric noisy.

The system should also fit the current architecture:

- The ebook reader remains the source of truth for reading progress.
- Lifestats acts as an append-only analytics sink.
- Sync failures should not lose progress.
- No backfill is required. Only sessions after rollout need to be synced.

## Decision

We will implement character syncing using a local-first confirmed-frontier model.

The ebook reader will maintain three distinct progress positions:

- `current_position`: the currently observed explored character count in the reader UI.
- `confirmed_position`: the furthest position that the tracker believes has been genuinely read.
- `last_flushed_position`: the furthest confirmed position already sent to lifestats.

Only positive deltas between `confirmed_position` and `last_flushed_position` will be logged to
lifestats.

We will not create one lifestats statistic per book. Instead, the reader will provision a single
global ebook bundle anchored around one `読書` statistic family. The reader will attach per-book
identity and metadata locally, but lifestats aggregation will be global for daily analytics.

Large jumps will not be trusted immediately. They will enter a pending state and become counted
only after they satisfy confidence heuristics:

- sufficient dwell time at the jumped-to position
- continued forward reading after the jump

Backward movement will not normally decrement lifestats. This keeps the external metric aligned
with "net new text read". Repair for accidental forward jumps will be handled by delaying
confirmation rather than by aggressively sending negative corrections.

The implementation will include a durable local sync queue with retry. The ebook reader local
database remains authoritative; lifestats sync is downstream and best-effort, but durable.

## Decision Details

### Metric Semantics

The primary metric is:

- `characters_read`: net new characters advanced through books per day

Interpretation:

- rereading does not add to the metric
- backward movement does not subtract from the metric in normal operation
- only confirmed positive progress is flushed

This keeps the metric stable and understandable in lifestats.

### Confirmation Model

The tracker will distinguish between observed movement and trusted movement.

Small forward movement:

- confirm immediately or near-immediately

Large forward jump:

- create a `pending_forward_jump`
- do not count it yet
- promote it to confirmed only after dwell time and/or continued forward reading

Backward movement:

- update local UI state
- do not emit negative lifestats deltas
- if a large forward jump was still pending, a backward move can cancel or reduce the pending jump

This mechanism also supports the "I read on my phone" case. A large forward jump on desktop can
still be accepted later if the user remains there and continues reading naturally.

### Global Bundle

We will create a new integration source, `ebook`, with a single global bundle.

Recommended v1 statistics:

- `characters_read`: `IntegerAmount`, update policy `sum`
- `reading_time`: `TimeSpans` or `IntegerAmount` depending on desired future use
- `current_position`: `IntegerAmount`, update policy `overwrite`
- `completed_book`: `Habit` or `PointInTime`

Notes:

- `characters_read` is the core requirement.
- `reading_time` is kept because it is already a useful dimension and may remain useful even if
  character progress becomes the primary metric.
- `current_position` is useful for debugging, repair, and future analytics even if not surfaced
  prominently.
- `completed_book` preserves a clean completion signal.

Because this is a global bundle rather than a per-book bundle, idempotency must be driven by
stable source tags such as:

- `source=ebook`
- `metric=characters_read`

Book identity should not be encoded into the lifestats statistic identity. Book identity is a
local concern for the reader.

### Source of Truth

The ebook reader local database remains the source of truth for:

- per-book progress
- session-level reading heuristics
- pending and confirmed positions
- queued but unsynced lifestats deltas

Lifestats receives only derived, append-only events or snapshot updates.

### Failure Handling

Sync to lifestats will use a durable local queue with retry.

Required properties:

- queue survives tab reloads and browser restarts
- queue entries are idempotent or de-duplicatable
- transient network failure does not lose progress
- sync can be retried in the background and on startup

No historical backfill is required.

## Refactoring Decision

This feature should include a modest refactor before adding lifestats sync logic.

We will extract a pure progress-tracking module from the current Svelte component logic.

### New Modules

#### `ReadingProgressTracker`

A pure TypeScript module responsible for:

- ingesting position and timer events
- maintaining `current_position`, `confirmed_position`, and pending jump state
- calculating local character deltas
- deciding what is flushable to lifestats

Suggested input events:

- `tick(elapsedSeconds, exploredCharCount)`
- `pause()`
- `resume()`
- `freeze(position)`
- `unfreeze()`
- `close()`

Suggested output/state:

- `currentPosition`
- `confirmedPosition`
- `lastFlushedPosition`
- `pendingJump`
- `sessionConfirmedCharacters`
- `flushableCharacters`
- `diagnosticReason`

This module must be testable without Svelte or browser DOM coupling.

#### `LifestatsSyncAdapter`

A thin adapter responsible for:

- provisioning the ebook bundle
- persisting bundle ids locally
- enqueueing sync work
- flushing queue items to lifestats
- retrying failed syncs

This adapter should not contain reading heuristics.

#### `EbookBundleFactory`

A new backend bundle factory in the application layer responsible for:

- provisioning the ebook integration bundle under source `ebook`
- defining canonical tags used for idempotent lookup
- keeping bundle identity global rather than per-book

#### `EbookEventIngestion`

A dedicated backend ingestion path responsible for:

- accepting append-only ebook sync events
- deduplicating retries by stable event id
- mapping accepted events to the provisioned ebook statistics
- guaranteeing that transport retries do not double-log character deltas

### Keep In Component

The Svelte component should remain responsible for:

- wiring browser events to the tracker
- displaying local statistics and warnings
- exposing user controls
- invoking persistence and sync adapters

This separation keeps UI, heuristics, and transport logic independent.

## Current Codebase Alignment

The implementation should be anchored to the current code rather than introduced as a parallel
tracking path.

Key observations from the existing codebase:

- the reader already computes `exploredCharCount` in the render path and uses it as the observed
  UI position
- the reader already has a local reading tracker that maintains per-book daily statistics such as
  `charactersRead` and `readingTime`
- there is currently an ad hoc lifestats heartbeat experiment embedded directly in the continuous
  reader Svelte component
- the current bookmark schema stores observed progress, but it does not store the confirmed/flushed
  frontier required by this ADR

Implementation consequences:

- `exploredCharCount` remains the observed source position
- existing local reader statistics remain the source for reader-local analytics and UI
- lifestats sync state becomes a separate local model with explicit confirmation and queueing
- the hardcoded heartbeat integration should be removed rather than extended

This means the reader will have three distinct layers of state:

1. observed reader position
2. local reader statistics and bookmarks
3. lifestats sync frontier and durable outbound queue

Only the third layer should drive lifestats writes.

## Alternatives Considered

### 1. Raw Start-to-End Position Delta

Use session start position and session end position, then flush the difference.

Pros:

- simple to reason about
- easy to implement

Cons:

- fragile in the presence of accidental jumps
- loses recovery opportunities during a session
- depends too heavily on correct close behavior
- poor crash resilience

Rejected because it is too brittle for the desired product semantics.

### 2. Only Flush on Close

Calculate progress only when the reader closes.

Pros:

- simpler external sync model
- avoids frequent sync traffic

Cons:

- progress can be lost on crash
- delayed visibility in lifestats
- difficult to recover partial session progress

Rejected because a durable incremental model is safer and more useful.

### 3. Per-Book Lifestats Bundle

Provision one set of lifestats statistics per book.

Pros:

- richer book-level analytics in lifestats
- easier direct debugging per book

Cons:

- operationally noisy
- creates many short-lived statistics
- does not match the desired user workflow

Rejected because the desired primary metric is global daily reading volume.

### 4. View-Time-Based Character Attribution

Count characters only after they have remained visible long enough.

Pros:

- potentially higher fidelity

Cons:

- much more complex
- more DOM-dependent
- harder to debug and tune

Rejected for v1. The confirmed-frontier model gives most of the value with less complexity.

## Consequences

Positive:

- lifestats receives a stable daily metric for new reading progress
- accidental jumps are less likely to corrupt the metric
- the phone-reading case can still reconcile through later confirmed jumps
- the progress logic becomes testable outside the UI
- sync failures become recoverable

Negative:

- implementation complexity is higher than naive deltas
- jump confirmation heuristics will need tuning
- the reader now owns more explicit sync state

Risks:

- over-conservative heuristics may undercount legitimate large jumps
- over-permissive heuristics may still overcount accidental jumps
- queue duplication bugs could double-log progress unless idempotency is handled carefully

## Implementation Plan

### Phase 1: Define Boundaries

1. Remove the assumption that the render component is allowed to talk to lifestats directly.
2. Treat the current reader component as a position sensor only.
3. Reuse the existing local tracker for reader-local statistics, but do not use its raw
   `charactersRead` values as the lifestats source of truth.
4. Extract pure progress confirmation logic into a dedicated TypeScript reducer-like module.
5. Define explicit tracker states for:
   - steady forward reading
   - pending forward jump
   - paused
   - frozen

Deliverable:

- a pure `ReadingProgressTracker` with unit tests for jump confirmation, cancellation, and flushable
  delta calculation

### Phase 2: Add Ebook Integration Bundle

1. Add a new integration factory in the application layer for `ebook`.
2. Expose it through `POST /integrations/ebook`.
3. Register the new factory in the bootstrap composition root.
4. Define the v1 bundle response with ids for:
   - `characters_read`
5. Optionally reserve room for later v2 metrics such as:
   - `reading_time`
   - `completed_books`
6. Keep the bundle global, keyed only by source and metric tags.

Recommended v1 backend tags:

- `source=ebook`
- `metric=characters_read`
- `bundle_version=1`

Deliverable:

- idempotent ebook bundle provisioning in the backend

### Phase 3: Add Reader Sync Adapter

1. Add a dedicated lifestats client for the ebook reader.
2. Provision or load the ebook bundle on demand.
3. Introduce durable queued sync records in the existing IndexedDB database rather than ephemeral
   in-memory state.
4. Persist a per-book ebook sync state record containing:
   - `dataId`
   - `currentPosition`
   - `confirmedPosition`
   - `lastFlushedPosition`
   - `pendingJump`
   - `bundleVersion`
   - `updatedAt`
5. Persist a queue entry model containing:
   - `eventId`
   - `dataId`
   - `dateKey`
   - `metric`
   - `delta`
   - `confirmedPositionAfter`
   - `bookTitle`
   - `createdAt`
   - `attemptCount`
   - `nextRetryAt`
6. Flush queue entries in the background and on startup.
7. Ensure only positive confirmed deltas are enqueued for `characters_read`.

Deliverable:

- durable best-effort sync to lifestats

### Phase 4: Integrate Tracker With UI

1. Feed observed position, pause, resume, freeze, and unfreeze events from the UI into
   `ReadingProgressTracker`.
2. Keep the existing local reader statistics update flow intact.
3. Route only confirmed flushable deltas into the sync queue.
4. Remove the hardcoded heartbeat session calls from the continuous reader component.
5. Keep existing threshold settings where possible, but reinterpret them through tracker state
   transitions rather than direct mutation.

Deliverable:

- UI still behaves as before, but progress confirmation is explicit and sync-aware

### Phase 5: Add Idempotent Backend Ingestion

1. Do not rely on `POST /statistics/{id}/log/{date}` for retried `IntegerAmount(sum)` writes.
2. Add a dedicated ebook ingestion endpoint, for example:
   - `POST /integrations/ebook/events`
3. Accept an append-only event payload containing:
   - `event_id`
   - `date`
   - `metric`
   - `delta`
   - `book_id`
   - `book_title`
   - `confirmed_position_after`
4. Persist accepted event ids on the backend.
5. If an event id is retried, return success without applying the delta a second time.

Deliverable:

- transport-safe lifestats ingestion for retried character deltas

### Phase 6: Validation

Test scenarios:

- normal steady reading
- small backward corrections
- accidental large forward scroll then return
- legitimate large jump followed by dwell and continued reading
- read on phone, then reopen on desktop at a later position
- browser reload with unsynced queue entries
- lifestats unavailable during session, then available later
- request timeout after server success, followed by client retry of the same event id

Deliverable:

- confidence that local statistics and lifestats remain consistent enough for daily use

## Open Tuning Questions

These are not blocking architectural decisions, but they should be tuned during implementation:

- what dwell duration confirms a large jump
- what forward distance counts as a suspicious jump
- whether `reading_time` belongs in the ebook bundle at all in v1
- whether completion should be represented as `completed_books: IntegerAmount(sum)` in a later
  version
- how aggressively pending jumps should be reduced after backward movement

## Initial Recommendations For Tuning

- start with conservative jump thresholds
- prefer undercounting suspicious jumps over overcounting them
- allow promotion of pending jumps through continued forward reading
- never send negative character deltas to lifestats in v1
- keep the queue format append-only and explicit so it can be inspected during debugging
- use explicit event ids for every outbound sync event
- keep the v1 bundle minimal and add secondary metrics only after the character pipeline is stable

## Concrete v1 Implementation Decisions

These choices are sufficiently clear to treat as implementation defaults rather than open questions.

- The current ad hoc heartbeat integration in the reader component should be removed.
- The ebook reader local database should be upgraded with dedicated sync state and queue stores.
- `ReadingProgressTracker` should be implemented as a pure TypeScript module with no Svelte or DOM
  dependencies.
- The v1 lifestats bundle should contain only `characters_read`.
- `current_position` should remain local-only in v1 because a global lifestats statistic would not
  have meaningful semantics across books.
- `completed_book` should not be added in v1; if needed later, it should become a global
  `completed_books` counter rather than a per-book identity encoded into lifestats.
- Backend ingestion for ebook sync should be idempotent by event id rather than relying on
  best-effort retries against the generic statistics logging route.
