# `TODO.md`: Implement Dynamic and Robust Plot Rendering

This plan addresses two critical bugs: the hardcoded Y-axis and empty `TimeFrames` plots. We will implement a robust, domain-driven solution that pushes data interpretation logic into the correct architectural layers.

## Phase 1: Enhance the Domain Model (`lifestats-domain`)

We will enrich our domain's plot data structures to be type-safe and renderer-agnostic, eliminating the need for the renderer to interpret generic JSON.

- **1.1. Create `DrawableData` Enum:**
  - In `lifestats-domain/src/plot/data.rs`, define a new public enum:
    ```rust
    pub enum DrawableData {
        Integer(i64),
        TimeFrames(lifestats_domain::shared::time_frame_list::TimeFrameList),
    }
    ```
  - This enum represents data that has been processed and is ready for visualization.

- **1.2. Update `DataPoint` Struct:**
  - In `lifestats-domain/src/plot/data.rs`, modify the `DataPoint` struct.
  - Replace `value: Value` with `value: DrawableData`.
  - Add a `color` field: `pub color: [u8; 3]`. This will store an RGB tuple.
  - The `goal` field can remain `Option<Value>` as it's used for reference, not direct drawing.

- **1.3. Create `YAxis` Enum (As Previously Designed):**
  - Create `lifestats-domain/src/plot/axis.rs` with the `YAxis` enum: `Integer { min: i64, max: i64 }` and `Time { min: chrono::NaiveTime, max: chrono::NaiveTime }`.
  - Implement the `YAxis::from_data_points` factory function. **Crucially, this function will now `match` on the `DrawableData` enum inside each `DataPoint`**, which is much safer and cleaner than parsing JSON.
  - Add `pub y_axis: YAxis` to the `PlotData` struct.

## Phase 2: Implement Business Logic in the Application Layer (`lifestats-application`)

The `PlottingService` will now be responsible for interpreting raw data, calculating colors, and preparing the type-safe `PlotData` object.

- **2.1. Create a Color Mapping Helper:**
  - In a new module within `lifestats-application`, create a helper function `fn percentage_to_color(percentage: Percentage) -> [u8; 3]` that maps goal achievement percentages to RGB color tuples (e.g., >=100% is green, <90% is red).

- **2.2. Heavily Refactor `PlottingService::get_plot_data`:**
  - This method has a new, critical responsibility.
  - **Logic:**
    1.  Load `plot_spec`, `log_values`, and `goals` as before.
    2.  Load the `AnyStatistic` metadata using the `statistic_repository.find_metadata_by_id`. This is essential for accessing domain logic.
    3.  Iterate through the `log_values` to create the `points: Vec<DataPoint>`. For each log entry:
        a. **Convert `serde_json::Value` to `DrawableData`:** Use a `match plot_spec.plot_type` to safely deserialize the JSON `value` into the correct `DrawableData` variant (e.g., `DrawableData::Integer` or `DrawableData::TimeFrames`). If deserialization fails, return an `Err`. **This eliminates the silent failure.**
        b. **Calculate Color:** Use the loaded `AnyStatistic`, the `log_value`, and the corresponding `goal` to call the domain's goal achievement logic (e.g., `any_stat.get_percentage_achieved(...)`).
        c. Pass the resulting percentage to your new `percentage_to_color` helper.
        d. Instantiate `DataPoint` with the type-safe `DrawableData` and the calculated `color`.
    4.  After building the `points` vector, call `YAxis::from_data_points(&points, plot_spec.plot_type)?` to get the axis configuration.
    5.  Construct and return the final, fully-prepared `PlotData` object.

## Phase 3: Simplify the Infrastructure Adapter (`plotting`)

The renderer's role is now much simpler: it receives fully processed data and just needs to draw it.

- **3.1. Refactor `render_time_frames`:**
  - In `plotting/src/plotters_renderer.rs`, find the `render_time_frames` function.
  - **Remove all `serde_json::from_value` calls.** The `point.value` is now a `DrawableData` enum.
  - Use a `match` on `point.value` to get the `TimeFrameList`. If it's not the `TimeFrames` variant, this indicates a logic error upstream, and you should return an `Err`.
  - Use the `point.color` field to color the bars instead of a hardcoded color like `BLUE`.
  - Use the pre-calculated `data.y_axis` for the chart's Y-range.

- **3.2. Refactor `render_bar_chart`, `render_line_chart`, `render_scatter`:**
  - Apply the same pattern:
    1.  Remove hardcoded axis ranges and use `data.y_axis`.
    2.  Remove any data parsing. `match` on `point.value` to get the `i64` from `DrawableData::Integer`.
    3.  Use `point.color` to set the color for bars, circles, etc.
