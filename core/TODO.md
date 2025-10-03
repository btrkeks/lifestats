# TODO: Refactor for Polymorphic Application Services

**Objective:** Decouple the application services from concrete domain types (`TimeSpansGoal`, `TimeFrameList`, etc.). This will make the services truly polymorphic, allowing them to orchestrate any type of statistic without modification. This is a foundational step to improve maintainability and adhere to the Open/Closed Principle.

**Guiding Principle:** Dependencies must always point inward. The `application` layer should depend on the `domain` layer's abstractions, not its concrete implementations. Adapters will be responsible for converting their specific data formats into primitive types that the application services can handle.

---

## Phase 1: Redefine the Repository Port for Polymorphism

Our current `StatisticRepository` port is too generic (`find_log_value<V>`), which forces the application layer to know concrete types. We will refactor it to handle serialized data (`String` or `serde_json::Value`), moving the responsibility of type-safety into the domain and the persistence adapter.

- **1. Modify Port Definition (`lifestats-domain/src/statistic/ports/statistic_repository.rs`):**
  - **Action:** Replace the generic methods for logs and goals with methods that operate on `serde_json::Value`.
  - **Rationale:** This change makes the repository's contract explicit: it is a mechanism for storing and retrieving type-erased JSON data. The application layer no longer needs to know the specific `LogValueType` or `GoalType`.

  ```rust
  // BEFORE:
  // fn find_log_value<V>(&self, ...) -> Result<Option<V>> where V: serde::de::DeserializeOwned + 'static;
  // fn save_log_value<V>(&self, ..., value: V) -> Result<()> where V: serde::Serialize + 'static;
  // (and similar for goals)

  // AFTER:
  fn find_log_value_json(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>>;
  fn save_log_value_json(&self, id: StatisticId, date: NaiveDate, value: serde_json::Value) -> Result<()>;
  fn find_goal_json(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>>;
  fn save_goal_json(&self, id: StatisticId, date: NaiveDate, goal: Option<serde_json::Value>) -> Result<()>;
  fn find_default_goal_json(&self, id: StatisticId) -> Result<Option<serde_json::Value>>;
  fn save_default_goal_json(&self, id: StatisticId, goal: serde_json::Value) -> Result<()>;
  // Note: The `*_in_range` methods should also be updated to return `Vec<DatedValue<serde_json::Value>>`.
  ```

- **2. Update Adapter Implementation (`lifestats-persistence/src/statistics_repository.rs`):**
  - **Action:** Implement the new `*_json` methods. The internal logic will remain similar but will now work directly with serialized strings from the database, converting them to `serde_json::Value`.
  - **Rationale:** The persistence adapter's job is to translate between the database format (serialized strings) and the port's required format (`serde_json::Value`).

  ```rust
  // Example for `find_log_value_json`:
  // 1. Fetch the `LoggingDataRaw` from the database.
  // 2. If a row is found, parse its `value: String` into a `serde_json::Value`.
  //    - `serde_json::from_str(&raw.value)?`
  // 3. Return `Ok(Some(json_value))` or `Ok(None)`.

  // Example for `save_log_value_json`:
  // 1. Serialize the incoming `serde_json::Value` to a `String`.
  //    - `serde_json::to_string(&value)?`
  // 2. Create a `LoggingDataRaw` instance with the string.
  // 3. Perform the Diesel upsert operation.
  ```

## Phase 2: Empower the Domain with Polymorphic Logic

Now, we will add methods to `AnyStatistic` that allow the application layer to "tell" it to perform an action using primitive types, without needing to "ask" for its concrete type.

- **1. Create Polymorphic Logging Method (`lifestats-domain/src/statistic/any_statistic.rs`):**
  - **Action:** Add a new method `calculate_new_log_value_from_primitives` to `AnyStatistic`. This method will take string slices for the value and policy, perform the necessary downcasting and parsing _inside_ the domain, and return a `serde_json::Value` representing the new state.
  - **Rationale:** This is the core of the refactor. It moves the responsibility of parsing and type-specific logic from the application layer into the domain, where it belongs. The application layer can now treat every statistic identically.

  ```rust
  // In `impl AnyStatistic`
  pub fn calculate_new_log_value_from_primitives(
      &self,
      current_value_json: Option<serde_json::Value>,
      new_value_str: &str,
      update_policy_str: Option<&str>,
  ) -> Result<(serde_json::Value, bool)> { // Result is from `anyhow` or your domain error
      // Use the `dispatch_statistic!` macro to call a helper on the inner statistic.
      // The helper will:
      // 1. Deserialize `current_value_json` into its specific `LogValueType`.
      // 2. Call the existing `calculate_new_log_value_from_str` method.
      // 3. Serialize the resulting new `LogValueType` back to `serde_json::Value`.
      // 4. Return `(new_value_json, changed)`.
  }
  ```

- **2. Create Polymorphic Goal Management Methods (`lifestats-domain/src/statistic/any_statistic.rs`):**
  - **Action:** Add `create_goal_from_primitives` and `set_default_goal_from_primitives` methods to `AnyStatistic`. These will accept a `HashMap<String, String>` or similar structure containing the raw goal data.
  - **Rationale:** Similar to logging, this moves goal creation logic into the domain. The application service no longer needs to know how to construct a `TimeSpansGoal` vs. an `AmountGoal`.

  ```rust
  // In `impl AnyStatistic`
  pub fn create_goal_from_primitives(
      &self,
      data: &std::collections::HashMap<String, String>,
  ) -> Result<serde_json::Value> {
      // Dispatch to an inner helper that knows how to parse the HashMap
      // for its specific goal type (e.g., "target" and "efficiency" for TimeSpansGoal).
      // It should construct the concrete goal object and serialize it to JSON.
  }

  pub fn set_default_goal_from_primitives(
      &mut self,
      data: &std::collections::HashMap<String, String>,
      start_date: NaiveDate,
  ) -> Result<serde_json::Value> {
      // Similar to above, but also updates the internal `default_goal` field.
  }
  ```

## Phase 3: Refactor the Application Services

With the port and domain updated, we can now simplify the application services to use the new polymorphic interfaces.

- **1. Refactor `LoggingService` (`lifestats-application/src/use_cases/logging_service.rs`):**
  - **Action:** Replace `log_timeframe_list` with a single, generic method, e.g., `log_value`. This new method will orchestrate the flow using the new polymorphic methods.
  - **Rationale:** This achieves our primary goal. The `LoggingService` becomes a simple orchestrator, completely unaware of whether it's logging a `TimeFrameList`, an `i64`, or a `bool`.

  ```rust
  // BEFORE (in LoggingService):
  // - Checks if statistic is `as_time_spans`.
  // - Fetches `Option<TimeFrameList>`.
  // - Calls `calculate_new_log_value` with strong types.
  // - Saves `TimeFrameList`.

  // AFTER (in a new generic `log_value` method):
  pub fn log_value(&self, command: LogValueCommand) -> Result<()> {
      // 1. Fetch the domain entity
      let statistic = self.statistic_repository.find_metadata_by_id(command.statistic_id)?;

      // 2. Fetch the current value as JSON
      let current_value_json = self.statistic_repository.find_log_value_json(command.statistic_id, command.date)?;

      // 3. TELL, DON'T ASK: Ask the domain object to calculate its new state from primitives.
      let (new_value_json, changed) = statistic.calculate_new_log_value_from_primitives(
          current_value_json,
          &command.value_str,
          command.update_policy_str.as_deref(),
      )?;

      // 4. If changed, save the new JSON state and publish an event.
      if changed {
          self.statistic_repository.save_log_value_json(command.statistic_id, command.date, new_value_json)?;
          self.event_publisher.publish_event_async(Event::ValueUpdated(command.statistic_id));
      }

      Ok(())
  }
  ```

- **2. Refactor `GoalManagementService` (`lifestats-application/src/use_cases/goal_management_service.rs`):**
  - **Action:** Rewrite the `set_goal_for_date` and `set_default_goal` methods to be generic. They will no longer contain any logic specific to `TimeSpansGoal`.
  - **Rationale:** This service will now support setting goals for _any_ statistic type without modification, fulfilling the Open/Closed Principle.

  ```rust
  // Example for `set_goal_for_date`:
  pub fn set_goal_for_date(&self, command: SetGoalCommand) -> Result<()> {
      // 1. Fetch the statistic metadata.
      let statistic = self.statistic_repository.find_metadata_by_id(command.statistic_id)?;

      // 2. Tell the domain object to create a goal from the raw data.
      let goal_json = statistic.create_goal_from_primitives(&command.goal_data)?;

      // 3. Save the resulting JSON via the repository.
      self.statistic_repository.save_goal_json(command.statistic_id, command.date, Some(goal_json))?;

      self.event_publisher.publish_event_async(Event::GoalUpdated(command.statistic_id));
      Ok(())
  }
  ```

- **3. Update Driving Adapters:**
  - **Action:** All driving adapters (CLI, API client, etc.) must be updated to call the new, generic application service methods. The `web_api` client, for example, should send structured JSON rather than formatted strings.
  - **Rationale:** This completes the end-to-end flow, ensuring the entire system uses the new, robust architecture.
