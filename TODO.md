# TODO: Implement Goal Performance & Trend Analysis

This feature introduces a new service to analyze goal achievement over a date range, providing an overall score and a trend indicator based on an Exponentially Weighted Moving Average (EWMA).

The implementation is broken down by architectural layer, starting from the domain core and moving outwards to the application and web API adapters. Reuse existing `DomainResult`, `ApplicationResult`, and `ApplicationError` types where appropriate; do not introduce new ones.

## 1. Domain Model (`lifestats-core`)

The first step is to create a new domain service that encapsulates the core business logic for calculating a time-series of daily goal achievements. This service will be a reusable asset for future features.

- [ ] **1.1. Create a new `GoalAchievementSeriesService` domain service.**
  - **File:** Create `core/core/src/statistic/services/mod.rs` and `core/core/src/statistic/services/goal_achievement_series.rs`.
  - **Struct:** Define `pub struct GoalAchievementSeriesService`. It should be stateless and take dependencies via its methods.
  - **Method:** Implement `pub fn calculate_series(&self, repo: &dyn StatisticRepository, statistic_id: StatisticId, date_range: DateRange) -> DomainResult<Vec<DatedValue<Percentage>>>`. The application layer will be responsible for providing the `repo` reference.
  - **Responsibility:** This method's sole responsibility is to orchestrate data fetching and apply business rules to generate a daily score for every day in the `date_range`. The logic must correctly handle days with daily goals, default goals, suppressed goals, and missing log values.
  - **Type:** Use the existing `DatedValue<T>` type from `lifestats_core::shared::dated_value`.

- [ ] **1.2. Add a helper method to `AnyStatistic` for calculating achievement.**
  - **File:** `core/core/src/statistic/any_statistic.rs`.
  - **Method:** Add `pub fn get_achievement_percentage(&self, log_value: Option<&dyn Any>, goal: Option<&dyn Any>) -> DomainResult<Percentage>`.
  - **Logic:** Reuse the same `Any`-based approach as `evaluate_streak_for_day` for consistency. This involves downcasting the `log_value` and `goal` to their concrete types based on the `AnyStatistic` variant and then calling the `get_percentage_achieved` method on the goal.

- [ ] **1.3. Add Unit Tests for the Domain Service.**
  - Create unit tests for `GoalAchievementSeriesService` using a mock `StatisticRepository`.
  - Test scenarios including:
    - A simple series with logs and goals present.
    - Days with no log value (should result in 0% or 100% based on goal type).
    - Days with a daily goal overriding the default goal.
    - Days with a suppressed goal (should result in 100% achievement).
    - An empty date range (should return an empty `Vec`).

- [ ] **1.4. Wire up the new domain modules.**
  - Update `core/core/src/statistic/mod.rs` to include `pub mod services;`.
  - Update `core/core/src/statistic/services/mod.rs` to export the new service: `pub mod goal_achievement_series;`.

## 2. Application Layer (`lifestats-application`)

With the domain service in place, we will build the application-level components that consume its output and calculate the final trend.

- [ ] **2.1. Define the public Data Transfer Objects (DTOs).**
  - **File:** Create `core/application/src/dto/goal_performance.rs`.
  - **Struct:** Define `pub struct GoalPerformanceDto` containing `overall_score: Percentage` and `trend: TrendDto`.
  - **Enum:** Define `pub enum TrendDto` with variants: `Improving { strength: f64 }`, `Declining { strength: f64 }`, `Stable`, and `InsufficientData`.
  - **Annotations:** Add `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` and `#[serde(tag = "direction", rename_all = "snake_case")]` to ensure a stable JSON contract.

- [ ] **2.2. Implement the hardened `Ewma` utility.**
  - **File:** Create `core/application/src/services/ewma_calculator.rs`.
  - **API:** Implement a safe constructor `pub fn from_span(span: u32, initial_value: f64) -> Self`. The constructor should `assert!(span > 0)`.
  - **Logic:** Implement `pub fn add(&mut self, value: f64)`.
  - **Tests:** Add unit tests to verify the calculation against a known sequence and test edge cases for `span`.

- [ ] **2.3. Implement the `GoalPerformanceAnalyzer` service.**
  - **File:** Create `core/application/src/services/goal_performance_analyzer.rs`.
  - **Composition:** The `GoalPerformanceAnalyzer` should be constructed in the application's composition root (e.g., where `AppState` is built). It should accept an `Arc<dyn StatisticRepository>` in its constructor.
  - **Constants:** Define constants for EWMA parameters (`FAST_EWMA_SPAN`, `SLOW_EWMA_SPAN`), trend calculation (`MIN_DATA_POINTS_FOR_TREND`), and stability (`TREND_EPSILON: f64 = 1.0`).
  - **Method:** Implement `pub fn analyze(&self, statistic_id: StatisticId, date_range: DateRange) -> ApplicationResult<GoalPerformanceDto>`.
    - Instantiate the domain service: `let series_service = GoalAchievementSeriesService::new();`.
    - Call `series_service.calculate_series(&*self.repo, ...)` to get the daily scores.
    - **Error Handling:** If the returned series is empty, return `Err(ApplicationError::InsufficientData("No data available in the selected date range".to_string()))`.
    - If the series has fewer points than `MIN_DATA_POINTS_FOR_TREND`, return a DTO with `TrendDto::InsufficientData`.
    - Calculate `overall_score` by averaging the percentages in the series.
    - Calculate `trend` using the `Ewma` utility. Normalize the `strength` using the formula: `((fast_val - slow_val).abs() / 100.0).clamp(0.0, 1.0)`.
  - **Tests:** Add unit tests, mocking the `StatisticRepository`, to verify the trend logic for improving, declining, stable, and insufficient data scenarios.

- [ ] **2.4. Wire up the CQRS `GetGoalPerformanceQuery`.**
  - **Directory:** Create `core/application/src/use_cases/queries/get_goal_performance/`.
  - **Query:** Define `GetGoalPerformanceQuery` in `query.rs`.
  - **Handler:** In `handler.rs`, define `GetGoalPerformanceQueryHandler`.
    - Its constructor should accept an `Arc<GoalPerformanceAnalyzer>` for clean dependency injection.
    - Implement an `async fn handle(...)` method that aligns with your `QueryBus` trait, calling `analyzer.analyze(...)`.

- [ ] **2.5. Wire up the new application modules.**
  - Update `core/application/src/dto/mod.rs`, `services/mod.rs`, and `use_cases/queries/mod.rs` to export the new modules.

## 3. Adapters (`web-api`)

Finally, expose the new application query via a public HTTP endpoint.

- [ ] **3.1. Add the new HTTP route handler.**
  - **File:** `core/infrastructure/web_api/src/routes/statistics.rs`.
  - **Handler:** Create an `async fn get_goal_performance` handler.
  - **Route:** `GET /statistics/{id}/performance`
  - **Inputs:** Use `axum::extract::{Path, Query, State}` to get the `statistic_id`, `start_date`, and `end_date`.
  - **Logic:**
    1.  Parse inputs and create `StatisticId` and `DateRange` value objects.
    2.  Create a `GetGoalPerformanceQuery`.
    3.  Dispatch the query using `state.query_bus.dispatch(query).await?`.
    4.  Return the resulting `GoalPerformanceDto` as `Json`.
    5.  Handle errors, mapping `ApplicationError::NotFound` to HTTP 404 and `ApplicationError::InsufficientData` to HTTP 400.

- [ ] **3.2. Register the new route.**
  - **File:** `core/infrastructure/web_api/src/lib.rs` (or wherever your Axum router is defined).
  - Add the new `/statistics/:id/performance` route to your router, connecting it to the `get_goal_performance` handler.

- [ ] **3.3. Add an End-to-End Test.**
  - In the `web-api` tests, create a new test that uses a real in-memory database.
  - Set up a statistic with some log and goal data.
  - Call the new endpoint using a test client.
  - Assert that the response status is 200 OK and that the JSON body matches the expected `GoalPerformanceDto` shape.
  - Add test cases for a non-existent statistic ID (assert 404) and a date range with no data (assert 400).
