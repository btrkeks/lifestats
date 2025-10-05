### `TODO.md`

Here is the plan to fix the overflow panic by correcting the logic for preparing plot data.

## Refactor: Ensure Plot Data Preparation Respects Plot Type

### Rationale

The current implementation of `json_to_drawable_data` incorrectly converts `TimeFrameList` data into an integer for `TimeFrames` plots, breaking the renderer's expectations and causing a panic. The fix is to make this helper function aware of the `plot_type` it is preparing data for, ensuring the `DrawableData` variant always matches what the plot type requires.

### ## Application Layer (`lifestats-application`)

#### 1. Update `json_to_drawable_data` to be `plot_type`-aware

- **File:** `core/application/src/helpers.rs`
- **Action:** Modify the function signature to accept `plot_type` and use it to guide the conversion logic.

  ```rust
  // In core/application/src/helpers.rs
  use lifestats_domain::plot::specification::PlotType; // Add this use statement

  pub fn json_to_drawable_data(
      statistic: &AnyStatistic,
      value: &Value,
      plot_type: PlotType, // Add this parameter
  ) -> Result<DrawableData> {
      match plot_type {
          // For plot types that MUST have TimeFrames data:
          PlotType::TimeFrames => {
              let time_frame_list = serde_json::from_value::<TimeFrameList>(value.clone())
                  .map_err(|e| anyhow!("Expected TimeFrameList for TimeFrames plot: {}", e))?;
              Ok(DrawableData::TimeFrames(time_frame_list))
          }
          // For all other plot types, use the generic IntegerStatistic trait:
          PlotType::BarChart
          | PlotType::LineChart
          | PlotType::Scatter
          | PlotType::CircularProgress => {
              let int_value = statistic.value_as_i64_from_json(value).map_err(|_| {
                  anyhow!(
                      "Statistic '{}' cannot be represented as an integer for a {:?} plot.",
                      statistic.get_metadata().name(),
                      plot_type
                  )
              })?;
              Ok(DrawableData::Integer(int_value))
          }
      }
  }
  ```

#### 2. Update the `PlottingService` Call Site

- **File:** `core/application/src/use_cases/plotting_service.rs`
- **Action:** Pass the `plot_spec.plot_type` to the updated helper function.

  ```rust
  // In PlottingService::get_plot_data's .map() closure

  // ... before ...
  // let drawable_value = json_to_drawable_data(&statistic, &log.value)?;

  // ... after ...
  let drawable_value = json_to_drawable_data(&statistic, &log.value, plot_spec.plot_type)?;
  ```

This change ensures that when you're creating a `TimeFrames` plot, the data is always prepared as `DrawableData::TimeFrames`, satisfying the renderer's invariant and fixing the panic. It also correctly leverages the powerful `IntegerStatistic` trait for all other appropriate plot types.
