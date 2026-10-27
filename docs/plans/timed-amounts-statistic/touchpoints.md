# Exhaustive implementation touchpoints

These are the implementation edits. Every file under `sketch/` mirrors an intended edit below. The OpenAPI snapshot and two mandatory docs are additional intended edits.

**compile-checked** means adding the selected variants/trait references makes a missing arm/module fail typechecking (inferred from source; no compiler was run). **silent-if-missed** means the feature can compile but fail or disappear at runtime, omit documentation/tests, or rely on a test rather than a compiler. Where a file contains both kinds, the row names the silent sites explicitly. NEW modules are compile-checked only once their documented callers/exports are present: no compiler can detect an entirely unstarted feature.

## Core domain

| File | Enforcement | Required change |
|---|---|---|
| `core/core/src/lib.rs` | compile-checked | Export source domain module. |
| `core/core/src/opennutritracker.rs` | compile-checked | NEW `OpenNutriTrackerMetric` enum (single identity key: `tag()`, `display_name()`, `new_statistic()`, `all()`), complete two-metric day, replacement/error and one atomic store port. |
| `core/core/src/shared/mod.rs` | compile-checked | Export new shared value module. |
| `core/core/src/shared/timed_amount_list.rs` | compile-checked | NEW immutable entry (`Option<NaiveTime>`, amount) and list types, strict serde/FromStr through the constructors, canonical multiset order (unknown times first), bounded checked `total()`; pure tests. |
| `core/core/src/statistic/any_statistic.rs` | compile-checked | TimedAmounts in AnyStatistic/AnyGoal, dispatch macro, JSON projection, default goal and legacy streak typed arms. No unused convenience APIs. |
| `core/core/src/statistic/types/logable_statistics/mod.rs` | compile-checked | Export concrete statistic module. |
| `core/core/src/statistic/types/logable_statistics/timed_amounts.rs` | compile-checked | NEW native Statistic/LogableStatistic/IntegerStatistic (daily sum via `total()`). |
| `core/core/src/statistic/value_objects/goals/mod.rs` | compile-checked | Export concrete goal module. |
| `core/core/src/statistic/value_objects/goals/timed_amounts_goal.rs` | compile-checked | NEW list-aware bounded AmountGoal adapter, strict goal serde, missing vs zero; pure tests. |
| `core/core/src/statistic/value_objects/update_policies/mod.rs` | compile-checked | Export policy. |
| `core/core/src/statistic/value_objects/update_policies/replace.rs` | compile-checked | NEW overwrite-only default policy, FromStr and equality behavior; pure tests. |

## Application

| File | Enforcement | Required change |
|---|---|---|
| `core/application/src/dto/statistic.rs` | compile-checked | Type variant + `as_str`/summary arms; `FromStr` derived from `EnumIter` over `as_str` (was a silent string match); exhaustive `is_manually_loggable` (TimedAmounts → false); roundtrip test iterates the enum. Add `strum`/`strum_macros` 0.27 to `core/application/Cargo.toml`. |
| `core/application/src/services/daily_achievement_evaluator.rs` | compile-checked | parse_day_value sum and typed eval_range arms; keep TimeSpans merging exclusive. |
| `core/application/src/services/goal_resolver.rs` | compile-checked | Unwrap AnyGoal::TimedAmounts to concrete untagged JSON; resolver history unchanged. |
| `core/application/src/services/plot_data_builder.rs` | compile-checked | Both list and goal drawable matches; sum/target to existing scalar Number, reject interval/circular type. |
| `core/application/src/use_cases/commands/create_statistic/handler.rs` | compile-checked | TimedAmounts arm returns `InvalidInput`: only the source creates these statistics. |
| `core/application/src/use_cases/commands/set_daily_goal/command.rs` | compile-checked | Validated TimedAmountsGoal payload variant; no transport scalar bag in new branch. |
| `core/application/src/use_cases/commands/set_daily_goal/handler.rs` | silent-if-missed | Tuple wildcard requires explicit new goal pairing. Shared helper serves default, daily and range. |
| `core/application/src/use_cases/integration/service.rs` | compile-checked | Trait method + impl, optional store field, `with_opennutritracker_day_store`; passes `today`, no definitions and no generic factory. |
| `core/application/src/use_cases/queries/get_dashboard/handler.rs` | compile-checked | Type name arm; sum via existing integer JSON projection. |
| `core/application/src/use_cases/queries/get_loggable_metadata.rs` | compile-checked | Replace the 4-type allowlist with `StatisticType::is_manually_loggable` (exhaustive). |
| `core/application/src/use_cases/queries/get_plot_data/handler.rs` | compile-checked | Independent typed goal-scoring arm; also validate TimedAmounts plot type before points loop (guard behavior is silent). |

## Persistence

| File | Enforcement | Required change |
|---|---|---|
| `core/infrastructure/persistence/src/db/mappers.rs` | compile-checked | DB enum/`as_str`/typed reconstruction; `FromSql` derived from `EnumIter` over `as_str` (was the severe silent decoder); encoder `_ =>` replaced by explicit `Summing | TimeDifference` arm. Add `strum`/`strum_macros` 0.27 to persistence `Cargo.toml`. |
| `core/infrastructure/persistence/src/lib.rs` | compile-checked | Declare concrete source store implementation module. |
| `core/infrastructure/persistence/src/opennutritracker_day_store.rs` | compile-checked | NEW immediate transaction, source+metric conflict/units, preserve metadata/goals, canonical typed list comparison, both upserts including []. |

## Composition root

| File | Enforcement | Required change |
|---|---|---|
| `lifestats-bootstrap/src/lib.rs` | silent-if-missed | Inject SQLite as OpenNutriTrackerDayStore or endpoint sees missing-store 500. |

## HTTP adapter

| File | Enforcement | Required change |
|---|---|---|
| `core/infrastructure/web_api/src/dto/statistic.rs` | compile-checked | Metadata type-name arm; create-request type comment. No schema field additions. |
| `core/infrastructure/web_api/src/helpers.rs` | compile-checked | DELETE (no callers; duplicates `plot_data_builder`). Remove `pub mod helpers;` from `src/lib.rs:3` and `mod helpers;` from `src/main.rs:3`. Separate small commit. |
| `core/infrastructure/web_api/src/routes/goals.rs` | compile-checked | New goal-parser arm using validated TimedAmountsGoal, existing string-map DTO family. |
| `core/infrastructure/web_api/src/routes/mod.rs` | silent-if-missed | Declare module AND register exact PUT route; missing router registration silently returns404. |
| `core/infrastructure/web_api/src/routes/openapi.rs` | silent-if-missed | Add annotated source path and both request schemas to existing integrations registry. |
| `core/infrastructure/web_api/src/routes/opennutritracker.rs` | compile-checked | NEW strict raw nested request DTO/parse + domain conversion; 204 endpoint, typed error conversion, utoipa path, post-commit events. |
| `core/infrastructure/web_api/src/routes/plots.rs` | silent-if-missed | BOTH image/data endpoint error maps: InvalidInput→400 rather than current blanket500. |
| `core/infrastructure/web_api/src/routes/statistics.rs` | compile-checked | Log parser arm returns 400 "written by its source integration". `create_statistic` (:430) error map → `ApiError::from` so InvalidInput is 400, not 500 (mapping omission is silent). |

## SDK

| File | Enforcement | Required change |
|---|---|---|
| `clients/lifestats-js/src/schemas.ts` | silent-if-missed | Zod statisticTypeSchema name; omission rejects entire list once a new statistic exists. |
| `clients/lifestats-js/src/types.ts` | compile-checked | `StatisticType = z.infer<typeof statisticTypeSchema>` (removes the second hand-written list; check for an import cycle). |

## Web UI

| File | Enforcement | Required change |
|---|---|---|
| `ui/web-ui/src/features/goals/components/DayActions.tsx` | compile-checked | describeDayResult new amount-style GoalData arm; existing null text preserved. |
| `ui/web-ui/src/features/goals/components/GoalFields.tsx` | silent-if-missed | Render AmountFields for new type; add explicit return and never guard so future omissions fail. |
| `ui/web-ui/src/features/goals/insights.ts` | compile-checked | GoalData goalLevel arm returns scalar target; existing count trend code reused. |
| `ui/web-ui/src/features/goals/today.ts` | compile-checked | Both GoalData switches: preserve null as No data; total/cap labels with mg. |
| `ui/web-ui/src/features/goals/useGoalForm.ts` | compile-checked | GoalData to form-values arm; no new form hook. |
| `ui/web-ui/src/lib/goalFormat.ts` | compile-checked | All four GoalData switch arms; amount target/direction/equality, mg in description. |
| `ui/web-ui/src/lib/measure.ts` | compile-checked | TimedAmounts count branch; mg name regex addition is silent-if-missed. |
| `ui/web-ui/src/lib/schemas.ts` | compile-checked | Bounded target form/default/parse/stored-transform branches; preserve raw target_amount ↔ form target mapping. |
| `ui/web-ui/src/lib/statisticTypes.ts` | compile-checked | New Record entry; `derived: boolean` → exhaustive `origin: 'logged' | 'derived' | 'source'`; TimedAmounts is `source`, so it is not offered for creation. Goals stay editable. |
| `ui/web-ui/src/features/statistics/components/CreateStatisticDialog.tsx` | compile-checked | `:210` reads `origin === 'derived'` (forced by removing `derived`). |
| `ui/web-ui/src/types/index.ts` | silent-if-missed | StatisticType union AND GoalData discriminant; numeric DayValue unchanged. |

## Integration/roundtrip tests

| File | Enforcement | Required change |
|---|---|---|
| `core/infrastructure/persistence/tests/timed_amounts_roundtrip.rs` | silent-if-missed | NEW metadata/list/goals/history coexistence and legacy discriminant roundtrips with disposable DB. |
| `core/infrastructure/web_api/tests/integrations_api.rs` | compile-checked | Handwritten IntegrationServiceTrait implementation needs new method. |
| `core/infrastructure/web_api/tests/opennutritracker_days_e2e.rs` | silent-if-missed | NEW real SQLite/router contract, transaction, raw-token, null time, generic create/log 400, read/plot consistency and OpenAPI cases. |

## OpenAPI and mandatory documentation

| File | Enforcement | Sections to update |
|---|---|---|
| `core/infrastructure/web_api/openapi.json` | silent-if-missed | Regenerate snapshot from routes/openapi.rs; required caffeine/nicotine arrays, strict nested fields and constraints, path date and error/status responses. Existing snapshot check catches drift at test time. |
| `core/infrastructure/web_api/README.md` | silent-if-missed | API Routes Reference → Integrations (new PUT; no new POST); Request/Response DTOs → new “Replacing OpenNutriTracker daily intakes” section beside Beurer/Yomikata; Creating a Statistic (TimedAmounts is source-provisioned only, 400); Logging a Value (TimedAmounts rejected with 400, written via the source PUT); Checking if Value is Logged ([] vs absent); Setting a Goal (native-list total target, null vs []); Getting Daily Achievements (sum value); Plots (scalar totals/types, no dose markers); Error Handling (reuse structured integration errors, generic invalid-input 400); Adding a New Integration (atomic source day-store pattern and bootstrap injection). Reproduce contract.md's request/success/errors, attributed-day/time/units, ownership, atomicity, provisioning, omission/empty/replay and stale-write limitations. |
| `.claude/skills/lifestats/SKILL.md` | silent-if-missed | Domain rules/type inventory; Logging values table (TimedAmounts not manually loggable; written by the source PUT); Finding statistics (absent from /statistics/loggable); Reading data (raw list vs numeric sum, [] vs absent); Goals (target/is_at_least, missing scores0); integration paragraphs before Sessions (exact new PUT and complete-day example/errors, auto-provision/no POST, `time: null` for unknown times); Plots (allowed scalar types/no markers). Fix stale “OpenAPI this route only” sentence to its actual integration scope. |

## Deliberately unchanged

- No SQL migration, `db/schema.rs`, `db/models.rs`, generic `repository.rs` production methods, or new ledger/binding/revision table. New type text plus immutable JSON is sufficient. New roundtrip tests live in the dedicated test file, not the repository source test module.
- No new crates. `strum`/`strum_macros` 0.27 (already in `core/core`) are added to the application and persistence manifests for the derived parsers. Pure domain computation only.
- No `StatisticRepository` trait method or unrelated repository mocks. The source port is separate, implemented on the existing SQLite adapter.
- No new `from_`/`into_`/`as_` or typed setter helpers on AnyStatistic without callers.
- No `set_default_goal`/`set_daily_goal_range` handler edits: both use the shared parser; verify them through behavior. No default-history/resolver semantic change beyond AnyGoal serialization.
- No list-bearing DayValue, `dto/daily_achievements.rs`, metadata unit/tag addition, new range-total route, or source-only raw read. Existing raw GET carries the list, existing scalar reads carry the sum.
- No `core/plot-domain` changes, plotting renderer changes, interval timeline/summary/allocation changes, or TimeSpans metadata filter changes. Backend labels retain `(mg)` through the statistic name; UI suffix follows existing name-unit recognition.
- No UI API, raw-list viewer, DayActions value-entry UI, TrendChart/Rail changes: count values work through existing paths. TimedAmounts is not offered for creation.
- No `log_value` command/handler change: there is no `LogValuePayload::TimedAmounts`, so the generic write path cannot reach this type (the route rejects it first).
- No JS SDK client/browser method needed by the separate Flutter sync work. Update the SDK's Zod enum so existing list calls keep working once a TimedAmounts statistic exists.
- No OpenNutriTracker Dart/app files are edited in this backend design. The client must add mg metrics and meet this contract in separate work.
- No generic source BundleFactory registration or `POST /integrations/opennutritracker` support. PUT provisioning is the single path.

## Verification ownership

Domain tests belong inside the three new value/goal/policy modules. Router/SQLite and persistence tests are listed above and sketched with unfinished test bodies. `README.md` → Verification plan gives the behavioral proof obligations. UI/SDK typecheck and live UI inspection are implementation-phase checks; no test suite or build was run in this design round.
