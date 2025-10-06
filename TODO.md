# `TODO.md` - Priority 3: Solidify the Error Handling Strategy

## Objective

The goal of this refactoring is to create a clean, layered error handling strategy. Our current `StatisticError` mixes concerns from different architectural layers (domain, repository, general). This makes it difficult for consumers like the Web API to translate errors into meaningful responses (e.g., `400 Bad Request` vs. `500 Internal Server Error`).

We will introduce distinct error types for each layer:

1.  **Repository Layer:** A new `RepositoryError` for persistence-related failures.
2.  **Domain Layer:** The existing `DomainError` will be the sole representation of business rule violations.
3.  **Application Layer:** A new `ApplicationError` to classify errors from the layers below it.

This will allow for clean, explicit error propagation and translation at each architectural boundary.

## Phase 1: Define Core and Repository Error Types

We will start at the innermost layers by defining the new error contracts.

### 1.1. Define `RepositoryError` in `lifestats-core`

- **Action:** In a new file `core/core/src/statistic/ports/error.rs`, define a `RepositoryError` enum. This error type will become part of the public contract for all persistence ports.
- **Justification:** By defining the error type alongside the port (trait), we decouple the application from the specific errors of the persistence implementation (e.g., `diesel::Error`).

  ```rust
  // In: core/core/src/statistic/ports/error.rs
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum RepositoryError {
      #[error("Resource not found")]
      NotFound,

      #[error("Operation failed: {0}")]
      OperationFailed(#[from] anyhow::Error),
  }
  ```

- **Action:** Expose this new module by adding `pub mod error;` to `core/core/src/statistic/ports/mod.rs`.

### 1.2. Update the `StatisticRepository` Port

- **Action:** In `core/core/src/statistic/ports/statistic_repository.rs`, update all method signatures to return `Result<..., RepositoryError>` instead of `anyhow::Result`.
- **Justification:** This enforces the new, explicit error contract for all repository implementations.

  ```rust
  // In: core/core/src/statistic/ports/statistic_repository.rs
  use super::error::RepositoryError; // Add this import

  // ...

  #[cfg_attr(any(test, feature = "testing"), mockall::automock)]
  pub trait StatisticRepository {
      // Change every method signature like this:
      // Before: fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
      // After:
      fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic, RepositoryError>;

      // ... update all other methods in the trait ...
      fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>, RepositoryError>;
      fn save_metadata(&self, statistic: &AnyStatistic) -> Result<(), RepositoryError>;
      fn find_log_value(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>, RepositoryError>;
      // etc.
  }
  ```

### 1.3. Update the `PlotRepository` Port (in `plot-domain`)

- **Action:** Perform the same change for the `PlotRepository` trait in `plot-domain`. You can add a similar `error.rs` module in `plot-domain` or, for simplicity in this project, have `plot-domain` depend on `lifestats-core` and reuse the same `RepositoryError`. Let's reuse it.
- **Instruction:**
  1.  Ensure `plot-domain/Cargo.toml` has `lifestats-core` as a dependency.
  2.  Update `core/plot-domain/src/repository.rs` to use `lifestats_core::statistic::ports::error::RepositoryError`.

  ```rust
  // In: core/plot-domain/src/repository.rs
  use lifestats_core::statistic::ports::error::RepositoryError; // Add this import

  // ...

  #[cfg_attr(feature = "testing", mockall::automock)]
  pub trait PlotRepository: Send + Sync {
      // Change every method signature:
      fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>, RepositoryError>;
      // ... update all other methods ...
  }
  ```

## Phase 2: Implement the Application Error Layer

This is the central translation layer that gives context to errors from lower layers.

### 2.1. Define `ApplicationError`

- **Action:** In a new file `core/application/src/error.rs`, define the `ApplicationError` enum.
- **Justification:** This enum classifies errors into categories that the outer layers (like the API) can act upon, such as distinguishing a user error from a server error.

  ```rust
  // In: core/application/src/error.rs
  use thiserror::Error;
  use lifestats_core::statistic::{error::DomainError, ports::error::RepositoryError};

  #[derive(Debug, Error)]
  pub enum ApplicationError {
      #[error("A business rule was violated: {0}")]
      Domain(#[from] DomainError),

      #[error("A persistence error occurred: {0}")]
      Repository(#[from] RepositoryError),

      #[error("An unexpected infrastructure error occurred: {0}")]
      Infrastructure(#[from] anyhow::Error),
  }

  pub type ApplicationResult<T> = Result<T, ApplicationError>;
  ```

- **Action:** Expose this module by adding `pub mod error;` to `core/application/src/lib.rs`.

### 2.2. Update Application Services

- **Action:** Go through every service in `core/application/src/use_cases/` (`LoggingService`, `GoalManagementService`, `PlottingService`, `StatisticQueryService`).
- **Instruction:** Change the return type of every public method from `anyhow::Result<...>` to `ApplicationResult<...>` (which is `Result<..., ApplicationError>`).
- **Justification:** This forces the application services to conform to the new error handling strategy. The `#[from]` attributes on `ApplicationError` mean that in most cases, you will only need to change the function signature, and the `?` operator will handle the error conversion automatically.

  ```rust
  // Example in core/application/src/use_cases/logging_service.rs
  use crate::error::ApplicationResult; // Add this import

  // ...

  // Before: pub fn log_value(&self, command: LogValueCommand) -> Result<()>
  // After:
  pub fn log_value(&self, command: LogValueCommand) -> ApplicationResult<()> {
      // ... implementation remains the same ...
      // The `?` operator on repository or domain calls will now
      // automatically convert RepositoryError/DomainError into ApplicationError.
      let statistic = self.statistic_repository.find_metadata_by_id(command.statistic_id)?;

      // ...
      Ok(())
  }
  ```

## Phase 3: Update Adapters (Persistence and Web API)

Now we update the concrete implementations to conform to the new contracts.

### 3.1. Update `lifestats-persistence` Adapter

- **Action:** In `core/infrastructure/persistence/src/repository.rs` and `plot_repository.rs`, update all method signatures to match the traits they implement (`Result<..., RepositoryError>`).
- **Action:** Map `diesel::Error` to `RepositoryError`. You can do this by creating a `From` implementation.

  ```rust
  // Add this to a relevant place, e.g., at the top of repository.rs
  use lifestats_core::statistic::ports::error::RepositoryError;

  impl From<diesel::Error> for RepositoryError {
      fn from(err: diesel::Error) -> Self {
          match err {
              diesel::Error::NotFound => RepositoryError::NotFound,
              _ => RepositoryError::OperationFailed(err.into()),
          }
      }
  }
  ```

- **Action:** Update the method implementations to handle the error conversion. The `?` operator or `.map_err` will now work cleanly.

  ```rust
  // Example in core/infrastructure/persistence/src/repository.rs
  fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic, RepositoryError> {
      let mut conn = self.connection().map_err(|e| RepositoryError::OperationFailed(e))?;

      let dto = statistics::table
          .find(id.0 as i32)
          .first::<StatisticDto>(&mut conn)?; // The `?` will now convert diesel::Error to RepositoryError

      // ...
      Ok(dto.into_any_statistic(default_goal)?) // This `?` handles anyhow conversion
  }
  ```

- **Note:** You will also need to update mocks and tests that use the repositories to align with the new error types.

### 3.2. Update `web-api` Adapter

- **Action:** In `core/infrastructure/web_api/src/error.rs`, remove the `From<StatisticError>` and `From<DomainError>` implementations for `ApiError`.
- **Action:** Add a single `From<ApplicationError>` implementation for `ApiError`.
- **Justification:** This is the final and most important translation step. The API layer now only needs to know about `ApplicationError` and can map its variants to the correct HTTP status codes without knowledge of the underlying cause.

  ```rust
  // In: core/infrastructure/web_api/src/error.rs
  use lifestats_application::error::ApplicationError;
  use lifestats_core::statistic::ports::error::RepositoryError;

  impl From<ApplicationError> for ApiError {
      fn from(err: ApplicationError) -> Self {
          match err {
              // User-fixable errors -> 400 Bad Request
              ApplicationError::Domain(domain_err) => {
                  ApiError::BadRequest(domain_err.to_string())
              }

              // Not found error -> 404 Not Found
              ApplicationError::Repository(RepositoryError::NotFound) => {
                  ApiError::NotFound("The requested resource was not found.".to_string()) // Add a NotFound variant to ApiError
              }

              // Database or other unexpected errors -> 500 Internal Server Error
              ApplicationError::Repository(err) => {
                  ApiError::InternalServerError(err.into())
              }
              ApplicationError::Infrastructure(err) => {
                  ApiError::InternalServerError(err)
              }
          }
      }
  }
  ```

  _(You will need to add a `NotFound(String)` variant to your `ApiError` enum to handle this case gracefully)._

- **Action:** Update all API route handlers in `routes/*.rs` to return `Result<..., ApiError>`. The `?` operator on application service calls will now work seamlessly.

## Phase 4: Final Cleanup and Verification

### 4.1. Delete the Old `StatisticError`

- **Action:** In `core/core/src/statistic/error.rs`, **delete** the entire `StatisticError` enum. It should now be completely unused across the workspace.
- **Justification:** This final deletion confirms that the refactoring is complete and all parts of the system have adopted the new layered error strategy.

### 4.2. Verify Full Project Compilation and Testing

- **Action:** Run `cargo check --workspace` and `cargo test --workspace` from the root of the project.
- **Goal:** Ensure the entire workspace compiles and all tests pass. This confirms that the new error handling has been implemented correctly without breaking existing functionality.
