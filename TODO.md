# Refactor: Consolidate Logging Logic into `LoggingService`

**Objective:** Finalize the service layer refactoring by migrating all logging-related logic into the `LoggingService` and removing the obsolete `log.rs` file. This will establish a single, consistent pattern for all application use cases and eliminate architectural remnants.

---

## 1. Eliminate Orphaned Logging Functions

The file `core/application/src/log.rs` contains outdated logic that bypasses our service architecture. We will migrate its functionality into the `LoggingService` where it belongs.

### 1.1. Enhance `LoggingService` with Type-Safe Methods

- **Task:** Replicate the functionality of the old `log_time_frame` free-standing function as a new, type-safe method on `LoggingService`. This service already has a method with this name, but the one in `log.rs` is slightly different. We need to ensure the service method is robust and complete.
- **Why:** Application logic must reside within services. Providing a type-safe method (`log_time_frame` which takes `NaiveTime`) is preferable to a string-based one (`log_value` which takes a `value_str`) for internal system operations, as it prevents an entire class of parsing errors.
- **Action:**
  1.  Open `core/application/src/use_cases/logging_service.rs`.
  2.  Review the existing `log_time_frame` method. It already correctly implements the required logic (fetching the domain entity, creating the `TimeFrame`, calculating the new value, and saving). Verify that its logic is complete and matches the intent of the old function.
  3.  Ensure the `log_time_frame` method correctly uses the injected `statistic_repository` and `event_publisher` from the `self` context. _The current implementation in the file dump is already excellent and serves as a perfect model._

### 1.2. Consolidate String-Based Logging

- **Task:** Ensure the polymorphic `log_value` method on `LoggingService` fully covers the use cases of the old `log_statistic` function.
- **Why:** The `log_value` method is the public entry point for any external adapter (like a CLI or API) that works with string inputs. It acts as the "anti-corruption layer" that translates raw data into domain concepts.
- **Action:**
  1.  In `core/application/src/use_cases/logging_service.rs`, review the `log_value` method.
  2.  Compare its logic to the old `log_statistic` function in `log.rs`. The service method is far more robust: it correctly fetches the entity, uses pure domain logic for calculation (`calculate_new_log_value`), and handles persistence. The existing implementation is superior and already replaces the old function.

### 1.3. Delete the Obsolete `log.rs` File

- **Task:** Remove the now-redundant `log.rs` file and update the module tree.
- **Why:** This file represents an obsolete architectural pattern. Deleting it prevents confusion and ensures all developers use the `LoggingService` as the single source of truth for logging operations.
- **Action:**
  1.  **Delete the file**: `core/application/src/log.rs`.
  2.  Search the entire codebase for any remaining `use` statements that import from `crate::log` or `lifestats_application::log` and remove them. These should not exist if the `stopwatch_logger` and other adapters were refactored correctly, but it's important to verify.

## 2. Refine Application Crate Structure

With the old files removed, let's ensure the `lifestats-application` crate is clean and well-organized.

### 2.1. Remove `app_context.rs`

- **Task:** Delete the now-defunct `app_context.rs` file.
- **Why:** The `AppContext` struct was part of the old dependency injection pattern. It has been superseded by the `Application` struct in the `lifestats-bootstrap` crate. Removing it finalizes our transition to the new architecture.
- **Action:**
  1.  If you have not already, **delete the file**: `core/application/src/app_context.rs`.
  2.  This file might already be gone or commented out from the previous refactor. This step ensures it is fully removed.

### 2.2. Clean up `lib.rs` Public Exports

- **Task:** Review the public exports (`pub use`) in `lifestats-application/src/lib.rs` to ensure they expose the correct and necessary APIs to the outside world (the adapters).
- **Why:** A crate's `lib.rs` file defines its public API. It should be intentional and clean. We should primarily export the service structs and any necessary command/DTO structs.
- **Action:**
  1.  Open `core/application/src/lib.rs`.
  2.  Ensure it exports the application services and any other types that adapters need to interact with. A good set of exports would look like this:

  ```rust
  // In: core/application/src/lib.rs

  pub mod error;
  pub mod use_cases;
  // helpers module is internal, no need to make it public.

  // Publicly export the services themselves so adapters can use them.
  pub use use_cases::{
      GoalManagementService, LoggingService, PlottingService, StatisticManagementService,
      StatisticQueryService, HeartbeatSession, HeartbeatConfig, HeartbeatUpdate, RealTimeProvider,
      // etc.
  };

  // Publicly export key types that adapters will need.
  pub use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
  pub use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
  ```

After completing these steps, the `lifestats-application` crate will be a clean, consistent, and well-defined layer. The entire system will adhere strictly to our target architecture.
