# Domain and application semantics architecture exploration

Read-only research, 2026-10-22. Repository scope: `/home/daniel/.local/share/src/lifestats`. Assigned angle: product/domain model, public Rust APIs and ports, commands/queries, source/derived values, goals, progress, serialization, and seams for a new frontend/logger. No repository, database, service, Git, or external-state mutations were performed. Only this report was written. No delegation.

Evidence convention: **Implementation** means directly read source; **Intent** means documentation or comments; **Inference** means a consequence or design recommendation inferred from that source. Line references are relative to the repository root. Findings were not exercised against a live database or server.

## Overview

**Implementation:** The strongest existing boundary is between daily statistics and bounded progress items. Daily statistics hold one canonical value per statistic/date, with per-type update and goal behavior. Progress items hold absolute checkpoints with source-event identity, distinguish current position from furthest position, and derive daily progress. Keep these as sibling product concepts, rather than fitting progress into `AnyStatistic` (`core/core/src/statistic/ports/statistic_repository.rs:26`, `core/core/src/progress/ports/progress_repository.rs:16`, `core/application/src/use_cases/queries/get_progress_summary/handler.rs:33`).

The daily-statistic Rust types are useful, pure calculation objects, but the complete write boundary is weaker than the documentation claims. Callers supply both an arbitrary ID and an arbitrary loaded statistic; the repository accepts untyped JSON for any ID; direct write paths bypass command validation and events. Derived statistics exist as enum variants and metadata containers, but have no implemented calculation pipeline and cannot be persisted by the current SQLite mapper. These are concrete capability and integrity gaps, not reasons to discard the existing domain model.

**Intent versus implementation:** `docs/ARCHITECTURE.md:116` describes `SourceStatistic`/`DerivedStatistic`, `Logable`/`Computable`, and compile-time protection. Actual code has `Statistic`, `LogableStatistic`, `IntegerStatistic`, `AnyStatistic`, and no `Computable` implementation in the searched active Rust tree. `core/core/Cargo.toml:9` also contradicts the root guide's literal “zero external dependencies”; chrono, serde, thiserror, etc. are present. Those helper dependencies do not themselves introduce adapter coupling. More material are raw `serde_json::Value` in the repository port and an application dependency that directly invokes a concrete renderer (`core/application/src/use_cases/queries/get_daily_timeline_plot/handler.rs:39`).

## Key Concepts

### Daily statistics and values

- `Statistic` associates a native `LogValueType` and matching `GoalType`, and exposes metadata plus the current default goal and its validity date (`core/core/src/statistic/traits/base_statistic.rs:7`).
- Four direct types implement `LogableStatistic`: integer amount → `i64`, habit → `bool`, point in time → `NaiveTime`, time spans → `TimeFrameList`. Their update calculations delegate to pure policy objects (`core/core/src/statistic/types/logable_statistics/integer_amount.rs:25`, `core/core/src/statistic/types/logable_statistics/habit.rs:26`, `core/core/src/statistic/types/logable_statistics/point_in_time.rs:31`, `core/core/src/statistic/types/logable_statistics/time_spans.rs:40`).
- `LogValuePayload` has four typed variants and explicit policy values. It excludes a derived payload but does not tie the payload, ID, and supplied statistic together (`core/application/src/use_cases/commands/log_value/command.rs:17`).
- `AnyStatistic` is a closed enum over six types and intentionally does not implement `Statistic`; it dispatches common operations and performs runtime downcasts/JSON conversions (`core/core/src/statistic/any_statistic.rs:80`, `:105`, `:113`, `:197`). The enum is a reasonable finite product sum, not inherently problematic coupling.
- `IntegerStatistic` provides numeric projection, not a universal quantity/unit model. Projection means boolean 0/1, seconds since midnight, interval duration seconds, or ordinary amount depending on the type (`core/core/src/statistic/traits/integer_statistic.rs:3`; corresponding concrete implementations).
- Summing stores a vector of source IDs; TimeDifference stores two IDs. Constructors accept untyped `StatisticId`s without verifying existence/type, and the structs expose only source IDs, metadata, goals, and integer conversion (`core/core/src/statistic/types/derived_statistics/summing_statistic.rs:49`, `core/core/src/statistic/types/derived_statistics/time_difference.rs:50`).

### Goals and missing values

Daily goals are genuinely tri-state: `NotSet` means consult the default; `Suppressed` means no goal even if a default exists; `Goal(JSON)` means explicit override (`core/core/src/statistic/value_objects/goals/goal_query_result.rs:3`). SQL distinguishes absence of a row from a row with NULL goal (`core/infrastructure/persistence/src/repository.rs:389`). A date with no applicable goal is distinct from an applicable goal with no logged value.

`Goal::get_percentage_achieved` accepts `Option<&LogValue>` deliberately (`core/core/src/statistic/value_objects/goals/goal_trait.rs:6`). Missing habit values act as false, so avoiding a bad habit succeeds without an explicit log (`habit_goal.rs:23`). Missing amount values act as zero; “at most” goals count zero as 100% (`amount_goal.rs:34`). Missing point-in-time values score zero (`finished_in_time_goal.rs:35`). TimeSpans goals compare total actual duration to target duration × efficiency, not compliance with the target intervals' clock positions (`time_spans_goal.rs:36`). A zero-duration effective target scores 100% (`:44`).

Achievement is percentage ≥100 (`goal_trait.rs:10`). Percentages can exceed 100 and colors distinguish overachievement (`core/application/src/services/plot_data_builder.rs:229` and tests at `:456`); a redesign must not blindly clamp every percentage to 100.

### Progress items

A `ProgressTarget` is a positive bound with an explicit unit; a checkpoint is a nonnegative absolute position with UTC `recorded_at`, independent `effective_date`, optional source/event identity, and note (`core/core/src/progress/item.rs:50`, `core/core/src/progress/checkpoint.rs:7`). `NewProgressItem` and `NewProgressCheckpoint` separate creation inputs from loaded entities. Typed ports accept these objects instead of arbitrary JSON (`core/core/src/progress/ports/progress_repository.rs:22`).

Progress is not a default/daily goal. Backwards movement changes latest position without subtracting earned progress. Daily deltas use a monotonically increasing frontier. This is explicit documented product intent, substantially implemented (`docs/plans/progress-items-architecture-adr.md:42`, `:76`; `core/application/src/use_cases/queries/get_progress_daily_deltas/handler.rs:48`).

## How It Works

### Representative integer logging chain

1. Router exposes `POST /statistics/{id}/log/{date}` (`core/infrastructure/web_api/src/routes/mod.rs:43`). The web request contains `value: String` and optional policy string (`core/infrastructure/web_api/src/dto/statistic.rs:29`).
2. The adapter loads metadata for the path ID, parses value according to the concrete enum variant, and chooses default policy: integer Sum, habit/point Overwrite, TimeSpans Append (`core/infrastructure/web_api/src/routes/statistics.rs:140`, `:155`, `:173`, `:197`, `:228`, `:251`). Parsing belongs at this transport boundary; duplicating the domain rule there does not.
3. It constructs `LogValueCommand { statistic_id, date, payload, statistic }` and invokes a newly instantiated generic handler (`statistics.rs:265`).
4. Handler loads raw current JSON **by command ID**, matches the supplied statistic and payload, deserializes, invokes the pure calculation, serializes result (`core/application/src/use_cases/commands/log_value/handler.rs:48`, `:56`, `:59`, `:68`). Integer Sum adds to current; Overwrite is unchanged for equal value; Ignore retains an existing value (`core/core/src/statistic/value_objects/update_policies/amount.rs:14`).
5. If changed, handler saves JSON by command ID and publishes `Event::ValueUpdated(id)` (`log_value/handler.rs:164`). SQLite performs an upsert keyed by `(statistic_id,date)` (`core/infrastructure/persistence/src/repository.rs:304`). These separate read and save calls do not form an atomic read-modify-write operation.
6. Bootstrap's publisher is a no-op (`lifestats-bootstrap/src/lib.rs:32`, `:74`). Publishing in this composition does not refresh subscribers.

**Failure behavior:** Metadata not found becomes 404 in this adapter; input parsing becomes 400. Payload/statistic mismatch or derived statistic inside the command becomes `ApplicationError::Infrastructure`, not a typed input failure (`log_value/handler.rs:149`). This route wraps handler errors as 500 (`statistics.rs:280`), despite a general `ApiError::from(ApplicationError)` implementation supporting 400/404 elsewhere (`core/infrastructure/web_api/src/error.rs:130`). Raw stored JSON failing deserialization errors the operation. Overlap in UpdateEndValue and repeated default Append can panic; see finding 1.

### Goal setting and read/evaluation chain

1. `PUT /statistics/{id}/goals/{date}` accepts `kind: goal` plus `HashMap<String,String>`, or `kind: suppressed`; default-goal requests also use the map (`core/infrastructure/web_api/src/dto/goal.rs:14`).
2. Adapter loads statistic type and translates strings to `GoalPayload`; the default branch has its own parser while the daily branch uses `parse_hashmap_into_goal_payload` (`core/infrastructure/web_api/src/routes/goals.rs:94`, `:320`).
3. `SetDailyGoalCommandHandler` reloads metadata, matches type, constructs concrete goals, serializes them, or stores `None` for suppression; it then emits GoalUpdated (`core/application/src/use_cases/commands/set_daily_goal/handler.rs:72`, `:99`). Type mismatches remain runtime errors.
4. Default setting instead loads metadata, chooses host-local today, mutates the entity, writes updated metadata and separately overwrites the current default goal (`core/application/src/use_cases/commands/set_default_goal/handler.rs:50`, `:54`, `:133`). The domain method merely replaces goal and date (`core/core/src/statistic/traits/base_statistic.rs:45`). There is no historical flush here.
5. `GET .../goals/{date}` returns the stored **override**, not a fully resolved goal (`core/application/src/use_cases/queries/get_daily_goal/handler.rs:31`). Its HTTP response preserves all three states (`core/infrastructure/web_api/src/dto/goal.rs:44`), unlike the stale null response description in `docs/goal-overrides.md:45`.
6. Range reads use `GoalResolver`: load overrides, read current default from metadata, apply override > suppression > current default if date ≥ valid-since (`core/application/src/services/goal_resolver.rs:61`, `:93`). `any_goal_to_json` deliberately serializes the inner concrete goal because serializing `AnyGoal` itself would add an enum wrapper (`:12`).
7. `DefaultDailyAchievementEvaluator` bulk-loads logs, resolves goals, deserializes per variant, invokes `Goal`, and emits one `DayEval` per date (`core/application/src/services/daily_achievement_evaluator.rs:64`, `:123`). TimeSpans additionally unions overlapping/adjacent frames before evaluation (`:142`, `:230`).
8. Score caps optional daily contributions then averages; performance averages active goal days and computes EWMA trend; streak scans newest→oldest over 90 days, skips no-goal days, and stops at first miss (`core/application/src/use_cases/queries/get_statistic_score/handler.rs:44`, `core/application/src/services/goal_performance_analyzer.rs:65`, `core/application/src/use_cases/queries/get_streak/handler.rs:52`). Plot-data calculation has a separate evaluation path (`core/application/src/use_cases/queries/get_plot_data/handler.rs:47`).

**Failure behavior:** Resolver/evaluator propagates storage and deserialization errors, except failure serializing the current default is converted into absence with `.ok()` (`goal_resolver.rs:80`). Default goal's two persistence calls can partially succeed. A default-goal edit can alter historical results even when no logs change; see finding 2.

### Progress logging and read chain

1. HTTP accepts independent effective date and optional recorded time, defaults omitted recorded time to `Utc::now`, then creates the command (`core/infrastructure/web_api/src/routes/progress.rs:129`).
2. `LogProgressCheckpointCommandHandler` loads item, rejects position > target, creates validated `NewProgressCheckpoint`, and calls typed repository (`core/application/src/use_cases/commands/log_progress_checkpoint/handler.rs:24`). Constructor rejects negative position and source_event_id without source (`core/core/src/progress/checkpoint.rs:30`).
3. SQLite inserts transactionally, returns Inserted or the prior Duplicate checkpoint identified by `(item,source,source_event_id)` (`core/infrastructure/persistence/src/progress_repository.rs:164`). HTTP returns 201 for Inserted and 200 for Duplicate (`routes/progress.rs:143`). This explicit outcome is a useful ingestion contract.
4. Summary reads latest by `(recorded_at DESC,id DESC)` and max position, calculates fractions 0..1, and completion from furthest (`progress_repository.rs:203`; `get_progress_summary/handler.rs:23`). Daily deltas read ordered checkpoints through a UTC date cutoff and attribute positive frontier increases to effective dates (`get_progress_daily_deltas/handler.rs:33`). Target edits cannot go below furthest observed position (`core/application/src/use_cases/commands/update_progress_target/handler.rs:29`).

### Other writers and the actual command boundary

Session heartbeat persistence applies `TimeFrameListUpdatePolicy::UpdateEndValue` directly, then calls repository, bypassing `LogValueCommand` and its event (`core/application/src/use_cases/session_tracking_service.rs:478`). Fitbit transforms and directly replaces JSON (`core/application/src/use_cases/commands/sync_fitbit_data.rs:324`). LiftLog replaces named days, deletes empty days, compares JSON for unchanged values, and writes directly (`core/application/src/use_cases/integration/service.rs:107`, `:349`). These are valid distinct ingestion semantics—absolute snapshots differ from additive manual logs—but there is no shared typed persistence/invalidation boundary beneath them.

## Where Things Live / Public Surface for New Clients

- Domain primitives, six statistic variants, policy calculations and goals: `core/core/src/statistic/`, shared date/interval/percentage objects: `core/core/src/shared/`; progress: `core/core/src/progress/`.
- Synchronous command/query interfaces: `core/application/src/cqrs.rs:12`; handlers under `core/application/src/use_cases/{commands,queries}/`; bulk goal resolution/evaluation under `core/application/src/services/`.
- Daily-statistic port: `core/core/src/statistic/ports/statistic_repository.rs:11`; typed progress port: `core/core/src/progress/ports/progress_repository.rs:22`; exact-tag integration lookup is an application port (`core/application/src/statistic/ports.rs:6`). Port ownership in the application is legitimate for application-specific integration needs; “all traits must be in domain” is not a useful migration objective by itself.
- Persistence adapters and conversion schema: `core/infrastructure/persistence/src/{repository,progress_repository}.rs`, `db/mappers.rs`; HTTP transport and DTOs: `core/infrastructure/web_api/src/{routes,dto}/`; composition: `lifestats-bootstrap/src/lib.rs:45`.
- Rust logger surface today: call `build_services`, then instantiate command/query handlers with exposed repository/publisher. This works, but exposes concrete `SqliteStatisticRepository` and bypass paths (`lifestats-bootstrap/src/lib.rs:45`). A new remote/mobile logger should use server transport, not share a mobile SQLite implementation of this internal Rust repository contract.
- HTTP inventory: statistics list/create, loggable list, item/delete, per-date value/timeframe write and logged read (`routes/mod.rs:29`); totals, score, achievements, performance, streak (`:53`); progress items/target/checkpoints/summary/deltas (`:78`); default/daily goals (`:102`); stateless plot data/image (`:115`); session heartbeat endpoints (`:129`); ingestion snapshots/provisioning (`:147`). No common computed-value endpoint is implemented.
- Wire contract is asymmetric: log request value is a string, including JSON encoded inside that string for intervals; goal input is a string map with keys `target`/`occurred`/`deadline`; stored/read goal objects use `target_amount`/`target` native values (`dto/statistic.rs:29`, `dto/goal.rs:6`, concrete goal definitions). Metadata response has type and default-goal JSON but omits tags, units, source IDs and default validity date (`dto/statistic.rs:56`). Progress DTOs are much more explicit and use fractional percent fields (`dto/progress.rs:114`).

## Prioritized Findings

### 1. P1 — Ordinary second TimeSpans write can panic

**Implementation/evidence:** `TimeFrameListUpdatePolicy::Append` is default and its existing-value branch is `todo!()` (`core/core/src/statistic/value_objects/update_policies/time_frame_list.rs:5`, `:21`). Both generic log and single-timeframe HTTP adapters default to Append (`core/infrastructure/web_api/src/routes/statistics.rs:251`, `:330`). UpdateEndValue also uses `expect` on legitimate overlap errors (`update_policies/time_frame_list.rs:26`).

**Consequence/inference:** Two valid disjoint writes on the same day follow None→first value→Some→panic. Overlapping user/logging sessions can panic rather than return a domain conflict. This is directly relevant before adding more independent loggers. Existing heartbeat paths choose UpdateEndValue and LiftLog replaces a whole day, so those routes do not establish that default Append works. Confidence: high, source-level deterministic path; not run.

**Candidate seam:** Make interval policy calculation return a domain result; implement an explicit append/union/reject-overlap product policy and use it beneath all writers. Distinguish expected input conflict from impossible internal state.

### 2. P1 — Changing the default goal discards historical applicability and is not atomic

**Implementation/evidence:** Setting a default replaces the single goal/start date with host-local today (`set_default_goal/handler.rs:54`); it updates metadata then separately saves the default (`:133`). Resolver returns no default before the new valid-since (`goal_resolver.rs:101`). SQLite updates metadata only (`repository.rs:157`) and upserts one default row (`:501`, `:89`). No historical goal flush or history write appears in this chain. Deleting the default removes that one row (`delete_default_goal/handler.rs:37`).

**Consequence/inference:** A default active since Monday, changed Friday, makes Monday–Thursday become no-goal days unless explicit overrides exist. Score/streak/performance can change retroactively or exclude that history. Deleting the default similarly removes historical default-based goals. A failure between metadata and default save also pairs the old goal with a new start date. Whether retroactive goal erasure is ever intentional is a product decision, but the implemented behavior is clear. Confidence: high mechanics; intended history semantics unverified.

**Candidate seam:** Atomic effective-dated goal history, or atomic materialization of the replaced default over prior dates; preserve daily override/suppression precedence. Use an explicit effective date/user-day policy, not an invisible host-local clock. `SetDefaultGoalCommand` currently has no date (`command.rs:10`).

### 3. P1 — Public logging API trusts caller-supplied identity/type; compile-time guarantee is narrower than claimed

**Implementation/evidence:** Command contains public independent `statistic_id`, `statistic`, and payload (`log_value/command.rs:40`). Handler reads/saves by ID and uses supplied statistic for calculation, with no equality check or metadata reload (`handler.rs:48`, `:56`, `:59`, `:167`). Port exposes `save_log_value(id,date,Value)` for any ID (`statistic_repository.rs:32`); SQLite serializes and upserts without checking statistic type (`repository.rs:304`).

**Consequence/inference:** A Rust logger can pass a loaded integer statistic A with habit statistic B's ID and integer payload; if B has no value, handler stores an integer under the habit ID. It can likewise use an unrelated direct statistic's instance with a derived target ID in an adapter that supports derived metadata. Existing web adapter correctly loads by path ID, so this is a shared-library boundary hole, not evidence of a web exploit. Raw repository writers bypass all such checks. Concrete derived types lacking `LogableStatistic` is useful compile-time protection for generic domain code, but does not protect the public application/persistence surface. Confidence: high static path.

**Candidate seam:** Command accepts ID plus typed intent only; application loads canonical target once and validates payload/type there. Use typed log/goal enums at internal ports, with JSON confined to storage/wire codecs. Keep raw save methods private to adapters if possible.

### 4. P1 — Amount goal score can wrap so overachievement becomes failure

**Implementation/evidence:** Amount percentage computes an `i64` ratio then casts to `u8` (`core/core/src/statistic/value_objects/goals/amount_goal.rs:38`, `:56`); achievement compares resulting byte to 100 (`goal_trait.rs:10`). `Percentage::new` accepts any byte, and float conversion uses saturating float→byte cast rather than range validation (`core/core/src/shared/percentage.rs:17`, `:33`). Same type represents intended 0..100 efficiency and potentially >100 achievement.

**Consequence/inference:** At-least goal 10 with value 30 computes 300 then casts to 44, so it is reported missed and can break a streak. Negative amounts can cast to high success percentages. Large multiplication can overflow. Efficiency 200 is accepted by API `u8` parsing despite error text promising 0..100 (`routes/goals.rs:165`, `:174`, `:383`). Confidence: high Rust conversion semantics; no execution performed.

**Candidate seam:** Separate validated bounded efficiency from a non-wrapping achievement ratio; decide signed amount semantics explicitly. Preserve intentional >100 scores. This is a semantic numeric type problem, not a mandate for dependency-free domain code.

### 5. P1/P2 — Derived statistics are an advertised but nonfunctional capability

**Implementation/evidence:** Create command accepts Summing and TimeDifference and verifies only presence of sources / count of two for difference (`create_statistic/handler.rs:90`). SQLite mapper enumerates four direct types and rejects other variants (`core/infrastructure/persistence/src/db/mappers.rs:29`, `:97`). Constructors have no computation methods. Active-tree search for `compute_value`, `Computable`, `source_ids(` found only the two source accessor definitions. Evaluator reads derived ID's stored logs rather than traversing source IDs (`daily_achievement_evaluator.rs:332`); GetLoggedValue rejects derived (`get_logged_value/handler.rs:35`); dashboard rejects them (`get_dashboard/handler.rs:135`); plot evaluation returns None (`get_plot_data/handler.rs:111`).

**Consequence/inference:** Mobile UI cannot implement the documented derived feature by selecting the exposed type and calling normal reads. Current SQLite create fails, so it is not established that any deployed derived data exists. Completing it needs source-reference persistence, source type/existence validation, graph rules and computed-value queries, not merely a new `Computable` trait. Confidence: high for active path; legacy excluded implementations not exhaustively searched.

**Candidate seam:** Separate statistic definitions from value reading; use a canonical value reader resolving source or derived values, returning explicit units/missing-data behavior. Until implemented, expose actual capabilities so clients do not offer unsupported types.

### 6. P1 for multi-writer rollout — Daily writes have neither atomic increment nor shared event/idempotency semantics

**Implementation/evidence:** Log command independently finds and saves (`log_value/handler.rs:56`, `:166`), SQLite pool connection is acquired separately per call (`repository.rs:288`, `:310`). Sum is default (`amount.rs:8`). Session, Fitbit and LiftLog bypass this handler and its event (`session_tracking_service.rs:478`, `sync_fitbit_data.rs:324`, `integration/service.rs:349`). Progress has a real transaction and duplicate outcome (`progress_repository.rs:164`), whereas normal logs have no source request identity (`log_value/command.rs:40`).

**Consequence/inference:** Two additive requests reading 10 can each save 11, losing one increment. A retry after ambiguous HTTP success can double a sum. Whole-day source snapshots can overwrite edits to the same statistic/day. These are possible schedules, not observed incidents; SQLite serializing each individual write does not make the pair atomic. Adding more loggers increases exposure. No-op bootstrap means current events do not refresh caches, but leaving multiple bypasses would make future invalidation incomplete. Confidence: high vulnerability; incidence unverified.

**Candidate seam:** Name separate intents (add observation with stable request identity, replace authoritative day snapshot, update active interval); execute each in an atomic repository operation or transaction boundary with explicit outcomes. Keep source ownership/conflict rules visible. Do not force snapshot imports through additive semantics.

### 7. P2 — Deserialization bypasses interval invariants and read paths normalize differently

**Implementation/evidence:** `TimeFrame::new` rejects start>end and append maintains disjoint sorted lists (`core/core/src/shared/time_frame_list.rs:61`, `:143`), but both types derive Deserialize (`:19`, `:123`), which directly fills private fields. `TimeFrameList::from_str` delegates to serde without validation (`:302`). Generic HTTP interval writes and goal parsing deserialize JSON directly (`routes/statistics.rs:239`, `routes/goals.rs:162`). Achievement evaluator coalesces overlap (`daily_achievement_evaluator.rs:230`); plot evaluation deserializes raw list and calls Goal directly (`get_plot_data/handler.rs:96`); totals sum raw frames (`get_time_spans_total/handler.rs:49`).

**Consequence/inference:** Overlapping frames 09–11 and 10–12 can enter as JSON and report three hours in achievement evaluation but four in totals/plots; a three-hour target can receive different percentages. Reversed intervals bypass the smart constructor and can trip debug assertions or produce negative durations (`time_frame_list.rs:55`). Private fields alone do not preserve invariants across serde. Confidence: high static path.

**Candidate seam:** Validating wire/storage decoders, one canonical interval normalization method, one daily value/evaluation pipeline consumed by totals, plots, achievement and dashboard reads. Preserve legacy data through explicit migration/repair rather than silently changing answers per consumer.

### 8. P2 — Unit/presentation knowledge leaks into integration tags and wire codecs

**Implementation/evidence:** `integer_amount_scalar` recognizes specifically `source=fitbit` and two metric strings to display amounts as seconds (`core/application/src/services/plot_data_builder.rs:14`). Other integers are treated as numbers regardless of their semantic quantity. Standard metadata DTO omits unit/tags (`dto/statistic.rs:56`). Goal serializers/readers know private object field names and input names differ from output (`dto/goal.rs:6`; `amount_goal.rs:6`; `get_plot_data/handler.rs:58`). TimeSpans plotting uses unadjusted target duration while achievement uses efficiency-adjusted target (`plot_data_builder.rs:188`; `time_spans_goal.rs:41`).

**Consequence/inference:** A new duration logger may need another source-name special case, and a React Native frontend cannot derive an unambiguous display/input contract from numeric values and statistic type alone. Some differences may be intentional visualization policy; explicit unit and effective target fields would let clients choose deliberately. Confidence: high code observation; desired display design unverified.

**Candidate seam:** Definition-level quantity/unit metadata and versioned discriminated transport DTOs, independent of source. Keep compatibility codecs for existing string requests. Put source-specific extraction beside integration code, not in generic plot-data code.

### 9. P2 / product decision — Progress daily range mixes effective dates and UTC recorded-time cutoff

**Implementation/evidence:** Dates are independently supplied (`checkpoint.rs:20`; `routes/progress.rs:130`). Delta query turns requested effective end date into UTC 23:59:59 and filters repository by recorded_at before testing effective_date (`get_progress_daily_deltas/handler.rs:33`, `:54`). Documentation explicitly allows local/source dates (`docs/plans/progress-items-architecture-adr.md:191`) and separately prescribes “up to end” (`:581`).

**Consequence/inference:** A checkpoint recorded Jan 2 at 00:30Z but attributed to Jan 1 is absent from Jan 1-only deltas but can appear when querying Jan 1–2. A late imported/backdated checkpoint has the same issue. Thus narrowing a date range may change whether an effective-date contribution exists. This may encode intended as-of behavior, but query/API does not name a separate as-of timestamp. Confidence: high mechanical example, medium whether classified as bug.

**Candidate seam:** Choose whether daily-delta query means effective-date history or a historical as-of snapshot; separate those parameters. Maintain chronological frontier ordering and include all needed earlier checkpoints.

## Preservation Constraints and Candidate Module Seams

1. Keep `statistics` (per-day values/goals), `progress` (bounded objects/checkpoints), `sessions` (interval capture), and integration ingestion as explicit concepts. Existing progress split is deliberate and working; ADR explicitly rejected `AnyStatistic::Progress` (`docs/plans/progress-items-architecture-adr.md:47`). No evidence supports a microservice split.
2. Preserve daily-goal tri-state, deletion meaning “return to default,” no-goal exclusion from scoring, and no-goal days not breaking streaks. Missing-value rules differ by goal and must remain explicit. Historical default behavior needs a deliberate product decision, not accidental preservation.
3. Preserve direct statistic native types and pure update/goal calculations. chrono, serde derives for safe shapes, error enums, and numeric helpers are legitimate dependencies. Remove real adapter dependencies, not all third-party crates.
4. Introduce a narrow typed application facade for clients: create/list capabilities; submit ID+intent; read canonical daily values/achievements; set goals with explicit effective dates. It can wrap existing handlers while centralizing canonical target loading and transactional operations. No need to rewrite CQRS markers or replace generic handlers merely for aesthetics.
5. Keep JSON encoding compatibility as an adapter concern. Existing DB rows and external clients rely on concrete goal/value shapes and string-based request formats. Version transport additions, preserve existing IDs/tags, and establish migration evidence before replacing storage. `any_goal_to_json` exists because enum serialization is already a known shape hazard (`goal_resolver.rs:12`).
6. Preserve source snapshot semantics: LiftLog only replaces named days, empty named days remove values, untouched days remain unchanged (`integration/service.rs:45`, `:349`). Its intervals use phone wall-clock timestamps (`liftlog_days.rs:4`), union overlaps, and apply a deliberate 60-second minimum (`integration/service.rs:129`, `:160`). Separate that source policy from generic interval validation.
7. Preserve progress duplicate outcome and stable source identity, latest/furthest distinction, stable recorded_at/id ordering, positive target bounds and explicit units. Enforce cross-object invariants atomically before increasing concurrent writers; current command-level checks for target/checkpoint bounds are separate from persistence (`log_progress_checkpoint/handler.rs:28`; `update_progress_target/handler.rs:29`).
8. Use a single canonical daily read/evaluation service for all product views. Renderer contracts can remain separate; the concrete timeline renderer call belongs behind a rendering port or outside application orchestration (`get_daily_timeline_plot/handler.rs:39`).

Suggested sequence for parent design: first settle value/goal/date/ownership semantics and fix deterministic panic/numeric holes; then strengthen one write boundary and temporal goal persistence; then unify daily reads; finally expose an explicit versioned frontend contract and complete or hide derived capabilities. This sequence is an inference, not an implementation plan executed here.

## Gotchas

- “Default policy” differs by caller: fixed-value domain default is Ignore (`fixed_value.rs:5`), web uses Overwrite. Reusing `Default::default()` in a new logger changes behavior.
- Amount Sum marks changed even for adding zero; this is documented generic-policy behavior, not necessarily a bug (`amount.rs:17`).
- “Percent” means different scales: `Percentage.as_num()` is byte percentage; progress `percent_latest`/`percent_furthest` are fractions 0..1 (`get_progress_summary/handler.rs:40`, `:61`).
- `Seconds::from_i64` clamps negatives, while conversion from chrono Duration casts signed seconds to u64 (`core/core/src/shared/seconds.rs:7`, `:19`). Do not infer one uniform duration validation policy.
- DateRange is inclusive and validates ordering (`core/core/src/shared/date_range.rs:4`, `:19`). Clock's default today reads UTC's naive date despite a local-time comment (`core/application/src/time/clock.rs:9`); default-goal/create use host Local and Fitbit has configured timezone (`lifestats-bootstrap/src/lib.rs:122`). Day policy needs an explicit boundary.
- The general ports use synchronous calls. Async trait for exact-tag lookup is separate; wrapping synchronous handlers in async routes does not make database calls async.
- Metadata `statistic_type` is both an actual enum and sometimes a tag. The AnyStatistic conversion uses the enum; conversion from bare metadata can report Unknown (`core/infrastructure/web_api/src/dto/statistic.rs:83`, `:102`). Avoid taking tags as authoritative type identity.
- Docs contain proposed types/paths and stale goal response/scoring details (`docs/ARCHITECTURE.md:84`, `docs/statistics.md:19`, `:21`, `:23`, `docs/goal-overrides.md:45`). They are product context, not evidence of existing capability.

## Confidence and Verification Gaps

High confidence in traced handlers, policy branches, DTO shapes, mapper capability gap, and static mismatch/overflow/panic paths. Medium confidence in intended historical goal semantics, interval overlap policy, progress as-of behavior, and desired unit/display behavior. These require product decisions or parent synthesis.

No live data was inspected, no service requests were sent, no tests/builds were run, and no production writes occurred. No secret-bearing environment/config files were read. Existing tests were inspected only as source. No claim is made about actual historical user data loss, observed races, running binary version, excluded logger correctness, or deployed derived statistics. The active-tree negative search for derived computations was limited to core, bootstrap, and workspace MIDI logger; old excluded code may contain obsolete alternatives.

Useful follow-up verification after authorization for isolated scratch tests: second same-day interval append; mismatched command ID/instance using a mock repository; amount 30 against target 10; default goal replacement across dates; overlap JSON through total versus achievement/plot; two concurrent additive writes; progress effective-date range consistency. These are cheap targeted checks; a whole-repo rewrite is not justified before establishing intended behavior.

## Files Read / Inspected

Full files or selected numbered ranges unless marked search-only. Paths here abbreviate shared prefixes; the main report contains full relative citations.

- Guidance: root `AGENTS.md` (symlink to `CLAUDE.md`); user-provided LiftLog AGENTS; `logger_applications/LiftLog/docs/index.md`; `/home/daniel/.local/share/src/pstack-t3/.pstack-t3/current/{t3-adapter,pstack-models}.md`; `/home/daniel/.codex/skills/how/SKILL.md` and `references/explorer-prompt.md`. No nested guidance found under core/bootstrap/docs/events; root CLAUDE.local.md absent.
- Manifests: root `Cargo.toml`, `core/core/Cargo.toml`, `core/application/Cargo.toml`.
- `core/core/src/`: `lib.rs`; `statistic/any_statistic.rs`; `statistic/traits/{base_statistic,logable_statistic,integer_statistic}.rs`; `statistic/ports/{statistic_repository,error}.rs`; `statistic/types/logable_statistics/{integer_amount,habit,point_in_time,time_spans}.rs`; `statistic/types/derived_statistics/{summing_statistic,time_difference}.rs`; `statistic/value_objects/statistic_metadata.rs`; `statistic/value_objects/goals/{goal_trait,goal_query_result,amount_goal,habit_goal,time_spans_goal,finished_in_time_goal}.rs`; `statistic/value_objects/update_policies/{amount,fixed_value,time_frame_list}.rs`; `shared/{ids,logging_data,percentage,seconds,time_frame_list,date_range}.rs`; `shared/ports/error.rs`; `progress/{item,checkpoint,unit}.rs`; `progress/ports/progress_repository.rs`.
- `core/application/src/`: `cqrs.rs`; `statistic/ports.rs`; `time/clock.rs`; `dto/{statistic,logged_value}.rs`; `services/{goal_resolver,daily_achievement_evaluator,goal_performance_analyzer,plot_data_builder}.rs`; `use_cases/commands/log_value/{command,handler}.rs`; `use_cases/commands/set_daily_goal/{command,handler}.rs`; `use_cases/commands/set_default_goal/{command,handler}.rs`; `use_cases/commands/create_statistic/{command,handler}.rs`; `use_cases/commands/{delete_default_goal,delete_daily_goal,log_progress_checkpoint,update_progress_target}/handler.rs`; `use_cases/commands/log_progress_checkpoint/command.rs`; `use_cases/commands/sync_fitbit_data.rs`; `use_cases/queries/{get_logged_value,get_daily_goal,get_daily_achievements,get_statistic_score,get_streak,get_integer_amount_total,get_time_spans_total,get_time_spans_metadata,get_plot_data,get_daily_timeline_plot,get_daily_summary,get_dashboard,get_progress_summary,get_progress_daily_deltas}/handler.rs`; `use_cases/queries/get_logged_value/query.rs`; `use_cases/queries/get_loggable_metadata.rs`; `use_cases/integration/{service,liftlog_days}.rs`; `use_cases/session_tracking_service.rs` (selected methods); session_manager search-only.
- Adapters: `core/infrastructure/persistence/src/{repository,progress_repository}.rs`; `core/infrastructure/persistence/src/db/mappers.rs`; `core/infrastructure/web_api/src/routes/{mod,statistics,goals,progress}.rs`; `core/infrastructure/web_api/src/dto/{statistic,goal,progress}.rs`; `core/infrastructure/web_api/src/error.rs`; `lifestats-bootstrap/src/lib.rs`; `events/event_impl/src/event_port.rs`.
- Documentation: `docs/ARCHITECTURE.md`, `docs/statistics.md`, `docs/goal-overrides.md`, `docs/plans/progress-items-architecture-adr.md` (selected sections). Search-only: default-goal migration references, web e2e/statistics test references. File inventory searches covered active core/application/domain, adapter directories, docs, bootstrap, and relevant nested guidance.
