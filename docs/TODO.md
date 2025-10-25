# TODO: Implement Cross-Day TimeFrame Support

This series of tasks implements the "Date-Offset Model" to allow `TimeFrame` and `TimeFrameList` to accurately represent events that span across midnight, such as a night's sleep.

The core strategy is to enrich our domain model with a new `TimePoint` value object that includes a date offset. This localizes the complexity within the domain model, ensuring the rest of the application works with a more robust and expressive representation of time.

**Associated Breaking Change:** The JSON serialization format for `TimeFrameList` will change. Any existing stored data or API clients relying on the old format will need to be updated.

- **Old Format:** `[{"start":"22:00:00","end":"23:00:00"}]`
- **New Format:** `[{"start":{"date_offset":-1,"time":"22:00:00"},"end":{"date_offset":0,"time":"07:00:00"}}]`

---

## 1. Domain Model (`lifestats-core` crate)

We begin by establishing the new foundational value objects in the core domain crate. These changes are pure, have no external dependencies, and enable all subsequent work.

- **1.1. Define `TimePoint` Value Object:**
  - **Action:** In the `lifestats-core` crate, create a new file `src/shared/time_point.rs` and `mod.rs` entry.
  - **Implementation:** Define a new public struct `TimePoint` that contains `date_offset: i8` and `time: NaiveTime`. Derive `Debug`, `Copy`, `Clone`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Serialize`, and `Deserialize`.
  - **Rationale:** This struct is the cornerstone of the new model. Making it `Ord` is critical, as it allows `TimeFrame`s to be sorted correctly across day boundaries, which is essential for `TimeFrameList`'s invariants.

- **1.2. Refactor `TimeFrame` to Use `TimePoint`:**
  - **Action:** In `lifestats-core/src/shared/time_frame_list.rs`, modify the `TimeFrame` struct.
  - **Implementation:** Replace the `start: NaiveTime` and `end: NaiveTime` fields with `start: TimePoint` and `end: TimePoint`. Update the `TimeFrame::new` constructor accordingly. The validation `start > end` will continue to work correctly due to the `Ord` derive on `TimePoint`.
  - **Rationale:** This adapts the `TimeFrame` to understand cross-day contexts, making it a more powerful value object.

- **1.3. Update `TimeFrame::duration()` Logic:**
  - **Action:** In `lifestats-core/src/shared/time_frame_list.rs`, rewrite the `duration()` method in `TimeFrame`.
  - **Implementation:** The new logic must account for the `date_offset` of both the start and end `TimePoint`s to calculate the total duration in seconds. The formula is `(end_offset - start_offset) * seconds_in_a_day + (end_time - start_time)`.
  - **Rationale:** This is a critical business logic change. The `duration` must now correctly calculate spans that are longer than 24 hours or cross the midnight boundary.

- **1.4. Update `TimeFrame` Deserialization and Parsing:**
  - **Action:** In `lifestats-core/src/shared/time_frame_list.rs`, update any custom logic that assumes `start` and `end` are simple time strings.
  - **Implementation:** The `impl FromStr for TimeFrame` is now likely invalid or ambiguous. Remove it or adapt it to a new, unambiguous string format if required (e.g., `"-1@22:00-0@07:00"`). The `serde` derives on `TimePoint` should handle JSON correctly, but verify this with a unit test.
  - **Rationale:** Our data formats (both for storage and APIs) must be unambiguous. We are prioritizing the structured JSON format over a concise string format for clarity.

- **1.5. Verify `TimeFrameList` Logic:**
  - **Action:** In `lifestats-core/src/shared/time_frame_list.rs`, review the methods of `TimeFrameList`, especially `update_end_value`.
  - **Implementation:** The `update_end_value` method should now find a frame by its complete `start: TimePoint`, not just a `NaiveTime`. Add unit tests to `time_frame_list.rs` that specifically test cross-day scenarios: appending, updating, and rejecting overlaps for frames like `(-1@22:00 - 0@01:00)`.
  - **Rationale:** We must ensure the core collection invariants (sorted, non-overlapping) hold true with the new, richer `TimeFrame` definition.

---

## 2. Application Layer (`lifestats-application` crate)

With the domain model updated, we will now refactor the application services that create `TimeFrame` instances.

- **2.1. Refactor `HeartbeatSession` Persistence Logic:**
  - **Action:** In `lifestats-application/src/use_cases/session_tracking_service.rs`, modify the `Heart::persist_time_frame` method. It should no longer accept `NaiveTime` arguments.
  - **Implementation:**
    1.  Change the method to work with the absolute `start_time: DateTime<Local>` and `end_time: DateTime<Local>` from the `HeartTiming` struct.
    2.  Establish the canonical date for the log entry using `let canonical_date = end_time.date_naive()`.
    3.  Calculate the `date_offset` and `time` for the start `TimePoint` relative to the `canonical_date`.
    4.  Calculate the `date_offset` (which will be `0`) and `time` for the end `TimePoint`.
    5.  Construct the `TimeFrame` using these `TimePoint`s and proceed with saving it.
  - **Rationale:** This adapts the highest-complexity use case to the new domain model, ensuring that time tracking sessions that run overnight are persisted correctly as a single logical entry.

- **2.2. Update `FitbitSleepData` Transformation:**
  - **Action:** In `lifestats-application/src/integrations/fitbit/ports.rs`, refactor `FitbitSleepData::into_main_sleep_time_frames`.
  - **Implementation:** The method should use the absolute `fell_asleep_at_utc` and `woke_up_at_utc` `DateTime`s. It must convert them to the user's local timezone, determine the canonical log date (e.g., the local date of `woke_up_at_utc`), and then construct a single `TimeFrame` with the correct `TimePoint` offsets, similar to the `HeartbeatSession` logic.
  - **Rationale:** The Fitbit integration is a primary source of overnight data. This change ensures that sleep data is correctly mapped to our new, more accurate domain model.

---

## 3. Adapters & Data Formats

Finally, ensure any external interfaces are updated to handle the new model. This task is a placeholder for a broader review of all adapters.

- **3.1. Review and Update API/CLI Adapters:**
  - **Action:** Identify any endpoints or commands that allow users to manually input a `TimeFrame`.
  - **Implementation:** The input format must be made unambiguous. Prefer accepting two full `DateTime` strings instead of a simple time range like `"22:00-01:00"`. The adapter logic will be responsible for parsing these and constructing the `TimePoint`s for the `LogValueCommand`.
  - **Rationale:** To prevent ambiguity and bugs, the system's boundaries must enforce a clear and explicit data contract. We shift the responsibility of parsing potentially ambiguous user input entirely to the adapter layer.

- **3.2. Plan for Data Migration (If Necessary):**
  - **Action:** Analyze if there is existing `TimeFrameList` data in production storage.
  - **Implementation:** If data exists, create a one-off migration script to convert the old JSON format to the new `TimePoint`-based format.
  - **Rationale:** This ensures data continuity and prevents runtime deserialization errors after the application is deployed with the new domain model.
