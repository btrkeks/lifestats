# `TODO.md`: Implement Integration Bundles

This plan outlines the refactoring and feature work required to introduce the "Integration Bundle" pattern. The primary goal is to centralize the logic for creating sets of related statistics for third-party integrations like Anki, making the system more robust, maintainable, and user-friendly.

We will use the Anki integration as the first implementation of this new pattern.

## Part 1: Enhance the Core Domain & Application Layers

First, we will lay the foundation in `lifestats-core` and `lifestats-application`. These changes are generic and will support all future integrations.

### `lifestats-core`: Domain Model & Repository Port

- **1. Enhance `StatisticMetadata` with Tags:**
  - In `lifestats-core/src/statistic/value_objects/statistic_metadata.rs`, add a new private field: `tags: std::collections::HashMap<String, String>`.
  - Initialize this field to an empty `HashMap` in the `new()` constructor.
  - Add a public builder-style method `with_tag(mut self, key: String, value: String) -> Self` to add a tag.
  - Add a public getter `get_tag(&self, key: &str) -> Option<&String>`.
  - Ensure `tags` are properly handled in serialization/deserialization if you are using `serde`.

- **2. Update `StatisticRepository` Port:**
  - In `lifestats-core/src/statistic/ports/statistic_repository.rs`, add a new method to the `StatisticRepository` trait:
    ```rust
    fn find_by_tags(&self, tags: &std::collections::HashMap<String, String>) -> Result<Vec<AnyStatistic>, RepositoryError>;
    ```
  - _Note to Implementer:_ The persistence adapter (e.g., `SqliteStatisticRepository`) will need to implement this. A common approach is to store tags as a JSONB column in the `statistics` table.

### `lifestats-application`: The Bundle Provisioning Framework

- **1. Create the `integration` Module:**
  - Create a new module directory: `lifestats-application/src/use_cases/integration/`.
  - Add a `mod.rs` file to this new directory.

- **2. Define the `BundleFactory` Trait (The Strategy):**
  - In a new file `lifestats-application/src/use_cases/integration/bundle_factory.rs`, define the `BundleFactory` trait and its associated DTOs. This is the core "Port" for our strategy pattern.

    ```rust
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use crate::error::ApplicationResult;
    use std::collections::HashMap;

    // Represents the output of a provisioning operation.
    pub struct ProvisionedStatistic {
        pub key: String, // e.g., "time_spent", "disruptions"
        pub statistic: AnyStatistic,
    }

    /// A trait for factories that know how to create the set of statistics
    /// for a specific third-party integration.
    pub trait BundleFactory: Send + Sync {
        /// The unique name of the source, e.g., "anki", "fitbit".
        fn source_name(&self) -> &'static str;

        /// Creates the domain entities for the bundle based on context.
        /// The context might contain "deck_name" for Anki, for example.
        fn provision(&self, context: &HashMap<String, String>) -> ApplicationResult<Vec<ProvisionedStatistic>>;
    }
    ```

- **3. Implement the `IntegrationService`:**
  - In a new file `lifestats-application/src/use_cases/integration/service.rs`, create the main orchestrating service.
  - This service will be generic and work with any `BundleFactory`. It should be idempotent.
  - Define the service struct and its constructor, which will take the repository and a `Vec` of factories.
  - Implement the `provision_bundle` method. The logic should be:
    1.  Construct a `tags` map to search for an existing bundle (`source` + context).
    2.  Call `repo.find_by_tags()`. If statistics are found, map them to the `HashMap<String, StatisticId>` response and return early.
    3.  If not found, find the correct `BundleFactory` from the internal map based on the source name.
    4.  Call the factory's `provision` method to generate the `AnyStatistic` entities.
    5.  Iterate through the new statistics, call `repo.save_metadata()` for each one, and collect the newly assigned IDs.
    6.  Return the `HashMap<String, StatisticId>`.

## Part 2: Implement the Anki Integration Bundle

Now we'll create the concrete implementation for Anki using the framework we just built.

### `lifestats-application`: Anki-Specific Logic

- **1. Create the `AnkiBundleFactory`:**
  - In a new file `lifestats-application/src/use_cases/integration/anki_bundle_factory.rs`, create the `AnkiBundleFactory` struct.
  - Implement the `BundleFactory` trait for it.
  - The `source_name()` method should return `"anki"`.
  - The `provision()` method should:
    1.  Extract the `deck_name` from the context `HashMap`. Return an error if it's missing.
    2.  Create three `AnyStatistic` domain objects:
        - **Time Spent:** A `TimeSpansStatistic`. Name it `[Anki] Study Time ({deck_name})`.
        - **Disruptions:** An `IntegerAmountStatistic`. Name it `[Anki] Disruptions ({deck_name})`.
        - **Backlog:** An `IntegerAmountStatistic`. Name it `[Anki] Backlog ({deck_name})`.
    3.  For **each** statistic, use the `.with_tag()` method on `StatisticMetadata` to add two tags: `("source", "anki")` and `("deck_name", deck_name)`.
    4.  Wrap each created statistic in a `ProvisionedStatistic` struct with a unique key (e.g., `"time_spent"`, `"disruptions"`, `"backlog"`) and return the `Vec`.

- **2. Register the Anki Factory:**
  - In the application's main setup function (where you construct services for dependency injection), instantiate `AnkiBundleFactory`.
  - Pass it into the `IntegrationService` constructor's `Vec<Arc<dyn BundleFactory>>`.

## Part 3: Expose via Adapters and Refactor the Client

Finally, we expose the new functionality through the API and update the Anki plugin to use it.

### `web-api` Crate: The REST Endpoint

- **1. Create a New API Endpoint:**
  - Create a new handler for `POST /integrations/:source_name`.
  - This handler should accept the `IntegrationService` via dependency injection (e.g., `axum::State`).
  - It will parse the source name from the URL path and the context (`deck_name`) from the JSON body.
  - It calls `integration_service.provision_bundle()` with the data.
  - On success, it returns a `200 OK` with the JSON payload `{"time_spent_id": X, "disruptions_id": Y, ...}`.
  - On failure, it returns an appropriate error code (e.g., `400 Bad Request` for missing context, `500` for other errors).

### `AnkiLogger` (Python Client):

- **1. Refactor Configuration Logic:**
  - The `AnkiLogger`'s configuration flow needs to be updated. The goal is to move from manual ID entry to an automated provisioning step.
  - Add UI (e.g., a button in the Anki deck options screen) that triggers the provisioning.
  - When triggered, the plugin should make a `POST` request to the new `/integrations/anki` endpoint with the current `deck_name`.

- **2. Update Configuration Storage:**
  - Modify the `config.json` to store the bundle of IDs for each deck. The new structure should look something like this:
    ```json
    {
      "api_base_url": "http://localhost:3000",
      "deck_mappings": {
        "Japanese": {
          "time_spent_id": 4,
          "disruptions_id": 31,
          "backlog_id": 32
        }
      }
    }
    ```

- **3. Implement New Logging Logic:**
  - Add new functions to `statistic_logging.py` to log disruptions and backlog. These will be simple calls to the existing `/log/` endpoint using the newly provisioned IDs.
  - Update the Anki event hooks (`__init__.py`) to correctly identify a "disruption" (e.g., when a logging session starts, increment the disruption count for that day) and log the backlog size when a session ends.
