# TODO – Statistic Score (Mean Daily Goal Achievement)

Goal: Add an application-layer function/use-case that computes a “score” for a statistic
over an arbitrary date range, defined as the **mean of `percentage_achieved` per day**.
This should reuse existing goal/achievement logic and stay aligned with Ports & Adapters.

---

## 1. Clarify semantics (design notes)

- **Score definition**
  - For each calendar day in the given `DateRange`:
    - If the statistic has a goal (daily or default, valid on that date):
      - Take the `percentage_achieved` returned by the goal logic for that day
        (including handling of “no log value” via `Goal::get_percentage_achieved(None)`).
      - Include this percentage in the average.
    - If the statistic has **no goal** on that day:
      - Exclude the day entirely from the average (do not treat it as 0%).
  - The final “score” is the **arithmetic mean** of these daily percentages.
    - If there are **no days with a goal** in the range, the score is `None`.

- **Supported statistics**
  - Score is only defined for statistics that support goals (e.g. Habit, IntegerAmount,
    FinishedInTime, TimeSpans, SecondsAmount).
  - For derived statistics where goals are not meaningful, the daily evaluator should
    produce `percentage_achieved = None` and the score will be `None`.

Keep this behavior documented in code comments and tests so future changes don’t
accidentally shift semantics.

---

## 2. Application Services: Extend `DailyAchievementEvaluator`

**Objective:** Reuse the existing bulk evaluation pipeline and enrich it with
per-day percentages so multiple use cases (streak, consistency, score) can share logic.

### 2.1 Extend `DayEval` to carry percentages

- [ ] In `core/application/src/services/daily_achievement_evaluator.rs`, extend
      the `DayEval` struct to include the daily percentage:

  ```rust
  use lifestats_core::shared::percentage::Percentage;

  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct DayEval {
      pub date: NaiveDate,
      /// Whether this day had any applicable goal (default or daily)
      pub has_goal: bool,
      /// Whether the goal was achieved (typically percentage >= 100)
      pub achieved: bool,
      /// Computed goal achievement percentage for this day.
      /// - `Some(p)` when there is a goal (and we can evaluate it)
      /// - `None` when there is no goal or the statistic type does not support goals
      pub percentage_achieved: Option<Percentage>,
  }
  ```

* [ ] Update all sites that construct `DayEval` (especially tests) to provide
      `percentage_achieved`. For the existing use cases (streak, consistency):
  - When `has_goal == false`, use `percentage_achieved: None`.
  - When `has_goal == true`, you can set a sensible percentage for the scenario
    (e.g. `Some(Percentage::from(true))` for “achieved”, `Some(Percentage::from(false))`
    for “not achieved”) unless the test explicitly cares about concrete values.

### 2.2 Compute `percentage_achieved` in the default implementation

- [ ] In `DefaultDailyAchievementEvaluator`, inside the main per-day evaluation loop:
  - Where you currently decide if a day is “achieved” or not, also compute and store
    the underlying percentage:

    ```rust
    let percentage = goal_obj.get_percentage_achieved(log_value_opt);
    let achieved = percentage.as_num() >= 100;
    ```

  - For days **with a goal**, set:

    ```rust
    DayEval {
        date,
        has_goal: true,
        achieved,
        percentage_achieved: Some(percentage),
    }
    ```

  - For days **without any goal**, set:

    ```rust
    DayEval {
        date,
        has_goal: false,
        achieved: false,
        percentage_achieved: None,
    }
    ```

- [ ] Ensure all statistic types that currently participate in goal evaluation
      map to a `Percentage` via `Goal::get_percentage_achieved` and that derived
      or unsupported types leave `percentage_achieved` as `None`.

- [ ] Add a couple of focused unit tests at the `DefaultDailyAchievementEvaluator`
      level to assert:
  - A day with a goal and a log value yields a `Some(Percentage)` with the expected value.
  - A day with a goal and **no** log value still yields a `Some(Percentage)` that matches
    the goal’s semantics for “not logged”.
  - A day with **no goal** yields `percentage_achieved == None`.

This ensures the new field is correctly wired before we depend on it elsewhere.

---

## 3. Application Services: Pure score calculator

**Objective:** Keep the aggregation logic (mean of daily percentages) as a small,
testable function that can be reused.

- [ ] Add a new module `core/application/src/services/statistic_score_calculator.rs`
      with a small, pure function:

  ```rust
  use crate::services::daily_achievement_evaluator::DayEval;
  use lifestats_core::shared::percentage::Percentage;

  /// Computes the mean percentage achieved over all days with a goal.
  ///
  /// - Only `DayEval` entries with `percentage_achieved = Some(_)` are counted.
  /// - Returns `None` if there are no such days (no goals in range).
  pub fn compute_mean_percentage(evals: &[DayEval]) -> Option<Percentage> {
      let mut total: u32 = 0;
      let mut count: u32 = 0;

      for eval in evals {
          if let Some(p) = eval.percentage_achieved {
              total += p.as_num() as u32;
              count += 1;
          }
      }

      if count == 0 {
          return None;
      }

      // Integer mean with simple half-up rounding.
      let mean = (total + count / 2) / count;
      Some(Percentage::new(mean as u8))
  }
  ```

- [ ] Write unit tests for `compute_mean_percentage`:
  - Empty slice → `None`.
  - All `percentage_achieved == None` → `None`.
  - Single entry → returns that percentage.
  - Multiple entries with varying percentages to verify averaging and rounding behavior.

- [ ] Re-export `compute_mean_percentage` from `crate::services` if that’s the existing
      pattern for other helpers.

---

## 4. Use Case: `GetStatisticScoreQuery`

**Objective:** Express “calculate score for statistic in date range” as a proper
application-level query that depends on ports/services, not on infrastructure.

### 4.1 Define query + result types

- [ ] Create new module: `core/application/src/use_cases/queries/get_statistic_score/query.rs`:

  ```rust
  use crate::cqrs::Query;
  use chrono::NaiveDate;
  use lifestats_core::shared::date_range::DateRange;
  use lifestats_core::shared::ids::StatisticId;
  use lifestats_core::shared::percentage::Percentage;

  /// Query to compute the mean daily goal achievement percentage for a statistic
  /// over a given date range.
  pub struct GetStatisticScoreQuery {
      pub statistic_id: StatisticId,
      pub date_range: DateRange,
  }

  /// Result of a statistic score query.
  ///
  /// - `mean_percentage_achieved` is `None` if there were no days with a goal in the range.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct StatisticScoreResult {
      pub statistic_id: StatisticId,
      pub start_date: NaiveDate,
      pub end_date: NaiveDate,
      pub mean_percentage_achieved: Option<Percentage>,
      /// Number of days in the range that had a goal and contributed to the score
      pub days_with_goal: u32,
  }

  impl Query for GetStatisticScoreQuery {
      type Result = StatisticScoreResult;
  }
  ```

### 4.2 Implement the query handler

- [ ] Add handler module: `core/application/src/use_cases/queries/get_statistic_score/handler.rs`:

  ```rust
  use std::sync::Arc;

  use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

  use crate::cqrs::QueryHandler;
  use crate::error::ApplicationResult;
  use crate::services::{compute_mean_percentage, DailyAchievementEvaluator};

  use super::query::{GetStatisticScoreQuery, StatisticScoreResult};

  /// Handler for GetStatisticScoreQuery.
  ///
  /// Responsibilities:
  /// - Delegate per-day evaluation to DailyAchievementEvaluator
  /// - Aggregate daily percentages via compute_mean_percentage
  /// - Return a simple, application-level result struct
  pub struct GetStatisticScoreQueryHandler<R, E>
  where
      R: StatisticRepository + Send + Sync + 'static,
      E: DailyAchievementEvaluator + Send + Sync + 'static,
  {
      repository: Arc<R>,
      evaluator: Arc<E>,
  }

  impl<R, E> GetStatisticScoreQueryHandler<R, E>
  where
      R: StatisticRepository + Send + Sync + 'static,
      E: DailyAchievementEvaluator + Send + Sync + 'static,
  {
      pub fn new(repository: Arc<R>, evaluator: Arc<E>) -> Self {
          Self { repository, evaluator }
      }
  }

  impl<R, E> QueryHandler<GetStatisticScoreQuery> for GetStatisticScoreQueryHandler<R, E>
  where
      R: StatisticRepository + Send + Sync + 'static,
      E: DailyAchievementEvaluator + Send + Sync + 'static,
  {
      fn handle(&self, query: GetStatisticScoreQuery) -> ApplicationResult<StatisticScoreResult> {
          // Ensure statistic exists; lets us fail fast for invalid IDs
          let _statistic = self
              .repository
              .find_metadata_by_id(query.statistic_id)?;

          // Bulk-evaluate goal achievement for the full date range
          let evals = self.evaluator.eval_range(
              self.repository.as_ref(),
              query.statistic_id,
              query.date_range,
          )?;

          // Aggregate percentages
          let mean_percentage = compute_mean_percentage(&evals);

          let days_with_goal = evals
              .iter()
              .filter(|e| e.percentage_achieved.is_some())
              .count() as u32;

          Ok(StatisticScoreResult {
              statistic_id: query.statistic_id,
              start_date: query.date_range.start(),
              end_date: query.date_range.end(),
              mean_percentage_achieved: mean_percentage,
              days_with_goal,
          })
      }
  }
  ```

  > Note: `eval_range` signature above should match the existing `DailyAchievementEvaluator`
  > trait. Adjust parameter ordering as needed to match the actual trait definition.

### 4.3 Wire the use case into the public query module

- [ ] In `core/application/src/use_cases/queries/get_statistic_score/mod.rs`:

  ```rust
  mod handler;
  mod query;

  pub use handler::GetStatisticScoreQueryHandler;
  pub use query::{GetStatisticScoreQuery, StatisticScoreResult};
  ```

- [ ] Register the new module in `core/application/src/use_cases/queries/mod.rs`:

  ```rust
  pub mod get_statistic_score;

  pub use get_statistic_score::{
      GetStatisticScoreQuery,
      GetStatisticScoreQueryHandler,
      StatisticScoreResult,
  };
  ```

---

## 5. Adapters: Web API Endpoint (optional but recommended)

**Objective:** Expose the statistic score via HTTP without leaking application internals.

### 5.1 Define an API DTO

- [ ] In `core/infrastructure/web_api/src/dto` (or the existing statistics DTO module),
      add a response type for the score, using primitives suitable for JSON:

  ```rust
  use serde::Serialize;

  #[derive(Debug, Serialize)]
  pub struct StatisticScoreResponse {
      pub statistic_id: u32,
      pub start_date: String, // "YYYY-MM-DD"
      pub end_date: String,   // "YYYY-MM-DD"
      /// Mean percentage as an integer 0-255 (or 0-100 depending on your conventions),
      /// or null when no goals exist in the range.
      pub mean_percentage_achieved: Option<u8>,
      pub days_with_goal: u32,
  }
  ```

### 5.2 Implement the route handler

- [ ] In `core/infrastructure/web_api/src/routes/statistics.rs` (or equivalent file
      where other statistic routes live), add:
  - A GET route like:

    ```text
    GET /statistics/:id/score?start=YYYY-MM-DD&end=YYYY-MM-DD
    ```

  - Handler responsibilities:
    - Parse `id` into `StatisticId`.
    - Parse `start` / `end` query params into `NaiveDate`, then into a `DateRange`.
    - Build and execute `GetStatisticScoreQuery` via the application layer.
    - Map `StatisticScoreResult` → `StatisticScoreResponse`.
      - Convert `StatisticId` to `u32`.
      - Convert `NaiveDate` to `String`.
      - Convert `Option<Percentage>` to `Option<u8>` using `as_num()`.

  - Return `400 Bad Request` for invalid dates or invalid ranges, and `500 Internal Error`
    for unexpected application errors (following the existing `ApiError` pattern).

### 5.3 Add route wiring tests

- [ ] Add an integration-ish test using the existing `mock_app_state` helper (if available)
      to verify:
  - Valid request returns 200 with correctly shaped JSON.
  - A range with no goals returns `mean_percentage_achieved: null` and `days_with_goal: 0`.
  - Invalid date format is rejected with a helpful error message.

---

## 6. Tests & Quality Gates

### 6.1 Update existing tests

- [ ] Update tests for `GetConsistencyScoreQueryHandler` and `GetStreakQueryHandler`
      that construct `DayEval` so they compile with the new `percentage_achieved` field.
  - For tests that do not care about the percentage, set it to a simple value:
    `Some(Percentage::from(eval.achieved))` or `None` depending on the semantics.

### 6.2 New tests for the score feature

- [ ] Add unit tests for `compute_mean_percentage` (see section 3).
- [ ] Add unit tests for `GetStatisticScoreQueryHandler`:
  - Use `MockStatisticRepository` and a test double for `DailyAchievementEvaluator`.
  - Scenarios:
    - No goals in range → `mean_percentage_achieved: None`, `days_with_goal: 0`.
    - Mixed percentages across days (e.g. 50%, 100%, 150%) → verify mean and rounding.
    - Non-existent statistic ID → handler propagates repository error.

### 6.3 Optional: High-level regression tests

- [ ] Once the HTTP endpoint exists, add a higher-level test (possibly in the
      web API crate’s test suite) that:
  - Sets up a small in-memory/stateless scenario with:
    - One statistic
    - A couple of goals and log values

  - Calls the HTTP endpoint
  - Asserts on the numeric score and day counts.

---

## 7. Documentation / Developer Notes (optional)

- [ ] Add a short section to the project’s architecture or README docs describing:
  - What “statistic score” means (mean daily percentage over goal days).
  - That it is an **application-level metric**, not a domain invariant.
  - How it relates to other metrics:
    - **Consistency score** = percentage of days achieved.
    - **Statistic score** = average quality of achievement per day (can capture
      over-achievement via percentages > 100).

This will help future contributors understand why the feature lives in the application
layer and why it reuses `DailyAchievementEvaluator` instead of re-querying the database.
