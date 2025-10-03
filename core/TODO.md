# TODO: Decouple Domain from Persistence (Active Record Refactor)

This is a major architectural refactoring with a single goal: to make our core domain model pure and independent of persistence concerns. We are moving from an "Active Record"-like pattern, where entities know how to save themselves, to a clean "Repository" pattern, where the Application Layer orchestrates all data access.

This will dramatically improve testability, clarify the flow of data, and make the system more flexible for future changes. We will proceed layer by layer, starting from the innermost `domain` crate and working our way out to the `persistence` adapter.

## 1. `lifestats-domain`: Redefine Boundaries and Purify Entities

Our first priority is to redefine the contracts (ports) and remove all persistence knowledge from the domain entities.

- [x] **1.1. Redefine Repository Ports (The New Contract):** The existing repository traits are too coarse. We need to define new, more granular traits in the `domain` crate that reflect how the Application Layer will now interact with data.
  - **Action:** In `lifestats-domain/src/statistic/repositories`, create or modify the repository traits. The goal is to separate metadata access from value/goal access.
  - **Example `StatisticRepository` Port:**

    ```rust
    // In lifestats-domain/src/statistic/repositories/statistic_repository.rs
    pub trait StatisticRepository {
        // --- Metadata Operations ---
        fn find_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
        fn find_all(&self) -> Result<Box<[AnyStatistic]>>;
        fn save(&self, statistic: &AnyStatistic) -> Result<()>; // Saves metadata

        // --- Granular Data Operations ---
        // Note: These may be better on separate, more specific repository traits
        // like LogRepository and GoalRepository, which is a good current pattern.
        // Let's refine those.
    }
    ```

  - **Example `LoggingDataRepository` & `GoalRepository` Ports:** These should be generic over their stored type.

    ```rust
    // In lifestats-domain/src/statistic/repositories/logging_data_repository.rs
    pub trait LoggingDataRepositoryTrait {
        type StoredType;
        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Self::StoredType>>;
        fn get_in_closed_range(&self, /*...*/) -> Result<Vec<DateValue<Self::StoredType>>>;
        fn insert_or_update(&self, id: StatisticId, date: NaiveDate, new_value: Self::StoredType) -> Result<()>;
    }

    // The GoalRepositoryTrait is similarly structured. This part is already in good shape.
    ```

- [ ] **1.2. Purge Repository Fields from all Domain Entities:** This is the core of the refactoring. Entities must become pure data + logic containers. They should not hold references to repositories.
  - **Action:** Go through every struct in `lifestats-domain/src/statistic/entities/` (e.g., `TimeSpansStatistic`, `IntegerAmountStatistic`, `HabitStatistic`, etc.).
  - **Remove** the `logged_values` and `goals` fields from each of them. These structs will now only contain their `metadata` and potentially other configuration.

- [ ] **1.3. Refactor Entity Methods to be Pure Functions:** Since entities no longer have repositories, their methods must be changed to accept data as arguments instead of fetching it themselves.
  - **Action:** Modify methods that previously performed I/O.
  - **Example for logging:**
    - **REMOVE** the existing `log_value` method on `LogableStatistic`.
    - **CREATE** a new, pure method like `calculate_new_log_value` on the entity.
    ```rust
    // In IntegerAmountStatistic, for example:
    pub fn calculate_new_log_value(
        &self,
        current_value: Option<i64>,
        new_value: i64,
        policy: AmountUpdatePolicy,
    ) -> (i64, bool) { // Returns (new_value, has_changed)
        policy.apply(current_value, new_value)
    }
    ```
  - **Example for streak calculation:**
    - **REMOVE** the `get_streak` method from the `Statistic` trait. This logic is too complex for a simple entity method that now has no data access.
    - **CREATE** an `enum StreakDayResult { Achieved, Missed, NoGoalContinue }`.
    - **CREATE** a new pure method `evaluate_streak_for_day` on `AnyStatistic`. This method will contain the core business rule for what constitutes a successful day.
    ```rust
    // On AnyStatistic
    pub fn evaluate_streak_for_day(&self, value: Option<&dyn Any>, goal: Option<&dyn Any>) -> StreakDayResult {
        // Polymorphically dispatch to the correct statistic type's implementation.
        // The implementation will cast the `Any` trait objects and use the goal's
        // `is_achieved` method.
        // ...
    }
    ```

- [ ] **1.4. Cleanup `Statistic` Trait:** The base `Statistic` trait in `traits/base_statistic.rs` contains methods that imply self-fetching (`get_log_value`, `get_goal`). These must be removed.
  - **Action:** Edit the `Statistic` trait.
  - **REMOVE** methods like `get_log_value`, `get_log_values_closed_range`, `get_goal`, `get_score`. The Application layer will be responsible for composing this logic now.
  - **KEEP** methods that relate to pure metadata or type information, like `get_metadata`, `id`, `get_name`.

---

## 2. `lifestats-application`: Become the Orchestrator

This layer now takes on the critical responsibility of managing the flow of data and coordinating the domain and persistence layers.

- [ ] **2.1. Formalize Use Cases as Application Services:** Replace the free-standing functions in `use_cases/` with formal service structs. This improves dependency management and clarifies the application's API.
  - **Action:** In the `use_cases` module, create new files for services.
  - **Example `LoggingService`:**

    ```rust
    // In lifestats-application/src/use_cases/logging_service.rs
    pub struct LoggingService {
        stats_repo: Arc<dyn StatisticRepository>, // Or more granular repos
        log_repo_factory: Arc<dyn LogRepositoryFactory>, // A way to get type-specific repos
        event_publisher: Arc<dyn EventPublisher>,
    }

    impl LoggingService {
        pub fn new(/*...dependencies...*/) -> Self { /*...*/ }
        // Use case methods will go here
    }
    ```

  - Create similar services: `GoalManagementService`, `StatisticQueryService`, etc.

- [ ] **2.2. Implement the "Orchestration" Pattern:** Refactor all use cases to follow the `fetch -> process -> save` pattern.
  - **Action:** Rewrite the logic inside the new service methods.
  - **Example `log_integer_value` in `LoggingService`:**

    ```rust
    pub fn log_integer_value(&self, command: LogValueCommand<i64>) -> Result<()> {
        // 1. FETCH metadata from repository
        let statistic = self.stats_repo.find_by_id(command.statistic_id)?;

        // 2. FETCH current data from repository
        let int_log_repo = self.log_repo_factory.integer_repo(); // Get specific repo
        let current_value = int_log_repo.get(command.statistic_id, command.date)?;

        // 3. PROCESS using the pure domain entity
        if let AnyStatistic::IntegerAmount(int_stat) = statistic {
            let (new_value, changed) = int_stat.calculate_new_log_value(
                current_value,
                command.value,
                command.policy,
            );

            // 4. SAVE the result back to the repository (if changed)
            if changed {
                int_log_repo.insert_or_update(command.statistic_id, command.date, new_value)?;
                self.event_publisher.publish(Event::ValueUpdated(command.statistic_id));
            }
        } else {
            return Err(anyhow!("Statistic is not of type IntegerAmount"));
        }

        Ok(())
    }
    ```

  - Apply this same pattern to all other use cases (setting goals, updating values, etc.).

- [ ] **2.3. Implement the `get_streak` Driving Loop:** Implement the dynamic, day-by-day fetching logic for the streak calculation as we designed.
  - **Action:** Create a `get_streak` method in the `StatisticQueryService`.
  - **Implementation:**

    ```rust
    pub fn get_streak(&self, statistic_id: StatisticId) -> Result<u32> {
        let statistic = self.stats_repo.find_by_id(statistic_id)?;
        let mut streak = 0;
        let mut current_date = chrono::Local::now().date_naive();
        const MAX_LOOKBACK_DAYS: i64 = 90; // Safety break

        for i in 0..MAX_LOOKBACK_DAYS {
            let date_to_check = current_date - chrono::Duration::days(i);

            // 1. FETCH data for a single day
            let value = self.log_repo.get(statistic_id, date_to_check)?; // Simplified
            let goal = self.goal_repo.get(statistic_id, date_to_check)?;  // Simplified

            // 2. PROCESS with the pure domain method
            match statistic.evaluate_streak_for_day(value.as_ref(), goal.as_ref()) {
                StreakDayResult::Achieved => streak += 1,
                StreakDayResult::Missed => break, // Streak broken, stop fetching.
                StreakDayResult::NoGoalContinue => continue,
            }
        }
        Ok(streak)
    }
    ```

---

## 3. `persistence-sqlite`: Implement the New Data Access Contract

The persistence layer must now be updated to implement the new, more granular repository traits defined in the domain.

- [ ] **3.1. Update `StatisticRepository` Implementation:** Modify `persistence/src/statistics_repository.rs`.
  - **Action:** The `get_by_id` and `get_all` methods will now be responsible for "hydrating" the pure `AnyStatistic` objects (which now only contain metadata). This should be simpler than before.
  - **Action:** The `save` method will now only be responsible for inserting/updating the `statistics` table row.

- [ ] **3.2. Ensure `LoggingDataRepository` and `GoalRepository` are Correct:**
  - **Action:** Review the existing implementations (`LoggingDataRepositoryTemplate`, `GoalRepositoryTemplate`). They already follow a good, granular pattern. Ensure their method signatures match the traits defined in the domain layer perfectly. They are now the primary way the Application Services will fetch and save daily data.

- [ ] **3.3. Update Unit and Integration Tests:** The entire testing landscape will change.
  - **Action:** Update domain unit tests. They will become much simpler as they no longer need repository mocks.
  - **Action:** Update application-layer tests. They will need to mock the new repository traits (`StatisticRepository`, `LoggingDataRepository`, etc.) and verify that the service methods call the correct repository methods in the correct order.
  - **Action:** Update integration tests. They will now construct the real `Sqlite...Repository` implementations, inject them into the application services, and test the full flow from the service layer down to the database.
