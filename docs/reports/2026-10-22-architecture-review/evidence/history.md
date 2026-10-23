# Source-history investigation: Lifestats ownership

Repository: `/home/daniel/.local/share/src/lifestats`; inspected HEAD `06e4e70888b2fca062e3fdf2fd6fed60fd39d389`, 2026-10-22. Analysis/design only. No repository, database, service, or external-state writes. Static architecture and deployment verification remain parent coverage.

Confidence: **Direct** = explicit written rationale; **Supported** = converging evidence; **Inferred** = interpretation; **Unknown** = searched without finding an answer. `commit:path:line` identifies a historical file; `HEAD:path:line` identifies the inspected checkout.

## What Searched

Read root `AGENTS.md`, the T3 adapter/model policy, `why/SKILL.md`, and its epistemics, investigator, and code-archaeology references. Read all three requested architecture/integration documents and the LiftLog docs index/sync guide. The root docs have no `index.md`; the LiftLog docs index was read for its sync guide.

Used bounded `git log --follow -12` for application dependencies, architecture documentation, logging handler, and bootstrap; a combined `git log -15` for anchors; and `git log -40 --format='%h %B'` scoped to application/core/bootstrap and the three documents. Scanned those messages for `#[0-9]+`, `https?://`, and ticket-like `[A-Z]{2,8}-[0-9]+` references. No substantive PR/ticket reference emerged; therefore no GitHub calls.

Ran blame on application dependencies, timeline rendering, bootstrap, and the progress ADR; pickaxe `git log -8 -S lifestats-plotting` and `-S NoOpEventPublisher`; bounded regex history searches for `offline|queue|retry|version|ordering` and `offline|outbox|idempoten|version|conflict|replay`. Read full relevant messages and scoped diffs/blobs for all five seeds and follow-ups `4177790`, `ffe0261`, `35c25d4`, `3c747ac`, `37372db`, `2ccb91b`, `3629bdc`, `e186592`, `e3a72f0`, `ed6e501`, `8373bd5`, `f4371e4`, `d6a4904`, `54c6888`, `ef786b7`, and `1ebfb93`. Large unrelated changes within those commits were not exhaustively reviewed.

## Evidence

### 1. Domain protection and independent capture adapters are explicit goals

**Direct.** `4177790` (butterkeks, 2025-10-03) rewrote the architecture guide: “The core domain logic is the most valuable asset and must be protected from external changes.” It requires inward dependencies and explicit ports, while defining the application as orchestration and binaries as driving adapters. Logger addition should require no core changes. Sources: `4177790:docs/ARCHITECTURE.md:17-24,86-108,181-183` (same text at HEAD).

**Direct, historical only.** The earlier `d87507d` document explicitly planned HTTP mobile integration and device sync, while rejecting microservices as unnecessary complexity and multi-user support as outside a personal tool: `d87507d:docs/ARCHITECTURE.md:585-598`. That document was subsequently rewritten. These are historical constraints, not proof of a currently accepted ADR. `d87507d` itself says the restructure is “currently broken” and a version-control checkpoint, so its directory arrangement is not a completed target architecture.

### 2. CQRS deliberately moved parsing outward and shortened handler lifetimes

**Direct.** `f72350e` (butterkeks, 2025-10-08) documents strongly typed command inputs, side-effect-free queries, and adapters constructing handlers on demand. It explicitly assigns user-input parsing to adapters and says adapter-specific logic belongs in adapter crates: `f72350e:core/application/README.md:3-9,47,66-99,122`.

The logging command says typed payloads replace strings, moving parsing to web/CLI adapters: `f72350e:core/application/src/use_cases/commands/log_value/command.rs:11-44`. The handler describes fetch → pure calculation → save if changed → publish if changed; implementation still matches this sequence: `HEAD:core/application/src/use_cases/commands/log_value/handler.rs:13-17,55-79,164-169`.

**Supported.** Bootstrap exposure of repositories is a consequence of this handler-construction model, not evidence that HTTP routes were intended to own business policy. The same commit removes long-lived logging/goal/query services from both `Application` and `AppState`, replacing them with shared dependencies; migrated MIDI/stopwatch callers instantiate queries. Sources: `f72350e:lifestats-bootstrap/src/lib.rs:26-66`; `f72350e:core/infrastructure/web_api/src/state.rs:8-29`; corresponding logger-main diffs.

### 3. Composition root and retained services have distinct histories

**Direct.** `3c747ac` (2025-10-07) introduces `build_services()` as “our Composition Root,” constructing SQLite, renderer, publisher, and integration components, and removes legacy stopwatch dependencies in favor of bootstrap. Sources: `3c747ac:lifestats-bootstrap/src/lib.rs:26-88`; `3c747ac:logger_applications/stopwatch_logger/Cargo.toml:6-10`.

**Direct.** The CQRS guide explicitly retains heartbeat buffering and the integration façade for multi-operation coordination: `f72350e:core/application/README.md:87-89`. `37372db` (2025-10-25) states why server session coordination is statistic-keyed: “One session per statistic (prevent conflicts).” It accepts restart loss of in-memory sessions because data is flushed, while sessions survive client disconnect. The source exposes that orchestration and map: `37372db:core/application/src/use_cases/session_manager.rs:1-11,43-50,94-112`.

**Unknown.** Bootstrap's no-op publisher dates to its creation and remains at `HEAD:lifestats-bootstrap/src/lib.rs:32-41,74`. Neither its introduction message nor inspected follow-ups explain whether suppressed notifications were temporary, optional, or deliberate product behavior. It is not evidence of durable event delivery.

### 4. Stateless plotting was intentional; concrete renderer ownership was not justified

**Direct.** `35c25d4` (2025-10-06) separates plot and statistic contexts. `cdec4f0` (2025-11-13) explicitly removes plot storage to eliminate stateful configuration, simplify mocks, and make transformations explicit. `2ccb91b` completes deletion, including `DROP TABLE IF EXISTS plots`: `2ccb91b:core/infrastructure/persistence/migrations/2025-11-13-102621_drop_plots_table/up.sql:1-2`.

**Direct observation; Unknown intent.** `cdec4f0` also adds application → concrete plotting infrastructure (`HEAD:core/application/Cargo.toml:10`) and timeline PNG rendering with fixed dimensions. Its local comment explains a duplicate event shape as avoiding a circular dependency, but does not explain why rendering must live in application: `cdec4f0:core/application/src/use_cases/queries/get_daily_timeline_plot/handler.rs:33-52`. General plot rendering remains in the HTTP adapter in that commit. Statelessness does not require this dependency direction.

**Direct.** Later behavior changes distinguish absent observations from active goals and suppressed goals. `3629bdc` says goal-only days make missed goals visible; `e186592` fixes suppressed days accidentally inheriting defaults; `e3a72f0` excludes suppressed/no-applicable-goal days from performance rather than inflating scores. These semantics should survive a renderer move.

### 5. Progress is intentionally separate canonical state

**Direct.** The progress ADR distinguishes daily observations from “where was this tracked object at this moment?” It chooses sibling objects/repositories/use cases, append-only absolute checkpoints, and derived daily deltas; rejects extending `AnyStatistic` or `LogValueCommand`. Sources: `HEAD:docs/plans/progress-items-architecture-adr.md:13-62,64-94,1057-1103`.

It deliberately separates UTC ordering from caller-chosen effective date, latest from furthest position, and source-event deduplication from manual repeated submissions. It forbids duplicate downstream side effects and unnecessary progress events without a subscriber: same document `191-215,405-427,733-742,843-853,880-889`.

**Supported.** Implementation follows the separate-repository design: `f4371e4:core/application/src/use_cases/commands/log_progress_checkpoint/handler.rs:23-52`. Ebook integration then bridges the contexts through a dedicated service (`d6a4904`), while its older event-store port promises recording an event once and applying its delta: `8373bd5:core/application/src/use_cases/integration/ebook_event_store.rs:18-23`. This is evidence of use-case-specific delivery contracts, not one universal sync mechanism.

### 6. LiftLog's ownership changed for concrete compatibility reasons

**Direct.** `54c6888` (Daniel Dratschuk, 2026-08-13) says backup ingestion enabled the stock Play Store app “with no fork”; parsing was isolated behind a parser port. `69c65bd` (2026-10-12) removes it because the app's table/backup feature no longer exists and ingestion never cleared a day. This is a justified retirement of a failed contract, not grounds for condemning all existing integration code.

`05c485b` explicitly implements named-day replacements, omission preservation, empty-day deletion, and harmless identical resends. `1ebfb93` records frozen total-weight data and accepted delayed-request overwrites. Exact contract: `HEAD:docs/liftlog-integration.md:12-48,71-73`.

**Direct.** The phone owns finished-workout capture and a durable changed-date queue. Queueing shares the session-write transaction; sequence-matched acknowledgements preserve edits made during a push; payloads are rebuilt from current rows. But ordering is explicitly accepted only for “one phone on a tailnet”: `HEAD:logger_applications/LiftLog/docs/LifestatsSync.md:26-44,54-81`.

## Contradictions

- **Direct text versus implementation:** inward-only/adapters-never-imported (`4177790:docs/ARCHITECTURE.md:94-102`) versus `cdec4f0`'s concrete renderer import. Preserve statelessness; renderer placement remains changeable.
- **Documentation overclaims:** the root guide says zero external domain dependencies, but even `35c25d4:core/core/Cargo.toml:9-18` already includes chrono/serde/etc. “Pure” and “no third-party crates” have not been equivalent historically.
- **Compile-time claim has limits:** architecture documentation promises prevention of derived logging; the public enum-based command still rejects derived/mismatched combinations at runtime (`HEAD:core/application/src/use_cases/commands/log_value/handler.rs:149-160`), with a mismatch test present since `f72350e:core/application/src/use_cases/commands/log_value/handler.rs:285-320`.
- **ADR chronology:** progress document says Proposed/2026-05-02, but Git adds it in `ef786b7` on 2026-07-29, after May implementation. Treat it as explicit recorded rationale, not proven contemporaneous approval.
- **Changed premise:** the accepted one-phone overwrite risk conflicts with the current confirmed multiple-device requirement. Durable acknowledgement is not conflict resolution.

## Gaps and Additional Leads

**Unknown:** no inspected record establishes general queued-command idempotency, cross-device conflict ownership, cache freshness, revision preconditions, or a shared offline protocol. The bounded searches found integration-specific mechanisms instead. No reason was found for events ports remaining packaged in `event_impl` or the no-op default.

No ticket, chat, analytics, observability, or unrelated personal-account search was performed: this delegated slice is source history only. Parent owns cross-source synthesis. One external lead is commit `db50932`'s `Claude-Session: https://claude.ai/code/session_01R6i2n9groH617L39vkGiJA`, concerning goal API semantics; recorded without opening it. No PR/ticket leads emerged.

## Preserve / Change / Avoid / Risk Constraints

- **Preserve — Direct:** domain rules behind explicit ports; typed adapter-to-application requests; read/write separation; application orchestration; centralized composition. Preserve behavioral contracts rather than historical folder names.
- **Preserve — Direct:** stateless render generation; missing-value/default/override/suppression distinctions; separate progress canonical history; source-event deduplication; explicit effective dates; LiftLog clear/replace/omit semantics and durable edit acknowledgement.
- **Change — Inferred planning consequence:** define device/source authority, operation identity, server ordering/conflict checks, and cache reconciliation before reusing one-phone replacement semantics for multiple devices. Revisit manual submissions lacking retry identity.
- **Change — Supported:** remove the concrete renderer dependency without restoring stored plots; distinguish optional notifications from durability. Historical rationale protects neither PNG rendering in application nor a no-op publisher.
- **Avoid — Direct/historical:** forcing progress into daily statistics, generic cleanup before semantic need, restoring obsolete backup parsing, or assuming every integration has identical canonical data. Historical rejection of microservices is a caution against adding deployment boundaries merely to express code boundaries.
- **Risk — Inferred:** offline heartbeat replay cannot automatically mean live heartbeat traffic; restart-loss acceptance is narrower than durable queued writes. Retrying additive logs needs deduplication; retrying snapshots needs ordering/authority. These are separate hazards and require explicit design decisions, not a claim that existing code already handles them.
