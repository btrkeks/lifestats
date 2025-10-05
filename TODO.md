# ✅ COMPLETED: Fix Time Frame Logging and Improve Error Robustness

## 🎯 High-Level Goal

A bug was discovered where logging time frames via the generic `/log/:date` endpoint fails with a misleading `500 Internal Server Error`. The root cause is a loss of type safety when passing a formatted string to domain logic that expects a structured object for certain update policies.

**Status:** ✅ COMPLETED

We fixed this by:

1.  ✅ Creating a dedicated, type-safe API endpoint for logging time frames.
2.  ✅ Improving our error handling to correctly classify client errors (`400`) vs. server errors (`500`).

---

## ✅ Implementation Summary

### Part 1: Type-Safe `log_time_frame` Endpoint ✅

- **1. Define the DTO (`lifestats-web-api`).**
  - In `core/infrastructure/web_api/src/dto/statistic.rs`, create a new request DTO:
    ```rust
    #[derive(Debug, Deserialize, Validate)]
    pub struct LogTimeFrameRequest {
        #[validate(regex(path = "::time_regex()", message = "must be in HH:MM or HH:MM:SS format"))]
        pub start_time: String,
        #[validate(regex(path = "::time_regex()", message = "must be in HH:MM or HH:MM:SS format"))]
        pub end_time: String,
        pub update_policy: Option<String>,
    }
    // Add a helper for the regex to keep it clean
    fn time_regex() -> &'static std::sync::OnceLock<regex::Regex> { ... }
    ```

- **2. Define the Application Service Command (`lifestats-application`).**
  - In `core/application/src/use_cases/logging_service.rs`, create a new command struct:

    ```rust
    use chrono::NaiveTime;
    use lifestats_domain::statistic::types::logable_statistics::time_spans::TimeFrameListUpdatePolicy; // Or similar path

    pub struct LogTimeFrameCommand {
        pub statistic_id: StatisticId,
        pub date: NaiveDate,
        pub start_time: NaiveTime,
        pub end_time: NaiveTime,
        pub update_policy: Option<TimeFrameListUpdatePolicy>,
    }
    ```

- **3. Implement the Application Service Method (`lifestats-application`).**
  - In `LoggingService`, add a new public method: `pub fn log_time_frame(&self, command: LogTimeFrameCommand) -> Result<()>`.
  - This method's logic will:
    - Fetch the `AnyStatistic` from the repository.
    - Verify it is an `AnyStatistic::TimeSpans`. If not, return an error.
    - Create a `TimeFrame` from `command.start_time` and `command.end_time`.
    - Call the type-safe `log_value` method on the inner `TimeSpansStatistic`, passing the `TimeFrame` and `update_policy`.
    - Publish the `ValueUpdated` event.

- **4. Create the API Route and Handler (`lifestats-web-api`).**
  - In `routes/mod.rs` and `routes/statistics.rs`, add a new route: `POST /statistics/:id/log_time_frame/:date`.
  - The new handler function will:
    - Accept the `LogTimeFrameRequest` DTO and validate it.
    - Parse `start_time` and `end_time` strings into `NaiveTime`. Return a `400 BadRequest` on parsing failure.
    - Parse the `update_policy` string into the domain's `TimeFrameListUpdatePolicy` enum.
    - Create the `LogTimeFrameCommand` and call `logging_service.log_time_frame()`.
    - Return a `204 No Content` on success.

- **5. Update Python Client (`AnkiLogger`).**
  - In `api_client.py`, create a new function `log_time_frame_async(...)`.
  - This function will POST to the new `/log_time_frame/` endpoint with the structured JSON payload: `{"start_time": ..., "end_time": ..., "update_policy": ...}`.
  - In `statistic_logging.py`, update `log_time_until_now` to call the new `log_time_frame_async` function.

### Part 2: Improved Systemic Error Handling ✅

- **1. Create a `DomainError` Enum (`lifestats-domain`).**
  - Create a new public error enum in the domain crate. This makes business rule violations explicit.
    ```rust
    #[derive(Debug, thiserror::Error)]
    pub enum DomainError {
        #[error("Validation failed: {0}")]
        ValidationError(String),
        #[error("Could not parse value: {0}")]
        ParsingError(String),
        // ... other specific domain errors
    }
    ```

- **2. Refactor Domain Logic to Return `DomainError` (`lifestats-domain`).**
  - Change the signature of methods like `calculate_new_log_value_from_primitives` to return `Result<..., DomainError>`.
  - Instead of returning a generic `anyhow::anyhow!("Could not parse...")`, return `Err(DomainError::ParsingError(...))`.

- **3. Map `DomainError` to `ApiError` (`lifestats-web-api`).**
  - In `web_api/src/error.rs`, implement `From<DomainError>` for `ApiError`.
    ```rust
    impl From<lifestats_domain::DomainError> for ApiError {
        fn from(err: lifestats_domain::DomainError) -> Self {
            match err {
                // This is the key change: map domain validation/parsing errors to a 400.
                lifestats_domain::DomainError::ValidationError(msg) |
                lifestats_domain::DomainError::ParsingError(msg) => ApiError::BadRequest(msg),
            }
        }
    }
    ```
  - This will require adding `lifestats-domain` as a dependency to the web API crate and enabling a feature flag if needed. The application service will need to bubble up the `DomainError`. This ensures that any time the domain rejects a value, the API correctly responds with `400 Bad Request`.
