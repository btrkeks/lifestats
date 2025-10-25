### **Design Blueprint: Integrating Fitbit Activity Lists**

#### 1. Overall Goal

To enable `lifestats` to synchronize specific, named Fitbit activities and store their occurrences as `TimeFrame` entries within corresponding `TimeSpansStatistic`s. The system will be:

- **Efficient:** Fetching the activity list only once per day.
- **Robust:** Handling timezones, midnight-spanning activities, API pagination, and data overlaps gracefully.
- **Extensible:** Allowing new activities to be tracked by adding tagged statistics, without code changes to the sync logic.
- **Architecturally Sound:** Respecting the boundaries between Domain, Application, and Infrastructure, and reusing existing patterns and ports.

#### 2. Refined Data Flow (At Runtime)

The `SyncFitbitDataCommandHandler` will operate as follows:

1.  **Discover Work:** At the start of the sync process, the handler will query the `StatisticRepository` using `find_by_tags` to find all statistics with `source=fitbit` and `metric=activity_log`. It will build an in-memory `HashMap<u64, StatisticId>` mapping a stable Fitbit `activityId` to our internal `StatisticId`.
2.  **Fetch Data Once Per Day:** For each day in the requested sync range, the handler will make a single, paginated API call to `fitbit_api.get_activity_list(date)`. The adapter will handle all pagination, returning a complete `Vec<FitbitActivityLog>` for that day.
3.  **Normalize and Distribute Data:** The handler will iterate through the returned `Vec<FitbitActivityLog>`. For each log, it will perform two critical normalization steps via a dedicated helper (e.g., `TimeNormalizer`):
    a. **Timezone Conversion:** Convert the UTC `start_time` and `duration` to the user's local timezone.
    b. **Midnight Splitting:** If an activity spans midnight (e.g., starts at 23:00, ends at 01:00), split it into two distinct `DatedTimeFrame` objects: one for Day N (23:00-24:00) and one for Day N+1 (00:00-01:00).
4.  **Orchestrate Domain Logic (The `LoggingService` Pattern):** For each normalized `DatedTimeFrame`:
    a. **Match to Statistic:** Look up the activity's `activityId` in the map from Step 1 to find the correct `StatisticId`.
    b. **Load:** Fetch the relevant `TimeSpansStatistic` and the current day's `TimeFrameList` (if any) from the `StatisticRepository`.
    c. **Calculate:** Invoke the domain's business logic: `statistic.calculate_new_log_value(current_value, new_frame, UpdatePolicy::Union)`. The new `Union` policy will deterministically merge the new `TimeFrame` with the existing list.
    d. **Save:** Persist the new, merged `TimeFrameList` back to the repository for that date.

#### 3. Architectural Layer Breakdown

##### **Layer 1: Domain (`lifestats-core`)**

- **`TimeFrameListUpdatePolicy`:** This is a key change. We must implement a robust update policy to handle real-world data.
  - **New `Union` Policy:** Create a new `TimeFrameListUpdatePolicy::Union` enum variant.
  - **Implementation:** Its `apply` method will take an existing `TimeFrameList` and a new one, returning a new list that represents the geometric union of all intervals. It must correctly merge overlapping and adjacent frames.
    - Example: `[8-10]` union `[9-11]` results in `[8-11]`.
    - Example: `[8-10]` union `[10-11]` results in `[8-11]`.
  - **No Panics:** This implementation must **never panic**. It should handle all edge cases of overlaps and adjacencies.
- **`TimeFrameList`:**
  - Consider adding a `merge(&mut self, other: TimeFrameList)` helper method to encapsulate the union logic, which the `Union` policy can then call.

##### **Layer 2: Application (`lifestats-application`)**

- **`integrations/fitbit/ports.rs`:**
  - **`FitbitActivityLog` DTO:** Update the DTO to include the stable ID.
    ```rust
    #[derive(Clone, Debug)]
    pub struct FitbitActivityLog {
        pub activity_id: u64, // The stable numeric ID from Fitbit
        pub name: String,
        pub start_time_utc: DateTime<Utc>,
        pub duration_ms: u64,
    }
    ```
  - **`FitbitDataApi` Port:** Add the `get_activity_list` method, which returns `ApplicationResult<Vec<FitbitActivityLog>>`.
  - **`FitbitMetric` Enum:** Add a new variant, `ActivityLog`, to represent this data category. Update its `canonical_tag_value` and `required_scopes`.
- **`use_cases/commands/sync_fitbit_data.rs`:**
  - **Refactor `SyncFitbitDataCommandHandler`:**
    - **Statistic Mapping:** Use `find_by_tags` to build the `activityId -> StatisticId` map.
    - **Time Normalization:** Create and use a helper struct or function, `TimeNormalizer`, responsible for timezone conversion and midnight splitting. This keeps the main handler logic clean.
    - **Orchestration Logic:** Implement the "Load -> Calculate -> Save" flow described in section 2, step 4. Explicitly pass `TimeFrameListUpdatePolicy::Union` to the `calculate_new_log_value` method.
- **`use_cases/integration/fitbit_bundle_factory.rs`:**
  - **Update `provision` Method:** When creating a `TimeSpansStatistic` for an activity:
    - Add the `metric: activity_log` tag.
    - Add the stable `activity_id` tag (e.g., `activity_id: "90009"` for "Run").
    - Keep the `activity_name: "Run"` tag for human readability.

##### **Layer 3: Infrastructure (`lifestats-fitbit-client`)**

- **`lib.rs`:**
  - **Define DTOs:** Create `ActivityListEnvelope` and `ActivityLog` structs for deserializing the `activities/list.json` response. The `ActivityLog` DTO must include `activityId`, `activityName`, `startTime`, and `duration`.
  - **Implement `get_activity_list`:**
    - This method will now contain a loop to handle pagination.
    - It will construct the initial URL, send the request, and parse the response.
    - If the `pagination.next` URL is present and not empty, it will fetch from that URL and continue until no `next` URL is provided.
    - It will aggregate the `activities` from all pages into a single `Vec`.
    - It will transform the infrastructure DTOs into the clean application-level `FitbitActivityLog` DTOs before returning.
