# `TODO.md`: Implement TimeFrames Plot Rendering

This document outlines the tasks required to implement the rendering logic for the `TimeFrames` plot type within the new plotting architecture. We will adapt the existing logic from the `archive` directory.

## Guiding Principles

- **Domain Purity:** The `lifestats-domain` crate must remain completely free of any `plotters`-specific code or types.
- **Adapter Responsibility:** All rendering logic, including coordinate calculations, color conversions, and drawing, belongs in the `plotting` adapter crate.
- **Data Flow:** The `PlottingService` provides the renderer with pure `PlotData`. The renderer's job is to translate this data into a visual representation.

## Phase 1: Implement `TimeFrames` Rendering in the `plotting` Adapter

The primary task is to create a new rendering function within the `PlottersRenderer` and wire it up.

- **1.1: Create a new rendering function in `plotters_renderer.rs`:**
  - In `core/infrastructure/plotting/src/plotters_renderer.rs`, create a new private function: `fn render_time_frames(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>>`.
  - This function will contain the `plotters`-specific logic for drawing a time frames chart.

- **1.2: Adapt the archived rendering logic:**
  - Copy the logic from `archive/domain/infrastructure/plotting/src/backends/plotters/time_frames_plot.rs`.
  - **Refactor the function to work with the new `PlotData` struct:**
    - The function will receive `&PlotData` instead of a long list of individual arguments (`title`, `data`, `x_min`, etc.). All necessary information is now on `data`.
    - The `data.points` vector contains `serde_json::Value`. You will need to deserialize the `value` field of each `DataPoint` into a `TimeFrameList` (e.g., `serde_json::from_value::<TimeFrameList>(point.value.clone())?`).
    - The `archive` code has a helper, `calculate_y_axis_bounds`, that determines the min/max `NaiveTime`. Adapt this to work with the `Vec<DataPoint>` from `PlotData`.
    - All helper functions from `archive/.../plotters/common.rs` (like `get_bar`, `get_horizontal_line`, `get_border`) should be moved into a new `core/infrastructure/plotting/src/common.rs` module if they haven't been already, and used here.

- **1.3: Handle `PlotType` in the `render` method:**
  - In the main `render` method of the `PlottersRenderer` implementation, update the `match` statement to call your new `render_time_frames` function when `data.plot_type` is `PlotType::TimeFrames`.
  - Remove the temporary `Err(anyhow!(...))` placeholder for this plot type.

- **1.4: Add a Unit Test for the Renderer:**
  - In `plotters_renderer.rs`, add a new test function, `test_render_time_frames_plot`.
  - Inside the test:
    1.  Create a sample `PlotData` instance with `plot_type: PlotType::TimeFrames`.
    2.  Populate `data.points` with `DataPoint`s where the `value` field is a `serde_json::Value` representation of a `TimeFrameList`.
    3.  Instantiate `PlottersRenderer` and call its `render` method with the test data.
    4.  Assert that the result is `Ok` and that the returned `Vec<u8>` is a valid PNG (e.g., by checking the first few bytes for the PNG signature: `&[0x89, 0x50, 0x4E, 0x47]`).

## Phase 2: Verify End-to-End Integration

Once the rendering component is complete, ensure it can be correctly invoked through the application.

- **2.1: Create a `PlotSpecification` for a `TimeFrames` plot:**
  - This can be done via a database seed, a temporary test, or by creating a new `PlotSpecification` in an existing end-to-end test.
  - Ensure the `plot_type` is set to `PlotType::TimeFrames`.
  - Ensure the `statistic_id` points to a statistic that logs `TimeFrameList` data (e.g., "Study Time").

- **2.2: Test the Web API Endpoint:**
  - Run the `web-api`.
  - Make a `GET` request to the `/plots/{id}/image` endpoint, where `{id}` is the ID of the `TimeFrames` plot specification you created.
  - Verify that the server returns a `200 OK` status and a `Content-Type: image/png` header.
  - Save the response body to a file (e.g., `output.png`) and confirm that it is a valid, rendered image of the time frames plot.
