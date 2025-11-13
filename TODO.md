# TODO: Show goals on days without logged data

The goal of this change is:

> **Goals should be visible in plots on all days where a goal is active, even if no data was logged on that day.**

Architecturally, we want:

- The _application layer_ to merge logs + goals over the date range.
- The _plot-domain_ to explicitly represent “no value logged”.
- The _renderers_ to draw goal lines whenever a goal exists, independent of logged data.

---

## 0. High-level behavior to implement

- Iterate over every day in the requested `DateRange`.
- For each date, look up:
  - an optional log value
  - an optional daily goal
  - a default goal (if active on that day)
- Create a `DataPoint` for any day that has **either** a log or a goal.
- Allow `DataPoint` to represent “no logged value”.
- Let axis calculation include both values _and_ goals.
- Let renderers:
  - draw bars/lines only for days with a value
  - draw goal lines on all days with a goal

---

## 1. Domain Model: `plot-domain` crate

### 1.1 Make `DataPoint` value optional

- [ ] **Change `DataPoint` to model missing values explicitly**
  - In the `plot-domain` crate (where `DataPoint` is defined), update:

    ```rust
    pub struct DataPoint {
        pub date: NaiveDate,
        pub value: DrawableData,
        pub color: [u8; 3],
        pub goal: Option<DrawableGoal>,
    }
    ```

    to:

    ```rust
    pub struct DataPoint {
        pub date: NaiveDate,
        pub value: Option<DrawableData>,
        pub color: [u8; 3],
        pub goal: Option<DrawableGoal>,
    }

    impl DataPoint {
        pub fn new(
            date: NaiveDate,
            value: Option<DrawableData>,
            color: [u8; 3],
            goal: Option<DrawableGoal>,
        ) -> Self {
            Self { date, value, color, goal }
        }
    }
    ```

- [ ] **Update any helper constructors / builders**
  - Search for all `DataPoint::new(` callsites and update them to pass `Some(drawable_value)` or `None` as appropriate.
  - The only place that should pass `None` will be the application-layer code for “goal but no log”.

### 1.2 Keep `PlotData` stable

- [ ] **Verify `PlotData` semantics**
  - Confirm that `PlotData` is still:

    ```rust
    pub struct PlotData {
        pub title: String,
        pub points: Vec<DataPoint>,
        pub date_range: DateRange,
        pub plot_type: PlotType,
        pub y_axis: YAxis,
    }
    ```

  - No change required here; the only difference is the meaning of each `DataPoint.value`.

---

## 2. Application Layer: building `PlotData`

> This is the heart of the change: we stop iterating “over logs” and start iterating “over dates”, joining in logs + goals per day.

### 2.1 Refactor `evaluate_goal_achievement` to accept optional log values

- [ ] **Change signature to accept `Option<&Value>`**
  - In the application service that computes `PlotData` (e.g. `GetPlotDataQueryHandler`), update:

    ```rust
    fn evaluate_goal_achievement(
        &self,
        statistic: &AnyStatistic,
        log_value: &serde_json::Value,
        goal_value: Option<&serde_json::Value>,
    ) -> ApplicationResult<Option<Percentage>>
    ```

    to:

    ```rust
    fn evaluate_goal_achievement(
        &self,
        statistic: &AnyStatistic,
        log_value: Option<&serde_json::Value>,
        goal_value: Option<&serde_json::Value>,
    ) -> ApplicationResult<Option<Percentage>>
    ```

- [ ] **Honor `Goal::get_percentage_achieved` semantics**
  - Inside the function, for each `AnyStatistic` variant:
    - Parse `goal_value` into the appropriate goal type (`HabitGoal`, `AmountGoal<_>`, `FinishedInTimeGoal`, `TimeSpansGoal`).
    - Parse `log_value` into the appropriate Rust type **only when `Some(..)`**.
    - Call the goal’s `get_percentage_achieved(value: Option<&T>)`.
  - Return:
    - `Ok(None)` if there is no goal for that day.
    - `Ok(Some(percentage))` otherwise (even if `log_value` was `None`).

  This keeps all “what does missing data mean” logic inside the goal types, not the renderer.

### 2.2 Build a `date -> log` map and `date -> goal` map

- [ ] **Create `log_map` for quick lookup**
  - From `find_log_values_in_range`, build:

    ```rust
    let mut log_map: HashMap<NaiveDate, serde_json::Value> = HashMap::new();
    for log in log_values {
        log_map.insert(log.date, log.value);
    }
    ```

- [ ] **Reuse/keep `daily_goal_map` and `default_goal_json`**
  - Keep (or create, if missing) a `HashMap<NaiveDate, serde_json::Value>` for explicit daily goals retrieved via `find_goals_in_range`.
  - Keep `default_goal_json: Option<serde_json::Value>` and `default_goal_valid_since: NaiveDate` as today.

### 2.3 Iterate over dates in the `DateRange`

- [ ] **Introduce a helper iterator over the date range**
  - If not already present, create a small utility to iterate over all days in a `DateRange`:

    ```rust
    fn dates_in_range(range: &DateRange) -> impl Iterator<Item = NaiveDate> {
        let mut current = range.start;
        std::iter::from_fn(move || {
            if current > range.end {
                None
            } else {
                let d = current;
                current = current.succ_opt().unwrap();
                Some(d)
            }
        })
    }
    ```

  - Keep this helper private to the application layer (or wherever it fits best) to avoid polluting domain objects with infrastructural helpers.

- [ ] **Replace “iterate logs” with “iterate range”**
  - Replace the current `log_values.into_iter().map(|log| ...)` with:

    ```rust
    let mut data_points = Vec::new();

    for date in dates_in_range(&query.date_range) {
        let log_value_json = log_map.get(&date);
        let goal_value_json = daily_goal_map
            .get(&date)
            .or_else(|| {
                if date >= default_goal_valid_since {
                    default_goal_json.as_ref()
                } else {
                    None
                }
            });

        // Skip days with neither log nor goal
        if log_value_json.is_none() && goal_value_json.is_none() {
            continue;
        }

        let percentage = self
            .evaluate_goal_achievement(&statistic, log_value_json, goal_value_json)?;

        let color = percentage_to_color(percentage);

        let drawable_value = log_value_json
            .map(|v| json_value_to_drawable_data(&statistic, v))
            .transpose()?;

        let drawable_goal = goal_value_json
            .map(|g| json_value_to_drawable_goal(&statistic, g))
            .transpose()?;

        data_points.push(DataPoint::new(date, drawable_value, color, drawable_goal));
    }
    ```

- [ ] **Keep the `PlotData` creation logic unchanged**
  - After constructing `data_points`, continue to:

    ```rust
    let y_axis = YAxis::from_data_points(&data_points, query.plot_type)?;
    Ok(PlotData::new(title, data_points, query.date_range, query.plot_type, y_axis))
    ```

  - This preserves the existing separation: application builds `PlotData`, renderer stays dumb.

---

## 3. Y-Axis / Plot Domain Logic

> The axis has to be derived from both values _and_ goals, but now values are optional.

### 3.1 Update axis calculations to handle `Option<DrawableData>`

- [ ] **Integer axis**
  - In `YAxis::calculate_integer_axis` (or equivalent), update value access:

    ```rust
    for point in points {
        if let DrawableData::Integer(val) = &point.value {
            values.push(*val);
        }
        if let Some(DrawableGoal::Integer(val)) = &point.goal {
            values.push(*val);
        }
    }
    ```

    to:

    ```rust
    for point in points {
        if let Some(DrawableData::Integer(val)) = &point.value {
            values.push(*val);
        }
        if let Some(DrawableGoal::Integer(val)) = &point.goal {
            values.push(*val);
        }
    }
    ```

- [ ] **Time-based / TimeFrames axis**
  - Apply the same pattern for `DrawableData::TimeFrames` and `DrawableGoal::TimeFrames`.
  - Only include real values (`Some`) in the collected list; always include goals.

### 3.2 Re-run / adjust axis tests

- [ ] **Update any tests that assume `value` is non-optional**
  - Fix compilation errors by wrapping sample values in `Some(..)`.
  - Add at least one test where:
    - Some `DataPoint`s have `value = None` and a goal, and
    - The axis still includes the goal values correctly.

---

## 4. Adapters: Plot Renderers

> Renderers should never need to know “why” a value is missing—only that it is `None`.

### 4.1 Common principles

- For **bars/lines/markers**, only draw if `point.value.is_some()`.
- For **goal lines**, continue to rely only on `point.goal`.
- Never synthesize dummy values for missing data in the renderer.

### 4.2 Bar chart renderer

- [ ] **Guard drawing by `value.is_some()`**
  - In the bar renderer loop:

    ```rust
    for point in &data.points {
        if let DrawableData::Integer(value) = &point.value {
            // draw bar
        }
    }
    ```

    becomes:

    ```rust
    for point in &data.points {
        if let Some(DrawableData::Integer(value)) = &point.value {
            // draw bar using value
        }
    }
    ```

- [ ] **Leave goal-line drawing logic unchanged**
  - Keep the loop that draws goal lines by iterating over `point.goal` as-is; it will now see goal-only days too.

### 4.3 Line chart renderer

- [ ] **Build line series only from points with values**
  - When constructing the `(x, y)` pairs for the line:

    ```rust
    let line_data: Vec<_> = data
        .points
        .iter()
        .filter_map(|point| {
            if let DrawableData::Integer(v) = &point.value {
                Some((SegmentValue::CenterOf(point.date), *v))
            } else {
                None
            }
        })
        .collect();
    ```

    → update to handle `Option`:

    ```rust
    let line_data: Vec<_> = data
        .points
        .iter()
        .filter_map(|point| {
            if let Some(DrawableData::Integer(v)) = &point.value {
                Some((SegmentValue::CenterOf(point.date), *v))
            } else {
                None
            }
        })
        .collect();
    ```

- [ ] **Horizontal markers, etc.**
  - Any additional series that depend on the logged value must also pattern-match on `Some(DrawableData::…)`.

- [ ] **Keep goal lines independent**
  - Leave the goal-line loop as-is; it only cares about `point.goal`.

### 4.4 TimeFrames renderer

- [ ] **Only draw spans when `value` exists**
  - Update:

    ```rust
    if let DrawableData::TimeFrames(list) = &point.value {
        // draw spans
    }
    ```

    to:

    ```rust
    if let Some(DrawableData::TimeFrames(list)) = &point.value {
        // draw spans
    }
    ```

- [ ] **Draw TimeFrames goals regardless of `value`**
  - Leave goal drawing loop intact; it will now draw goal spans on goal-only days.

---

## 5. Tests

> These tests pin the new behavior and guard against regressions.

### 5.1 Application-level tests for `GetPlotDataQueryHandler`

- [ ] **Test: goal-only day still has a `DataPoint`**
  - Scenario:
    - IntegerAmount statistic.
    - Default goal = `10`, valid from start date.
    - Date range: 2 days.
    - Log only on day 1.
  - Expectations:
    - `plot_data.points.len() == 2`.
    - Day 1: `value = Some(DrawableData::Integer(..))`, `goal = Some(DrawableGoal::Integer(10))`.
    - Day 2: `value = None`, `goal = Some(DrawableGoal::Integer(10))`.

- [ ] **Test: no log + no goal → no `DataPoint`**
  - Confirm that if there are neither logs nor any goal in the date range, `plot_data.points` is empty (preserves existing behavior).

- [ ] **Test: `evaluate_goal_achievement` handles `None`**
  - For each relevant statistic type:
    - Call `evaluate_goal_achievement(stat, None, Some(goal_json))`.
    - Assert:
      - No panic.
      - The returned `Percentage` matches the semantics of the relevant `Goal` implementation.

### 5.2 Axis tests

- [ ] **Test: axis includes goal-only days**
  - Build `DataPoint`s with:
    - Some points: `value = None`, `goal = Some(..)`.
    - Some points: `value = Some(..)`, `goal = None`.
  - Assert:
    - Axis min/max include both actual values and goal values, ignoring days that have neither.

### 5.3 Renderer smoke tests

- [ ] **Bar / line / TimeFrames renderer with goal-only points**
  - Build a small `PlotData` where:
    - One day has `value = Some(..), goal = Some(..)`.
    - One day has `value = None, goal = Some(..)`.
  - Pass it to each renderer (bar, line, timeframes).
  - Assert:
    - Rendering succeeds (no panic / error).
    - Output is created (basic smoke test).

---

## 6. Cleanup & Documentation

- [ ] **Code comments**
  - Add a short comment near `DataPoint.value: Option<DrawableData>` explaining:

    > `None` means “no logged value for this day”; a `DataPoint` may still exist if there is a goal or percentage/color to display.

- [ ] **Developer docs / README**
  - Add a brief note in any internal docs describing the new semantics:
    - Plots now show goals even on days without logged data.
    - The application service is responsible for merging logs + goals per date; renderers stay dumb.

---

By completing these steps, we preserve the Hexagonal/Ports & Adapters structure, keep the plotting domain honest about missing data, and ensure the UI behavior (“goals are visible even when you did nothing”) matches the domain semantics.
