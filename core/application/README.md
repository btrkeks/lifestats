# Application Service Layer API Reference

This document provides a comprehensive reference for all public functions available in the LifeStats application service layer. These services orchestrate domain logic and coordinate between domain entities and infrastructure adapters.

## Table of Contents

- [Overview](#overview)
- [LoggingService](#loggingservice)
- [StatisticQueryService](#statisticqueryservice)
- [StatisticManagementService](#statisticmanagementservice)
- [GoalManagementService](#goalmanagementservice)
- [HeartbeatSession](#heartbeatsession)
- [PlottingService](#plottingservice)
- [IntegrationService](#integrationservice)

---

## Overview

The application layer follows the **Ports & Adapters (Hexagonal) Architecture** pattern. All services:
- Accept repository ports as dependencies (via dependency injection)
- Contain no business logic (that lives in the domain)
- Orchestrate use cases by coordinating domain entities and infrastructure
- Return `ApplicationResult<T>` for error handling

### Service Construction Pattern

All services follow this construction pattern:

```rust
pub fn new(
    repository: Arc<R>,           // Repository port
    event_publisher: Arc<E>,      // Event publisher (where applicable)
    // ... other dependencies
) -> Self
```

---

## LoggingService

**Location:** `src/use_cases/logging_service.rs`

**Purpose:** Logs values to statistics following the domain's update policies.

### Constructor

```rust
pub fn new(
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
) -> Self
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static
```

### Methods

#### `log_value`

Generic polymorphic method that logs a value to any statistic type.

```rust
pub fn log_value(&self, command: LogValueCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct LogValueCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value_str: String,              // String representation to be parsed
    pub update_policy_str: Option<String>,  // e.g., "sum", "overwrite", "append"
}
```

**Behavior:**
1. Fetches the statistic metadata by ID
2. Parses the value string into the appropriate type
3. Calls pure domain logic to calculate the new value
4. Saves the new value if changed
5. Publishes a `ValueUpdated` event if changed

**Supported Statistic Types:**
- `IntegerAmount` - parses integer values
- `Habit` - parses boolean values ("true"/"false")
- `PointInTime` - parses time values ("HH:MM" or "HH:MM:SS")
- `TimeSpans` - parses JSON array of time frames

**Errors:**
- Returns error if statistic is derived (`Summing` or `TimeDifference`)
- Returns error if value string cannot be parsed
- Returns error if statistic ID not found

**Example:**
```rust
let command = LogValueCommand {
    statistic_id: StatisticId(1),
    date: Local::now().date_naive(),
    value_str: "5".to_string(),
    update_policy_str: Some("sum".to_string()),
};
service.log_value(command)?;
```

---

#### `log_time_frame`

Type-safe method for logging a time frame to a `TimeSpans` statistic.

```rust
pub fn log_time_frame(&self, command: LogTimeFrameCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct LogTimeFrameCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub update_policy: Option<TimeFrameListUpdatePolicy>,
}
```

**Behavior:**
1. Verifies the statistic is a `TimeSpans` type
2. Creates a `TimeFrame` from the start and end times
3. Applies the update policy (defaults to `Append`)
4. Saves the new value if changed
5. Publishes a `ValueUpdated` event if changed

**Errors:**
- Returns error if statistic is not a `TimeSpans` type
- Returns error if time frame is invalid (e.g., end before start)

**Example:**
```rust
let command = LogTimeFrameCommand {
    statistic_id: StatisticId(1),
    date: Local::now().date_naive(),
    start_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
    end_time: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
    update_policy: Some(TimeFrameListUpdatePolicy::Append),
};
service.log_time_frame(command)?;
```

---

## StatisticQueryService

**Location:** `src/use_cases/statistic_query_service.rs`

**Purpose:** Queries statistics, metadata, goals, and computes streaks.

### Constructor

```rust
pub fn new(statistic_repository: Arc<R>) -> Self
where
    R: StatisticRepository + Send + Sync + 'static
```

### Methods

#### `get_all_statistics`

Returns all statistics with their full type information.

```rust
pub fn get_all_statistics(&self) -> ApplicationResult<Vec<AnyStatistic>>
```

**Returns:** All statistics as `AnyStatistic` enum variants.

---

#### `get_statistic_by_id`

Retrieves a single statistic by its ID.

```rust
pub fn get_statistic_by_id(
    &self,
    statistic_id: StatisticId,
) -> ApplicationResult<AnyStatistic>
```

**Errors:** Returns error if statistic ID not found.

---

#### `get_all_metadata`

Returns metadata for all statistics (lightweight, no type info).

```rust
pub fn get_all_metadata(&self) -> ApplicationResult<Vec<StatisticMetadata>>
```

**Returns:** List of `StatisticMetadata` containing ID, name, and category.

---

#### `get_time_spans_metadata`

Returns metadata only for `TimeSpans` statistics.

```rust
pub fn get_time_spans_metadata(&self) -> ApplicationResult<Vec<StatisticMetadata>>
```

**Use Case:** Useful for UIs that need to filter by statistic type.

---

#### `get_metadata`

Retrieves metadata for a specific statistic.

```rust
pub fn get_metadata(&self, statistic_id: StatisticId) -> ApplicationResult<StatisticMetadata>
```

---

#### `get_default_goal`

Retrieves the default goal for a statistic as JSON.

```rust
pub fn get_default_goal(
    &self,
    statistic_id: StatisticId,
) -> ApplicationResult<Option<serde_json::Value>>
```

**Returns:** `Some(goal_json)` if a default goal exists, `None` otherwise.

---

#### `get_goal`

Retrieves the goal for a specific date as JSON.

```rust
pub fn get_goal(
    &self,
    statistic_id: StatisticId,
    date: NaiveDate,
) -> ApplicationResult<Option<serde_json::Value>>
```

**Returns:** `Some(goal_json)` if a goal exists for the date, `None` otherwise.

**Note:** This returns the per-day override goal only. Use domain logic or `get_streak` to fall back to default goals.

---

#### `get_streak`

Computes the current streak for a statistic based on goal achievement.

```rust
pub fn get_streak(&self, statistic_id: StatisticId) -> ApplicationResult<u32>
```

**Behavior:**
1. Loads the statistic and its default goal
2. Iterates backward from today up to 90 days
3. For each day:
   - Loads the logged value and per-day goal
   - Falls back to default goal if no per-day goal exists
   - Evaluates goal achievement using domain logic
4. Stops counting on first missed day
5. Returns the total streak count

**Returns:** Number of consecutive days the goal was achieved.

**Example:**
```rust
let streak = service.get_streak(StatisticId(1))?;
println!("Current streak: {} days", streak);
```

---

## StatisticManagementService

**Location:** `src/use_cases/statistic_management_service.rs`

**Purpose:** Creates, updates, and deletes statistics (CRUD operations on metadata).

### Constructor

```rust
pub fn new(
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
) -> Self
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static
```

### Methods

#### `create_statistic`

Creates a new statistic and persists it to the repository.

```rust
pub fn create_statistic(
    &self,
    command: CreateStatisticCommand,
) -> ApplicationResult<AnyStatistic>
```

**Command Structure:**
```rust
pub struct CreateStatisticCommand {
    pub name: String,
    pub category: String,
    pub statistic_type: String,  // "IntegerAmount", "Habit", "PointInTime",
                                  // "TimeSpans", "Summing", "TimeDifference"
    pub source_statistic_ids: Option<Vec<StatisticId>>,  // Required for
                                                          // Summing and TimeDifference
}
```

**Behavior:**
1. Constructs the appropriate `AnyStatistic` variant based on `statistic_type`
2. For derived statistics, validates `source_statistic_ids`:
   - `Summing` requires at least one source ID
   - `TimeDifference` requires exactly two source IDs
3. Saves to repository (repository assigns the ID)
4. Returns the created statistic with its assigned ID

**Errors:**
- Returns error for unknown `statistic_type`
- Returns error if `source_statistic_ids` is missing for derived statistics
- Returns error if `TimeDifference` doesn't have exactly 2 source IDs

**Example:**
```rust
let command = CreateStatisticCommand {
    name: "Push Ups".to_string(),
    category: "Fitness".to_string(),
    statistic_type: "IntegerAmount".to_string(),
    source_statistic_ids: None,
};
let created = service.create_statistic(command)?;
println!("Created statistic with ID: {}", created.get_metadata().id().0);
```

---

#### `update_statistic`

**Status:** Not yet implemented.

```rust
pub fn update_statistic(&self, command: UpdateStatisticCommand) -> ApplicationResult<()>
```

**Note:** Currently returns an error. Future implementation requires domain model enhancements to support immutable updates.

---

#### `delete_statistic`

Deletes a statistic and all its associated data.

```rust
pub fn delete_statistic(&self, command: DeleteStatisticCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct DeleteStatisticCommand {
    pub statistic_id: StatisticId,
}
```

**Behavior:**
1. Verifies the statistic exists
2. Deletes from repository

**Errors:** Returns error if statistic ID not found.

**Example:**
```rust
let command = DeleteStatisticCommand {
    statistic_id: StatisticId(42),
};
service.delete_statistic(command)?;
```

---

## GoalManagementService

**Location:** `src/use_cases/goal_management_service.rs`

**Purpose:** Manages goals for statistics (both per-day goals and default goals).

### Constructor

```rust
pub fn new(
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
) -> Self
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static
```

### Methods

#### `set_goal`

Sets a goal for a specific date using primitive data (generic for all statistic types).

```rust
pub fn set_goal(&self, command: SetGoalCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct SetGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub goal_data: HashMap<String, String>,  // Key-value pairs for goal parameters
}
```

**Goal Data by Statistic Type:**

| Statistic Type | Required Fields | Optional Fields | Example |
|----------------|----------------|-----------------|---------|
| `IntegerAmount` | `target` (i64) | `is_at_least` (bool, default: true) | `{"target": "100", "is_at_least": "true"}` |
| `Habit` | `occurred` (bool) | None | `{"occurred": "true"}` |
| `PointInTime` | `deadline` (HH:MM or HH:MM:SS) | None | `{"deadline": "14:30"}` |
| `TimeSpans` | `target` (JSON TimeFrameList) | `efficiency` (0-100, default: 100) | `{"target": "[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]", "efficiency": "85"}` |
| `Summing` / `TimeDifference` | `target` (i64 seconds) | `is_at_least` (bool, default: true) | `{"target": "3600"}` |

**Behavior:**
1. Fetches the statistic metadata
2. Parses the `goal_data` HashMap into a strongly-typed goal object
3. Serializes to JSON for persistence
4. Saves to repository
5. Publishes a `GoalUpdated` event

**Errors:**
- Returns error if required field is missing
- Returns error if field has invalid format

**Example:**
```rust
let mut goal_data = HashMap::new();
goal_data.insert("target".to_string(), "100".to_string());
goal_data.insert("is_at_least".to_string(), "true".to_string());

let command = SetGoalCommand {
    statistic_id: StatisticId(1),
    date: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
    goal_data,
};
service.set_goal(command)?;
```

---

#### `set_default_goal_generic`

Sets a new default goal for a statistic using primitive data.

```rust
pub fn set_default_goal_generic(
    &self,
    command: SetDefaultGoalGenericCommand,
) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct SetDefaultGoalGenericCommand {
    pub statistic_id: StatisticId,
    pub goal_data: HashMap<String, String>,  // Same format as SetGoalCommand
}
```

**Behavior:**
1. Fetches the statistic metadata
2. Parses the `goal_data` HashMap into a strongly-typed goal object
3. Updates the domain entity with the new default goal (sets valid_since to today)
4. Saves both the updated metadata and the goal JSON
5. Publishes a `GoalUpdated` event

**Note:** Uses the same `goal_data` format as `set_goal`. See table above.

**Example:**
```rust
let mut goal_data = HashMap::new();
goal_data.insert("deadline".to_string(), "14:30".to_string());

let command = SetDefaultGoalGenericCommand {
    statistic_id: StatisticId(5),
    goal_data,
};
service.set_default_goal_generic(command)?;
```

---

#### `delete_goal`

Deletes a goal for a specific date.

```rust
pub fn delete_goal(&self, command: DeleteGoalCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct DeleteGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}
```

**Behavior:**
1. Deletes the goal for the specified date
2. Publishes a `GoalUpdated` event

**Note:** After deletion, the default goal (if any) will be used for that date.

---

#### `delete_default_goal`

Deletes the default goal for a statistic.

```rust
pub fn delete_default_goal(&self, command: DeleteDefaultGoalCommand) -> ApplicationResult<()>
```

**Command Structure:**
```rust
pub struct DeleteDefaultGoalCommand {
    pub statistic_id: StatisticId,
}
```

**Behavior:**
1. Deletes the default goal
2. Publishes a `GoalUpdated` event

---

## HeartbeatSession

**Location:** `src/use_cases/session_tracking_service.rs`

**Purpose:** Manages heartbeat-based time tracking sessions for `TimeSpans` statistics.

### Constructor

```rust
pub fn new(
    statistic_id: StatisticId,
    repository: Arc<R>,
    time_provider: Arc<TP>,
    config: HeartbeatConfig,
) -> Self
where
    R: StatisticRepository + Send + Sync + 'static,
    TP: TimeProvider + 'static
```

**Configuration:**
```rust
pub struct HeartbeatConfig {
    pub idle_timeout: chrono::Duration,   // Time before session auto-stops
    pub flush_interval: chrono::Duration, // Interval for persisting to database
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            idle_timeout: chrono::Duration::seconds(60),
            flush_interval: chrono::Duration::seconds(60),
        }
    }
}
```

### Methods

#### `beat`

Sends a heartbeat signal to keep the session alive.

```rust
pub fn beat(&mut self)
```

**Behavior:**
- If a session is running, resets the idle timeout
- If no session is running, starts a new session automatically

**Note:** Sessions run in a background thread. Call this method repeatedly to track continuous work.

**Example:**
```rust
// In a loop or timer
session.beat();
```

---

#### `stop`

Manually stops the session and persists the final time frame.

```rust
pub fn stop(&mut self)
```

**Behavior:**
1. Sends a stop signal to the background thread
2. Waits for the thread to finish
3. Persists the final time frame to the repository

**Note:** Sessions are also stopped automatically by the `Drop` implementation.

---

#### `is_running`

Checks if a session is currently active.

```rust
pub fn is_running(&self) -> bool
```

**Returns:** `true` if the background thread is running, `false` otherwise.

---

#### `total_time_elapsed`

Returns the total time tracked in the current session.

```rust
pub fn total_time_elapsed(&self) -> chrono::Duration
```

**Returns:** The cumulative duration since the session started.

**Note:** This is updated every second while the session is active.

---

#### `subscribe_to_updates`

Subscribes to real-time updates about the session state.

```rust
pub fn subscribe_to_updates(&mut self) -> tmpsc::Receiver<HeartbeatUpdate>
```

**Returns:** A channel receiver for `HeartbeatUpdate` events.

**Update Types:**
```rust
pub enum HeartbeatUpdate {
    DurationChanged(chrono::Duration),  // Emitted every second
    Stopped,                             // Emitted when session stops
    Started,                             // Emitted when session starts
}
```

**Example:**
```rust
let mut updates_rx = session.subscribe_to_updates();

tokio::spawn(async move {
    while let Some(update) = updates_rx.recv().await {
        match update {
            HeartbeatUpdate::DurationChanged(d) => {
                println!("Elapsed: {} seconds", d.num_seconds());
            }
            HeartbeatUpdate::Started => println!("Session started"),
            HeartbeatUpdate::Stopped => println!("Session stopped"),
        }
    }
});
```

---

### Time Provider Abstraction

For testing, you can inject a custom `TimeProvider`:

```rust
pub trait TimeProvider: Send + Sync {
    fn now(&self) -> DateTime<Local>;
    fn current_instant(&self) -> Instant;
}

// Production implementation
pub struct RealTimeProvider;
```

---

## PlottingService

**Location:** `src/use_cases/plotting_service.rs`

**Purpose:** Generates plot data and rendered plot images.

### Constructor

```rust
pub fn new(
    statistic_repository: Arc<S>,
    plot_repository: Arc<P>,
    renderer: Arc<R>,
) -> Self
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    R: PlotRenderer + 'static
```

### Methods

#### `get_plot_data`

Retrieves and assembles plot-ready data for a given plot ID and date range.

```rust
pub fn get_plot_data(
    &self,
    plot_id: PlotId,
    date_range: DateRange,
) -> ApplicationResult<PlotData>
```

**Parameters:**
- `plot_id`: The ID of the plot specification
- `date_range`: The date range to fetch data for

**Returns:** `PlotData` containing:
- Title
- Data points with drawable values, colors, and goals
- Date range
- Plot type (LineChart, BarChart, TimeFrames)
- Y-axis configuration

**Behavior:**
1. Loads the plot specification
2. Loads statistic metadata
3. Loads log values and daily goals for the date range
4. For each data point:
   - Converts JSON value to `DrawableData` (type-safe!)
   - Determines the applicable goal (per-day override or default)
   - Evaluates goal achievement percentage
   - Calculates color based on achievement
5. Calculates Y-axis configuration
6. Returns assembled `PlotData`

**Example:**
```rust
let plot_id = PlotId::from(1);
let date_range = DateRange::new(
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
)?;

let plot_data = service.get_plot_data(plot_id, date_range)?;
println!("Plot: {}", plot_data.title);
println!("Points: {}", plot_data.points.len());
```

---

#### `generate_plot_image`

Generates a rendered plot image for a given plot ID, date range, and render configuration.

```rust
pub fn generate_plot_image(
    &self,
    plot_id: PlotId,
    date_range: DateRange,
    config: RenderConfig,
) -> ApplicationResult<Vec<u8>>
```

**Parameters:**
- `plot_id`: The ID of the plot specification
- `date_range`: The date range to plot
- `config`: Rendering configuration (width, height, etc.)

**Returns:** PNG image data as a `Vec<u8>`.

**Behavior:**
1. Calls `get_plot_data` to fetch the data
2. Passes the data to the injected renderer
3. Returns the rendered image bytes

**Example:**
```rust
let config = RenderConfig {
    width: 800,
    height: 600,
    // ... other config
};

let image_bytes = service.generate_plot_image(plot_id, date_range, config)?;
std::fs::write("plot.png", image_bytes)?;
```

---

## IntegrationService

**Location:** `src/use_cases/integration/service.rs`

**Purpose:** Provisions bundles of statistics for external integrations (e.g., Anki).

### Constructor

```rust
pub fn new(
    repository: Arc<dyn StatisticRepository + Send + Sync>,
    factories: Vec<Arc<dyn BundleFactory>>,
) -> Self
```

**Parameters:**
- `repository`: Statistic repository for persistence and querying
- `factories`: List of bundle factories, one per integration source

### Methods

#### `provision_bundle`

Provisions a bundle of statistics for the given source and context.

```rust
pub fn provision_bundle(
    &self,
    source: &str,
    context: HashMap<String, String>,
) -> ApplicationResult<HashMap<String, StatisticId>>
```

**Parameters:**
- `source`: Integration source name (e.g., "anki")
- `context`: Context parameters for the bundle (e.g., `{"deck_name": "Spanish"}`)

**Returns:** A map from logical key (e.g., "cards_studied") to `StatisticId`.

**Behavior (Idempotent):**
1. Constructs a tags map from source + context
2. Queries repository for existing statistics with those tags
3. If found: Returns existing statistic IDs
4. If not found:
   - Looks up the appropriate `BundleFactory` for the source
   - Calls the factory to provision new statistics
   - Persists the new statistics to the repository
   - Returns the new statistic IDs

**Example:**
```rust
let mut context = HashMap::new();
context.insert("deck_name".to_string(), "Spanish Vocabulary".to_string());

let stat_ids = service.provision_bundle("anki", context)?;

println!("Cards studied: {:?}", stat_ids.get("cards_studied"));
println!("Time spent: {:?}", stat_ids.get("time_spent"));
```

**Idempotency Guarantee:**
Calling `provision_bundle` multiple times with the same `source` and `context` will return the same statistic IDs without creating duplicates.

---

## Error Handling

All service methods return `ApplicationResult<T>`, which is an alias for `Result<T, ApplicationError>`.

**ApplicationError variants:**
```rust
pub enum ApplicationError {
    Repository(RepositoryError),     // Repository-layer errors
    Domain(anyhow::Error),           // Domain validation errors
    Infrastructure(anyhow::Error),   // Infrastructure errors
    InvalidInput(String),            // Invalid command parameters
}
```

**Example error handling:**
```rust
match service.log_value(command) {
    Ok(()) => println!("Value logged successfully"),
    Err(ApplicationError::InvalidInput(msg)) => {
        eprintln!("Invalid input: {}", msg);
    }
    Err(ApplicationError::Domain(err)) => {
        eprintln!("Domain error: {}", err);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

---

## Common Patterns

### Dependency Injection

All services accept their dependencies via the constructor:

```rust
let statistic_repo = Arc::new(SqliteStatisticRepository::new(pool));
let event_publisher = Arc::new(MyEventPublisher::new());

let logging_service = LoggingService::new(
    Arc::clone(&statistic_repo),
    Arc::clone(&event_publisher),
);
```

### Event Publishing

Services that modify data publish events asynchronously:

```rust
// In LoggingService:
if changed {
    self.statistic_repository.save_log_value(/* ... */)?;
    self.event_publisher.publish_event_async(Event::ValueUpdated(statistic_id));
}
```

Available events:
- `Event::ValueUpdated(StatisticId)` - Published when a log value changes
- `Event::GoalUpdated(StatisticId)` - Published when a goal changes

### Command Pattern

All mutating operations use command objects:

```rust
pub struct LogValueCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value_str: String,
    pub update_policy_str: Option<String>,
}
```

This pattern provides:
- Clear parameter documentation
- Testability (commands can be constructed in tests)
- Validation at the command level
- Extensibility (new fields can be added without breaking APIs)

---

## Testing

The application layer is designed for testability:

1. **Mock repositories** using `mockall`:
   ```rust
   let mut repo = MockStatisticRepository::new();
   repo.expect_find_metadata_by_id()
       .returning(|_| Ok(test_statistic));
   ```

2. **Mock time** for session tracking:
   ```rust
   struct MockTimeProvider { /* ... */ }
   impl TimeProvider for MockTimeProvider { /* ... */ }
   ```

3. **Test commands** independently:
   ```rust
   let command = LogValueCommand {
       statistic_id: StatisticId(1),
       date: test_date,
       value_str: "5".to_string(),
       update_policy_str: Some("sum".to_string()),
   };
   let result = service.log_value(command);
   assert!(result.is_ok());
   ```

See the test modules in each service file for comprehensive examples.

---

## Related Documentation

- [Architecture Overview](../../docs/ARCHITECTURE.md) - Hexagonal architecture principles
- [Domain Layer](../domain/README.md) - Pure business logic
- [Infrastructure Layer](../infrastructure/README.md) - Repository implementations

---

*Last updated: 2025-10-07*
