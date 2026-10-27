# Design sketch: `TimedAmounts` statistic and OpenNutriTracker day sync

Status: **approved 2026-10-27; implemented on `feature/timed-amounts-statistic`.** The sketch below is the contract the implementation follows. Files:

- `README.md`: this rationale.
- [`contract.md`](contract.md): the exact HTTP contract OpenNutriTracker calls.
- [`touchpoints.md`](touchpoints.md): every file the implementation edits, marked compile-checked or silent-if-missed.
- `sketch/`: code that mirrors repo paths, with `todo!()` bodies. New files are complete. Existing files show only the changed variants, arms and signatures.

## Problem

OpenNutriTracker will record each caffeine or nicotine intake with its own time. Lifestats has no type that keeps several timed values per day:

- `PointInTime` keeps one time.
- An `IntegerAmount` total plus a `PointInTime` "last dose" would be two statistics for one fact.

The shape is non-obvious for these reasons:

- `Statistic::GoalType: Goal<LogValue = Self::LogValueType>` (`core/core/src/statistic/traits/base_statistic.rs:9`). A list-valued type therefore cannot use `AmountGoal<i64>` directly.
- The only list precedent, `TimeFrameList`, lets serde skip its validating constructors (`core/core/src/shared/time_frame_list.rs:19`, `:123`). Its default `Append` policy is `todo!()` (`update_policies/time_frame_list.rs:21`).
- "Replace the day" means something different in each integration:
  - LiftLog: non-atomic, and an empty list deletes.
  - Beurer: a transactional store that overwrites the metrics supplied.
  - Yomikata: a ledger that keeps manual residuals.
  - Ebook: merges reports and orders them by `observed_at`.
- The list of statistic types is repeated in about ten places. About a third of them fail silently when one is missed. The worst is the persistence decoder (`mappers.rs:57`): if it is missed, rows still get written, and then every `GET /statistics` fails.
- OpenNutriTracker has no caffeine field yet. Its amounts are doubles, and date-only entries are stored at midnight as "date labels" with no known time (`day_boundary_calc.dart:142`).

## Usage (caller's view)

The app sends each day it touched as a complete snapshot of both metrics (details in [`contract.md`](contract.md)):

```http
PUT /integrations/opennutritracker/days/2026-10-27
Content-Type: application/json

{"caffeine":[{"time":"08:12:00","amount_mg":120},{"time":"13:40:00","amount_mg":60}],"nicotine":[]}
→ 204 No Content
```

You set the 400 mg cap with the existing goal endpoint and the IntegerAmount goal shape:

```http
PUT /statistics/42/goals/default
{"goal_data":{"target":"400","is_at_least":"false"},"valid_since":"2026-10-27"}
```

Server call sites. Tracing a write takes three files:

```rust
// web_api/src/routes/opennutritracker.rs: strict raw parse, then domain types
let values: OpenNutriTrackerDay = OpenNutriTrackerDayRequest::parse(&body)?.into_day()?;
let outcome = state.integration_service.replace_opennutritracker_day(day, values)?;
// publish ValueUpdated for outcome.changed_statistics, return 204

// application/.../integration/service.rs
store.replace_day(day, Local::now().date_naive(), values)

// persistence/src/opennutritracker_day_store.rs, one IMMEDIATE transaction
for metric in OpenNutriTrackerMetric::all() {
    let id = resolve_metric(conn, metric, today)?;          // by (source, metric.tag()); create via metric.new_statistic(today)
    if replace_list(conn, id, day, values.list(metric))? { changed.push(id) }
}
```

Read side. Every reader takes its daily number from the same method:

```rust
let list: TimedAmountList = serde_json::from_value(log_json)?; // validated on deserialize
DayValue::Amount(list.total())                                  // 180
TimedAmountsGoal::new(400, false)?.get_percentage_achieved(Some(&list))
```

## Shape

**Data first.** `TimedAmount { time: Option<NaiveTime>, amount: i64 }` and `TimedAmountList(Vec<TimedAmount>)` have private fields and are immutable (`sketch/core/core/src/shared/timed_amount_list.rs`):

- **Validation.** Construction, `Deserialize` and `FromStr` all apply the same checks: known times have whole seconds, amounts are from 1 up to the cap, at most 1000 entries, and the checked sum stays within the cap. Database JSON therefore can't bring in values the HTTP parser would reject (per boundary-discipline).
- **Canonical order.** Lists sort by (time, amount), unknown times first. Equality and change detection don't depend on the order the app sends.
- **Duplicates.** Duplicate entries are kept; two pieces of gum at 10:00 are two intakes.
- **One total.** `total()` is the only definition of the daily total. `IntegerStatistic`, the goal, the evaluator, the dashboard and the plots all call it, which avoids the three disagreeing TimeSpans totals.
- **Overflow.** The cap is JavaScript's largest safe integer, and `cap * 100` still fits in `i64`. The infallible `value_as_i64` and `AmountGoal`'s `* 100` therefore can't overflow.

**Write semantics.**

- `Replace` is the only update policy, and it is the default. The type can never inherit `Ignore` or `Append`.
- The only writer is `OpenNutriTrackerDayStore::replace_day(day, today, OpenNutriTrackerDay)`, a domain port like `BeurerDayStore`. It hides the transaction, identity resolution, conflict rejection, provisioning, typed canonical comparison and both writes.
- `OpenNutriTrackerDay` has named `caffeine` and `nicotine` fields. Both are required, so sending only caffeine can't leave a stale nicotine list in place.
- `[]` is stored as confirmed zero. A corrupt stored row is a storage error, never treated as "absent".
- The generic routes are closed, so this is the one way to write (per the "two ways to do one task" red flag):
  - the generic log route returns 400;
  - `POST /statistics` returns 400;
  - there is no `LogValuePayload::TimedAmounts`;
  - `/statistics/loggable` derives from an exhaustive `is_manually_loggable`.

**Identity has one source.** `OpenNutriTrackerMetric` (`Caffeine | Nicotine`) owns the `metric` tag, the initial name and `new_statistic(today)`. The store's lookup reads the same `tag()`. A statistic the store creates is therefore always the one it finds next time. Beurer, in contrast, writes tags in one file and looks them up in another. No application-level definitions module and no `BundleFactory` are needed.

**Goal.** `TimedAmountsGoal` wraps `AmountGoal<i64>` and stores the same JSON. Its target is validated on construction and on deserialization, in the range 0..cap. Effects:

- It reuses `GoalPayload::IntegerAmount`, the HTTP `goal_data` and the UI `AmountFields`.
- One arm in `parse_goal_payload_to_json` enables daily, default and range goals together.
- Behaviour that differs from IntegerAmount on purpose: a never-sent day scores 0%, while `[]` scores as a total of 0. A sync that didn't run can then never count as meeting the cap.

**The "nothing after 14:00" goal is deferred.** It needs the entry that was taken last, which is exactly what the deferred midnight boundary gets wrong (00:30 sorts first). Unknown times make it worse. A composite goal would also squeeze two independently failing limits into one percentage and one plot line. When it comes, it is a separate goal design on the same stored list, and no data migration is needed.

**Reads stay scalar.**

- Achievements, the dashboard and bar/line/scatter plots see the daily total.
- `time_frames` and `circular_progress` plots return 400, even for ranges with no data.
- The raw `GET /statistics/{id}/logged/{day}` returns every entry unchanged.
- There are no dose markers, no timeline entries and no allocation share.
- "mg" shows through the statistic name, as with Beurer's `(kg)`. The UI's name-unit regex gains `(mg)`.

**Fewer silent misses** in the lists this change touches anyway (per the "hand-synced list" red flag):

- The application `StatisticType::from_str` and the persistence `StatisticTypeDto` decoder are derived from `EnumIter` over the exhaustive `as_str`.
- The persistence encoder loses its `_ =>` arm.
- The loggable allowlist becomes exhaustive.
- The SDK's `StatisticType` is inferred from its Zod enum.
- The UI's `derived: boolean` becomes an exhaustive `origin`.
- `GoalFields` gets a `never` guard.
- The dead `web_api/src/helpers.rs` is deleted; it has no callers (checked by grep).

**Interface depth.** The public surface is:

- one HTTP route;
- one port method;
- one metric enum;
- two value types with read-only accessors;
- one goal type.

Callers never see transactions, tags, canonical JSON or conflict rules.

## Synthesis decision

Two runners on the same brief: Claude Opus 5.5 (candidate B) and GPT-6.1 Sol (candidate A). A GPT-6.1 Sol cross-judge scored them blind against a six-criterion rubric: A 25/30, B 18/30. Its verdict is in `/tmp/architect-timed-amounts/judge/verdict.md`.

**Agreement on the core.** Both candidates reached the same overall structure on their own:

- a validated canonical list with a single `total()`;
- an adapter that wraps `AmountGoal`;
- a replace-only policy;
- a Beurer-style transactional source store;
- no migration and scalar reads;
- the cutoff goal deferred.

That independent agreement is the strongest signal in the round.

**Base: A.** It protects arithmetic and deserialization more completely. Its goal target is bounded, and B's at-most `target * 100` can overflow. Its typed reads turn a corrupt row into an error, whereas B treated a corrupt row as absent, so a clear request would return 204 and change nothing. A's plot-type rejection works even on ranges with no data and maps to 400. Its touchpoint list is more accurate. I checked each of these against the source myself before agreeing.

**Grafted from B:**

1. Enum-derived parsers in the application and persistence layers, the explicit encoder arm, the Zod-inferred SDK type and the UI `origin` field.
2. Source-only v1: generic create and log are rejected, and the type is not offered for creation. While grafting this I found that the create route maps every error to 500 (`routes/statistics.rs:430`). The judge flagged it, and the sketch fixes the mapping.
3. `time: null` for intakes with a known day but an unknown time.
4. Deleting the dead `helpers.rs`.

**The identity graft is my own merge**, prompted by the judge's critique of A. A single `OpenNutriTrackerMetric` enum now drives both creation and lookup. This removed A's application-level definitions file and the port's `definitions` parameter.

**Rejected:**

- **From A:** whole-list overwrite for manually created TimedAmounts (scope with no consumer, and a second ownership case), and the unused `last_time()`.
- **From B:** the source-agnostic `TimedAmountsDayStore` with `Vec<MetricDay>`. It is generality without a second source, and its public vector allowed duplicate metric keys. I also rejected B's delete-on-empty, its partial-metric replace (`null` = untouched) and its composite-goal sketch.

## Tradeoffs accepted

- **Last write wins.** We accept last-write-wins with no revision in exchange for a stateless contract. A delayed retry of an old snapshot can revert a newer edit until the next sync of that day. This follows the LiftLog precedent and assumes one device.
- **Missing days fail the goal.** We accept that a day the app never sent fails a caffeine cap, in exchange for "not synced" never passing as "zero". The client resends a fixed recent window (about 14 days) on every sync, so days without intake arrive as `[]`.
- **Wall-clock times.** We accept wall-clock `NaiveTime` with no logical-day offset in exchange for a value as simple as `TimeSpans`. Totals are correct. Only order-based features are affected, and they are deferred.
- **Unknown times.** We accept `Option<NaiveTime>` in exchange for never inventing midnight or blocking a day's sync. Future cutoff and decay features must decide how to treat unknown times.
- **No manual entry.** We accept that a TimedAmounts statistic can't be created or logged by hand in v1, in exchange for one writer and no shared-state merge.
- **Fixed metric fields.** We accept a source-specific route with fixed metric fields in exchange for an exact, typed contract documented in OpenAPI. A new metric needs a server release, and it must be an optional field so older clients keep working.
- **Hardening edits beyond the feature.** We accept a few extra edits (strum derives in two manifests, the UI `origin` field, deleting `helpers.rs`) in exchange for turning the worst silent misses into compile errors. Each can be its own commit.
- **No mg unit type.** We accept "mg" coming from the statistic name in exchange for not adding a `ScalarUnit` variant. Renaming the statistic away from `(mg)` drops the UI suffix.

## Alternatives considered

**A source event ledger with derived projections.** It would add a table of intake events with stable source entry IDs and `occurred_at`, with total and last-dose statistics computed from it:

```rust
struct IntakeEvent { source_entry_id: EntryId, occurred_at: DateTime<FixedOffset>, amount: Milligrams }
trait IntakeStore { fn replace_source_day(&self, source: SourceId, day: NaiveDate, events: Vec<IntakeEvent>) -> Result<()>; }
// plus IntakeTotalStatistic / LastIntakeStatistic projections
```

It would give per-entry append, multiple writers, and real timestamps that solve midnight. It lost because:

- it needs a migration and a second copy of each day, which is split ownership;
- it needs a computation engine the repo doesn't have;
- callers must learn entry IDs;
- it brings back two statistics for one fact.

None of those capabilities is needed now. The chosen list JSON is exactly the projection such a ledger would write later, so this design doesn't block it.

**A generic `PUT /statistics/{id}/days/{day}` with client-side provisioning.** It would be reusable by the web UI. It lost on interface depth and ownership:

- the client must provision, store and recover statistic IDs;
- caffeine and nicotine would be two calls, so not atomic;
- the generic path has no transaction or owner check.

**A composite goal now**, `{amount: AmountGoal, last_entry_before: Option<NaiveTime>}`. It lost because one percentage and one plot target would hide two separate failures, and its "last entry" is wrong across midnight.

## Decisions (Daniel, 2026-10-27)

1. **Unknown times:** accept `"time": null`. Such an entry counts toward the total and makes no timing claim.
2. **`[]` vs a missing day:** `[]` is confirmed zero; a never-sent day counts as a missed goal. The app client resends the full lists for a fixed recent window (about 14 days) on every sync, so days without intake arrive as `[]` and deletes propagate. `contract.md` records this as the expected client behaviour.
3. **Source-only v1:** yes. Adding single entries by hand stays deferred.
4. **"Nothing after 14:00":** deferred, together with midnight handling.
5. **Names:** `[OpenNutriTracker] Caffeine (mg)` / `[OpenNutriTracker] Nicotine (mg)`, category `OpenNutriTracker`.
6. **Old SDK builds:** none known. Before finishing, every in-repo consumer of `GET /statistics` must tolerate an unknown or new `statistic_type` without failing the whole listing.

## Verification plan (for implementation)

1. **Pure domain tests.**
   - Lists: permutations compare and serialize equal; duplicates count twice; `null` time sorts first and round-trips.
   - Every decoder rejects what `new` rejects: zero or negative amounts, fractions, leap seconds, more than 1000 entries, an overflowing sum, a missing `time` key.
   - Goal truth table: never sent → 0%; `[]` → 100% for at-most; 180 against an at-most 400 → 222%; an out-of-range target is rejected on deserialize.
   - `Replace`: a replay of the same list counts as unchanged.
2. **Real SQLite persistence.** New and old type discriminants coexist. Metadata round-trips with and without a goal, and goal history round-trips. Use a copy of the compat snapshot, because the existing tooling migrates the snapshot in place.
3. **Real router and SQLite for the PUT.**
   - First PUT creates both statistics. A reordered replay is a no-op with no event. An edit removes the old entry. `[]` persists with `has_value:true`. Other days are untouched.
   - Every error row in `contract.md` returns its documented error and commits nothing.
   - Duplicate, wrong-type and wrong-unit bindings return 409.
   - An injected failure on the second write rolls back the first write and the provisioning.
4. **Consistency.** The raw GET, achievements, dashboard and plot data all report the same total. Unsupported plot types return 400 on empty ranges. Generic create and log return 400. The type is absent from `/statistics/loggable`.
5. **SDK and UI.** Typecheck and build both. Load a statistics list that includes a TimedAmounts statistic. Save and reload a goal. Today's view shows "No data" for a never-sent day and "0 mg" for `[]`. TimedAmounts is not in the create dialog.
6. **OpenAPI and docs.** Regenerate `openapi.json` (`UPDATE_OPENAPI=1`) and check required plus nullable `time`, `max_items` and integer bounds. Update `core/infrastructure/web_api/README.md` and `.claude/skills/lifestats/SKILL.md` per `touchpoints.md`.

## Next implementation step

Write `core/core/src/shared/timed_amount_list.rs` and `timed_amounts_goal.rs` with their pure tests: validation through serde, canonical order, `total()` bounds, and the missing-vs-`[]` goal table. Every other layer depends on these two.
