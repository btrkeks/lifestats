# TODO: Log “Anki finished cards time” via AnkiLogger (Option 1)

## Feature Summary

When Anki shows the **“congratulations”** screen (no more due cards for today),
the AnkiLogger addon should log a **PointInTime** value into lifestats representing
“the time I finished my Anki cards for the day”.

- This will be a **PointInTime statistic** using the existing
  `PointInTimeStatistic` + `FinishedInTimeGoal` domain types.
- It should be loggable via the existing `LogValueCommand` `PointInTime` payload
  with a `FixedValueUpdatePolicy` of `Overwrite`.
- The user can then set a **FinishedInTimeGoal** (deadline) using the existing goal
  mechanisms (`SetDefaultGoal` / `SetDailyGoal`) and get streaks / plots for
  "finished in time".

Design choice for semantics (decided here, no extra code complexity):

- We treat “finished time” as **the final time that day when Anki showed
  “congratulations” for the collection as a whole** (not per deck).
- Each new “congratulations” in the same day **overwrites** the previous value.

This matches the intuition: if you reach zero at 18:00, then later do more cards
and only finish again at 23:30, the day counts as 23:30.

---

## Architectural Principles (applied here)

- **Hexagonal / Ports & Adapters**
  - Core domain already models PointInTime + FinishedInTime goals.
  - We only add a new **integration bundle definition** and **adapter behavior**
    (Anki addon + web API usage).
- **Low coupling**
  - Lifestats core does **not** know about Anki UI concepts like “congratulations”.
  - The Anki addon translates that into a concrete log_value call.
- **High cohesion**
  - All Anki-specific metric metadata stays in the Anki integration bundle.
  - All “when/how to log” lives in the addon.
- **Reusability**
  - FinishedInTime is reusable for other integrations (e.g., “done with workday”).

---

## 1. Domain Model (Rust, `core/core` crate)

No new domain types are required. The domain already supports what we need:

- `PointInTimeStatistic` (log-able NaiveTime) with `FixedValueUpdatePolicy`.
- Goals: `FinishedInTimeGoal` with `LogValue = NaiveTime`.
- Setting default goals for point-in-time stats via `AnyStatistic::set_point_in_time_default_goal`.

### Tasks

- [ ] **(Optional, tiny)** Add a short doc comment near `PointInTimeStatistic` making
      explicit that it is intended for things like “wake-up time”, “finished
      studying”, etc., which covers our use case. This is just for future
      discoverability.

No code changes beyond comments are strictly necessary in the domain.

---

## 2. Application Layer: Anki Integration Bundle (Rust, `core/application`)

Goal: Extend the **Anki integration bundle** with a new **global finished time**
statistic that the addon can discover via tags.

We will follow the existing Fitbit/Anki integration patterns and keep all
metadata/tagging decisions in one well-factored place.

### 2.1. Define a new metric key for finished time

File: `core/application/src/integrations/anki/anki_bundle_factory.rs` (same file
where `METRIC_BACKLOG`, `METRIC_TIME_SPENT`, etc. live). :contentReference[oaicite:7]{index=7}

- [ ] Add a new metric key constant:

  ```rust
  pub const METRIC_FINISHED_TIME: &str = "finished_time";
  ```

Keep naming consistent with `METRIC_BACKLOG`, `METRIC_TIME_SPENT`,
`METRIC_DISRUPTIONS`.

### 2.2. Create a PointInTime statistic for “Anki finished time”

Still in `AnkiBundleFactory`:

- [ ] Add a function or inline code block (mirroring the Fitbit wake/sleep
      stats) that constructs a new `PointInTimeStatistic`:

  ```rust
  use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
  use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
  use chrono::NaiveDate;

  fn build_finished_time_stat(
      id: StatisticId,
      today: NaiveDate,
  ) -> AnyStatistic {
      let metadata = StatisticMetadata::new(
          id,
          "Anki".to_string(),
          "[Anki] Finished Time".to_string(),
      )
      .with_tag("source".to_string(), "anki".to_string())
      .with_tag("metric".to_string(), METRIC_FINISHED_TIME.to_string());
      // IMPORTANT: No deck_name tag -> global per Anki profile
      // No default goal yet; user configures via goals API.

      let stat = PointInTimeStatistic::new(metadata, None, today);
      AnyStatistic::from_point_in_time(stat)
  }
  ```

  Notes / rationale:
  - We **omit `deck_name`** to clearly signal that this metric is global,
    per Anki profile, matching how the “congratulations” screen works.
  - Category `"Anki"` keeps compatibility with other Anki metrics; name
    `[Anki] Finished Time` is human-friendly.
  - We attach `source=anki` + `metric=finished_time` tags so that adapters
    (Anki addon / web API clients) can reliably discover this stat via tags,
    same pattern as Fitbit.
  - No default `FinishedInTimeGoal` is set here to avoid hardcoding a deadline
    in the integration.

### 2.3. Include the new stat in the Anki bundle

Where `AnkiBundleFactory` currently builds its list of `ProvisionedStatistic`s,
extend it with the new global stat.

- [ ] Generate an ID for the new statistic using the same mechanism used for
      existing Anki stats (likely `id_provider.next()` or similar).
- [ ] Call `build_finished_time_stat` with that ID and the provisioning date.
- [ ] Add a `ProvisionedStatistic` entry with:

  ```rust
  ProvisionedStatistic::new(
      METRIC_FINISHED_TIME,
      finished_time_statistic_any, // AnyStatistic from helper above
  )
  ```

  Follow the existing pattern; keep the creation logic cohesive, e.g.:
  - Group all Anki stats creation together.
  - Avoid inlining long builders; use small helpers like `build_finished_time_stat`
    for readability and testability.

### 2.4. Tests for Anki bundle metadata

Add or extend tests in the Anki integration test module:

- [ ] Add a test that provisions an Anki bundle and asserts:
  - There is **exactly one** statistic with `metric=finished_time` and `source=anki`.
  - It has no `deck_name` tag (global).
  - It is of type `PointInTime` when passed through the application’s
    `StatisticSummary` mapping.

  This helps enforce the contract that the addon will rely on.

---

## 3. Application Layer: Goals API (Rust, `core/application`)

We want the user to be able to set a “finish by HH:MM” goal for this stat using
existing mechanisms (`SetDefaultGoalCommand` / `SetDailyGoalCommand`) which
already support `FinishedInTimeGoal`.

### 3.1. Confirm support for FinishedInTime goals in commands

- [ ] Verify the `SetDefaultGoal` and `SetDailyGoal` command handlers already
      handle `AnyStatistic::PointInTime` by creating `FinishedInTimeGoal`.
      (They do, but add a short test that specifically covers the PointInTime path.)

- [ ] Add tests (if missing) to ensure:
  - Passing a `FinishedInTime` goal payload to a non-PointInTime stat fails
    with a clear error.
  - Passing a `FinishedInTime` payload to the finished_time statistic works
    and stores a `FinishedInTimeGoal`.

No behavior change, just coverage to protect the feature we’re about to rely on.

---

## 4. Web API Adapter (Rust, `web_api` crate)

Goal: Make it easy for the Anki addon to:

1. Find the `finished_time` statistic.
2. Log a PointInTime value for today with `Overwrite` semantics.
3. Optionally configure the deadline goal.

Assume we follow the existing pattern: the web API parses request JSON into
strongly-typed `LogValueCommand` / `SetDefaultGoalCommand` using DTOs.

### 4.1. Statistic discovery endpoint (if not already present)

If there is not yet a convenient endpoint for finding stats by tags:

- [ ] Introduce an endpoint (e.g. `GET /statistics/search`) that accepts `tags`
      as query parameters (`?source=anki&metric=finished_time`) and returns the
      single matching statistic (or an error if 0/ >1).
- [ ] Internally, this delegates to `StatisticRepository::find_by_tags`.
- [ ] Ensure it returns the application-layer `StatisticSummary` DTO, which
      already carries the `StatisticType` (`PointInTime`).

### 4.2. Log value endpoint for PointInTime

There should already be a generic “log value” endpoint; we just need to ensure
it plays nicely with PointInTime.

- [ ] Confirm/extend the request DTO so that, when the statistic type is
      `PointInTime`, the payload can be:

  ```json
  {
    "value": "HH:MM:SS",
    "update_policy": "overwrite" // optional, defaults to "ignore" otherwise
  }
  ```

- [ ] Adapter logic must parse:
  - `"value"` into `chrono::NaiveTime`.
  - `"update_policy"` into `FixedValueUpdatePolicy` (default to `Overwrite`
    for this specific addon’s usage; generic API can still default to `Ignore`).

- [ ] Ensure that the API calls the `LogValueCommandHandler` with a
      `LogValuePayload::PointInTime { value, update_policy }`.

### 4.3. Optional: goal configuration endpoint ergonomics

- [ ] If not already ergonomic, add a small helper endpoint for setting a
      `FinishedInTime` default goal by time-of-day, e.g.:

  ```json
  {
    "statistic_id": "<id>",
    "goal_type": "FinishedInTime",
    "deadline": "21:30:00"
  }
  ```

  which maps to `SetDefaultGoalCommand` with `GoalPayload::FinishedInTime`.

This lets the Anki addon (or you via a UI) set the preferred deadline once,
instead of hardcoding anything in the integration.

---

## 5. AnkiLogger Addon (Python / Anki side)

This is the **primary adapter** that implements the user behavior: “when I see
the congratulations screen, log my finished time”.

### 5.1. Discover the finished_time statistic ID

In the addon initialization / config sync logic:

- [ ] Call the lifestats API endpoint to search for the statistic with tags:
  - `source=anki`
  - `metric=finished_time`

- [ ] Cache the returned `statistic_id` in the addon’s config, alongside any
      already stored IDs for backlog/time_spent/disruptions (if such mapping
      already exists).
- [ ] Handle error cases gracefully:
  - If 0 stats found: show a clear error in addon config that Anki integration
    was not provisioned.
  - If >1 stats found: log a warning; the addon can either pick the first or
    refuse to operate (I’d recommend refuse, with a good error message).

### 5.2. Hook into the “congratulations” event

In the Anki addon code:

- [ ] Use the appropriate Anki hook / signal that fires when the user has
      completed all due cards for the day and Anki shows the congratulations
      screen (this already exists – reuse the same mechanism that triggers
      existing logging to lifestats if present).
- [ ] In that hook:
  - Get **current local date & time**.
  - Build a request to the lifestats web API’s `log_value` endpoint:
    - `statistic_id`: ID discovered above.
    - `date`: today’s date (as NaiveDate string).
    - `value`: current time (HH:MM:SS) as NaiveTime.
    - `update_policy`: `"overwrite"` (we want last finish time of the day).

- [ ] Fire the request asynchronously / in a background thread so it doesn’t
      block the Anki UI (respect existing patterns in the addon).

### 5.3. Prevent logging across days / timezones

To keep semantics clean:

- [ ] Ensure the date you send to lifestats is the **local calendar date**
      according to the user’s system, not UTC. This is consistent with other
      stats (time_spent, etc.).
- [ ] If a request is retried or delayed, it should still log under the date of
      the **actual finishing day** (which you can compute at the moment the
      event fires, not when the HTTP call eventually succeeds).

### 5.4. Optional: first-run goal convenience

Optionally, the addon can offer a quick wizard:

- [ ] If the finished_time statistic exists but has no default goal yet, ask the
      user (once) for a preferred “finish by” time (e.g., 21:00).
- [ ] Call the goal configuration endpoint to set a `FinishedInTimeGoal` as
      default for that statistic.

This keeps goal configuration user-driven but convenient, and still outside the
core domain.

---

## 6. Testing & Quality

### 6.1. Rust side (core/application + web_api)

- [ ] **Unit tests** in Anki integration:
  - Provision bundle and assert metadata & tags for the finished_time stat.

- [ ] **Integration tests** for web API:
  - Logging a PointInTime value:
    - Request with `"value": "20:15:00", "update_policy": "overwrite"`.
    - See that repository receives a `serde_json` NaiveTime and that
      `LogValueCommandHandler` persists it.

  - Setting a `FinishedInTime` goal:
    - Request with a `deadline` time.
    - Confirm a `FinishedInTimeGoal` serialized as JSON is persisted and used
      by `GetPlotData` when evaluating goal achievement.

### 6.2. Python / Anki side

- [ ] **Unit-ish tests** (where possible) for:
  - Function that constructs the HTTP payload and URL.
  - Function that chooses the correct statistic ID from a list of stats (by tags).

- [ ] **Manual QA checklist**:
  - Start the day with due cards, finish them:
    - Verify that the finished_time stat has a value for today.

  - Finish cards twice in a day:
    - Verify the **latter** time is stored (overwrite behavior).

  - Set a FinishedInTime goal (e.g. 21:00):
    - Verify that plot/streak queries show achievement when finishing earlier
      and miss when finishing later.

---

## 7. Migration / Rollout

- [ ] Provision the Anki integration bundle for your user so the new
      `finished_time` stat exists before shipping the updated addon.
- [ ] Add a short changelog entry and help text describing:
  - What “Finished Time” measures.
  - How to set a deadline goal.
  - That multiple finishes per day are collapsed to the last one.
