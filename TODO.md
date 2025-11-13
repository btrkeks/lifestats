# TODO: Refactor Plotting to a Stateless, On-the-Fly Model

## 🎯 Objective

Refactor the plotting subsystem from a stateful, pre-configured model to a **stateless, on-the-fly generation** model.

- Eliminate stored plot configurations (`PlotSpecification`, `PlotRepository`, `plots` table).
- Treat rendering as an external concern behind a `PlotRenderer` port.
- Let the application assemble a complete `PlotData` domain object per request.
- Offer both **rendered images** and **raw plot data** via HTTP in a clean, testable architecture.

## 🏛️ Guiding Principles

- **Ports & Adapters (Hexagonal Architecture)**
  - `plot-domain` defines _what_ a plot is and the `PlotRenderer` port.
  - `lifestats-plotting` implements the port using a concrete library (e.g., `plotters`).
  - `lifestats-application` orchestrates data fetching and `PlotData` assembly.
  - `lifestats-web-api` translates HTTP ↔ application ↔ renderer.

- **Dependency Inversion**
  - Application and web layers depend on `plot-domain` abstractions, not on `lifestats-plotting`.

- **Single Responsibility**
  - Each crate owns one clear concern:
    - `plot-domain`: renderer-agnostic plot concepts.
    - `lifestats-application`: use case logic to build `PlotData`.
    - `lifestats-plotting`: `PlotData` → image bytes.
    - `lifestats-web-api`: HTTP routing, validation, and response formatting.

- **Pragmatism**
  - Prefer incremental, test-backed changes over big-bang rewrites.
  - Avoid premature generalization; keep abstractions as simple as possible.

---

## Phase 1: Solidify the Domain Layer (`plot-domain`)

Goal: Ensure the domain describes plots in a complete, renderer-agnostic way and exposes a clean port for rendering.

### 1. Refine the Central `PlotData` Domain Object

- **Action**
  - Review and refine `plot-domain::plot::data::PlotData` so it contains all information needed to describe a plot:
    - Plot metadata: `title`, optional `subtitle`, `PlotType`, etc.
    - Series data: a vector of `DataPoint`s, each with at least:
      - `date` (or time index),
      - raw value(s),
      - goal-related value(s),
      - computed `Percentage` of goal, if applicable,
      - any additional labels/flags needed for rendering.
    - Axis configuration: `YAxis` and (if needed) `XAxis` or other axes.
  - Verify `PlotData` does **not** reference rendering library types.

- **Guidelines**
  - Keep `PlotData` purely semantic: “what to draw”, not “how to draw it”.
  - Ensure it’s stable enough that multiple frontends or renderers can reuse it.

### 2. Clarify and Restrict `RenderConfig`

- **Action**
  - Define or refine `RenderConfig` in `plot-domain::plot::renderer`:
    - Include **semantic knobs** only, e.g.:
      - theme (e.g. `Theme::Light | Theme::Dark`),
      - whether to show goal lines, markers, grid, etc.,
      - high-level size categories if needed (e.g. `Small | Medium | Large`).
  - Avoid leaking low-level rendering details into the domain:
    - Do **not** include concrete pixel dimensions, font names, or library-specific types in `RenderConfig`.

- **Rationale**
  - Keeps `plot-domain` independent from any specific plotting library.
  - Makes it easy to swap `lifestats-plotting` implementation later.

### 3. Define the `PlotRenderer` Port

- **Action**
  - In `core/plot-domain/src/plot/renderer.rs`, define the port:

    ```rust
    pub trait PlotRenderer: Send + Sync {
        fn render(
            &self,
            data: &PlotData,
            config: &RenderConfig,
        ) -> anyhow::Result<Vec<u8>>; // PNG bytes for now
    }
    ```

  - Make the trait public and ensure `PlotData` / `RenderConfig` are also public and live in the domain.

- **Optional Future Evolution**
  - Consider later generalization (if needed) to support multiple output formats (e.g. PNG, SVG) with a `RenderedPlot` enum or an associated type. For now, keep it simple (`Vec<u8>`).

### 4. Axis Calculation & Edge Cases

- **Action**
  - Finalize `plot_domain::plot::axis::YAxis::from_data_points`:
    - It should compute sensible min/max, padding, and tick marks based on provided `DataPoint`s.
  - Define behaviour for **edge cases**:
    - **No data points**: e.g. decide to:
      - return a default “empty” axis + empty `PlotData`, or
      - surface a domain-level error that the application can propagate.
    - All values equal / very small range: ensure axis still renders meaningfully.

- **Tests**
  - Add unit tests in `plot-domain`:
    - empty data list,
    - strictly increasing / decreasing data,
    - all-equivalent values,
    - data with outliers.

---

## Phase 2: Application Logic (`lifestats-application`)

Goal: Provide a cohesive use case that turns statistics + logs + goals into a full `PlotData` object, independent of any rendering concerns.

### 5. Define the `GetPlotDataQuery`

- **Action**
  - In `core/application/src/use_cases/queries/get_plot_data/`:
    - Create a module `mod.rs` (or similar) with:

      ```rust
      pub struct GetPlotDataQuery {
          pub statistic_id: StatisticId,
          pub plot_type: PlotType,     // a domain enum, not a raw string
          pub date_range: DateRange,   // strongly-typed range
      }

      pub type GetPlotDataResult = PlotData;
      ```

    - Reuse domain types where possible (`StatisticId`, `PlotType`, `DateRange`).

- **Rationale**
  - Formalizes the contract for “asking the application for plot data”.
  - Keeps the application layer return type rich and domain-centric.

### 6. Implement `GetPlotDataQueryHandler`

- **Action**
  - Implement a handler struct (e.g. `GetPlotDataQueryHandler`) that depends on:
    - `StatisticRepository` (port),
    - any necessary clock / time service if date ranges need “today” or similar.

  - Handler steps:
    1. **Fetch Data**
       - Use `StatisticRepository` to load the `AnyStatistic` entity and all raw log and goal values (`serde_json::Value`) for the given `statistic_id` and `date_range`.
    2. **Transform Raw Values**
       - For each raw log/goal entry:
         - Use `AnyStatistic` domain methods to compute:
           - typed values,
           - goal achievement `Percentage` (if applicable).
         - Convert `serde_json::Value` into appropriate `DrawableData` / `DrawableGoal` enums.
           - Keep this conversion in the application layer (not in adapters).
    3. **Build Data Points**
       - Create `plot_domain::plot::data::DataPoint` instances from:
         - date,
         - computed values,
         - goal information,
         - any additional flags/labels.
    4. **Calculate Axis**
       - Call `YAxis::from_data_points(&data_points)` to compute the axis.
    5. **Assemble and Return**
       - Construct the final `PlotData` (title, type, data points, axis) and return it.

- **Edge Case Behaviour**
  - Decide and implement behaviour for:
    - No logs or goals in `date_range`.
    - Logs without goals, goals without logs.
    - Unsupported `PlotType` for the given `AnyStatistic` (surface a clear domain/application error).
  - Make these behaviours explicit in the handler’s tests.

- **Tests**
  - Add unit tests for `GetPlotDataQueryHandler`:
    - “Happy path”: logs + goals + valid range.
    - Empty range → empty plot or error (as decided).
    - Mismatched logs/goals.
    - Unsupported plot type for a statistic.

---

## Phase 3: Implement and Integrate Adapters

### 3a: Plotting Adapter (`lifestats-plotting`)

### 7. Implement `PlottersRenderer` Adapter

- **Action**
  - In the `lifestats-plotting` crate:
    - Implement a `PlottersRenderer` struct that holds any necessary configuration (e.g. default sizes, fonts).
    - Implement `plot_domain::plot::renderer::PlotRenderer` for `PlottersRenderer`:
      - Adapt existing `plotters` logic to take `&PlotData` + `&RenderConfig`.
      - Render to a PNG buffer (`Vec<u8>`).

- **Dependency Direction**
  - Ensure crate dependencies are one-way:
    - `lifestats-plotting` depends on `plot-domain`.
    - `plot-domain` **must not** depend on `lifestats-plotting`.

### 8. Add Golden Tests for Rendering

- **Action**
  - Add tests in `lifestats-plotting`:
    - Use a small, deterministic `PlotData` to render a plot.
    - Compare the output (or a hash of it) against a checked-in “golden” baseline.
  - Consider minor tolerances if the underlying library might change pixel-perfect output slightly.

- **Rationale**
  - Provides a strong safety net against accidental visual regressions.
  - Makes renderer refactors safer.

---

### 3b: Web Adapter (`lifestats-web-api`)

### 9. Update `AppState` for Renderer Injection

- **Action**
  - In `lifestats-web-api/src/state.rs`:
    - Remove `plot_repository: Arc<P>` from `AppState`.
    - Add `renderer: Arc<Rend>` where `Rend: PlotRenderer + 'static`.
    - Update `AppState::new` and all call sites (e.g. `main.rs`, tests) to:
      - Construct and inject a concrete `PlottersRenderer` instance at startup.

- **Rationale**
  - Applies dependency injection at the web layer.
  - Makes it easy to swap out the renderer for testing or future alternatives.

### 10. Implement Stateless Plot Endpoints

- **Action**
  - In `lifestats-web-api/src/routes/plots.rs` and `routes/mod.rs`:
    - Add and register:
      - `GET /statistics/:id/plot/image`
      - `GET /statistics/:id/plot/data`

- **Request Parameters**
  - Define a typed parameter struct for query/path parameters, e.g.:

    ```rust
    pub struct PlotRequestParams {
        pub statistic_id: StatisticId,
        pub plot_type: PlotType,      // parsed from query as a string enum
        pub date_from: Date,         // from query
        pub date_to: Date,           // from query
    }
    ```

  - Perform input validation:
    - Invalid `plot_type` → 400 Bad Request.
    - `date_from > date_to` → 400 Bad Request.
    - Enforce a maximum allowed date range window if needed (to avoid huge responses).

- **Guideline**
  - Prefer using Axum's `Path<...>` and `Query<...>` extractors directly in handler signatures to parse request parameters, rather than creating an intermediate PlotRequestParams struct, to leverage the framework's built-in validation and reduce boilerplate.

### 11. Implement `/image` Handler Logic

- **Action**
  - `GET /statistics/:id/plot/image` handler steps:
    1. Parse and validate inputs into `PlotRequestParams`.
    2. Map to `GetPlotDataQuery`.
    3. Execute `GetPlotDataQuery` via the application service.
    4. Call `renderer.render(&plot_data, &render_config)` to get `Vec<u8>`.
    5. Return the bytes with `Content-Type: image/png`.

- **Error Handling**
  - Map errors to HTTP responses:
    - Not found statistic → 404.
    - Validation / unsupported plot type → 400.
    - Rendering/internal errors → 500 (with internal logging).

### 12. Implement `/data` Handler Logic

- **Action**
  - `GET /statistics/:id/plot/data` handler steps:
    1. Parse and validate inputs (same as `/image`).
    2. Execute `GetPlotDataQuery` to get `PlotData`.
    3. Map `PlotData` into a dedicated DTO `PlotDataResponse`:
       - Use simple API-friendly shapes (e.g. arrays of primitives, simple enums).
       - Avoid exposing internal-only fields or domain-only concepts directly.
    4. Return JSON response with appropriate status code.

- **Rationale**
  - DTO layer decouples API shape from internal `PlotData`.
  - Makes `PlotData` refactors possible without breaking clients.

### 13. API Integration Tests

- **Action**
  - Add integration tests in `lifestats-web-api` for:
    - Successful `/image` call (e.g. returns 200 with non-empty PNG bytes).
    - Successful `/data` call (checks JSON structure and some values).
    - Invalid parameters (bad plot_type, invalid date range) → 400.
    - Non-existent statistic → 404.

---

## Phase 4: Cleanup, Deprecation & Migration

Goal: Remove the old stateful plotting model and align persistence and API with the new stateless design.

### 14. Deprecate and Remove `PlotSpecification` & `PlotRepository`

- **Action**
  - Delete:
    - `core/plot-domain/src/plot/specification.rs`.
    - `core/plot-domain/src/plot/repository.rs`.
    - `SqlitePlotRepository` implementation in `core/infrastructure/persistence`.
  - Remove any references from `AppState`, services, or tests.

- **Optional Transitional Step**
  - Before removal, mark these types as deprecated (Rust `#[deprecated]`) and ensure no remaining usages.

### 15. Drop the `plots` Table

- **Action**
  - Add a new database migration:
    - Drop the `plots` table and any related indices/constraints.
  - Confirm there is no remaining code that references this table.

- **Rationale**
  - Aligns the database schema with the stateless plotting model.
  - Simplifies persistence and reduces cognitive load.

### 16. Remove Old Plot API Routes

- **Action**
  - In `lifestats-web-api/src/routes/plots.rs` and `routes/mod.rs`:
    - Remove old handlers and routes (e.g. `/plots/:id/image` or similar).
  - Ensure there are no stray references in OpenAPI docs or API clients.

- **Optional Deprecation Strategy**
  - If external clients exist:
    - Temporarily keep old routes returning 301/307 redirects or deprecation warnings.
    - Log usage to gauge when it’s safe to fully remove.

### 17. Update Tests to Use New Endpoints

- **Action**
  - Find and update any E2E or integration tests that used old plot endpoints:
    - Migrate them to use `GET /statistics/:id/plot/image`.
    - Migrate them to use `GET /statistics/:id/plot/data`.

---

## Phase 5: Cross-Cutting Concerns & Documentation

### 18. Error Mapping and Logging

- **Action**
  - Standardize how errors travel from:
    - `PlotRenderer` → application → web API.
  - Ensure:
    - Client-visible responses are clean and not leaking internal details.
    - Detailed error context is logged at the appropriate level (e.g., warn/error).

### 19. Performance & Observability

- **Action**
  - Measure:
    - Latency for typical `/image` and `/data` requests for realistic date ranges.
  - If needed:
    - Plan caching strategies (HTTP caching, application-level memoization) in a separate iteration.
  - Add basic metrics:
    - Count of plot requests, error rates, average render times.

### 20. Documentation & Changelog

- **Action**
  - Update architecture docs (e.g. `docs/ARCHITECTURE.md` or equivalent):
    - Describe the stateless plotting flow and `PlotRenderer` port.
    - Document the new endpoints and expected inputs/outputs.
  - Update API docs / OpenAPI spec (if you have one) for:
    - `/statistics/:id/plot/image`
    - `/statistics/:id/plot/data`
  - Add a changelog entry:
    - Note removal of stored plot configurations and the `plots` table.
    - Announce the new stateless plot API routes.

---

## Phase 6: Cleanup Checklist

Before closing this refactor, verify:

- [ ] No remaining references to `PlotSpecification` / `PlotRepository` / `SqlitePlotRepository`.
- [ ] No code references the `plots` table.
- [ ] All plot-related tests pass and target the new endpoints and model.
- [ ] `plot-domain` does not depend on `lifestats-plotting` or other infrastructure crates.
- [ ] Web API composes `PlotRenderer` only via the port (trait), not via concrete types.
- [ ] Documentation and API specs are up to date.
