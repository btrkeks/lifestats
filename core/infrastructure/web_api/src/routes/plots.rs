use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::PlotId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;
use serde::Deserialize;

use crate::{dto::plot::PlotDataResponse, error::ApiError, state::AppState};
use events::event_port::EventPublisher;
use plot_domain::plot::renderer::RenderConfig;

#[derive(Debug, Deserialize)]
pub struct PlotImageQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// GET /plots/:id/image
/// Returns a rendered plot image as PNG
pub async fn get_plot_image<R, P, Rend, E, I>(
    State(state): State<AppState<R, P, Rend, E, I>>,
    Path(id): Path<u32>,
    Query(params): Query<PlotImageQuery>,
) -> Result<impl IntoResponse, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use crate::helpers;
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_plot_data::{
        GetPlotDataQuery, GetPlotDataQueryHandler,
    };
    use plot_domain::plot::axis::YAxis;
    use plot_domain::plot::data::{DataPoint, PlotData};

    let plot_id = PlotId(id);

    // Parse date range or use defaults
    let start = params
        .start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            // Default to last 14 days
            chrono::Local::now().date_naive() - chrono::Duration::days(14)
        });

    let end = params
        .end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let date_range = DateRange::new(start, end)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;

    // Create render config from query params or use defaults
    let config = RenderConfig {
        width: params.width.unwrap_or(800),
        height: params.height.unwrap_or(600),
    };

    // 1. Get plot specification to determine plot type
    let plot_spec = state
        .plot_repository
        .find_by_id(plot_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to load plot spec: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Plot {} not found", plot_id.0)))?;

    // 2. Execute GetPlotDataQuery for pure data
    let query = GetPlotDataQuery {
        plot_id,
        date_range,
    };

    let handler = GetPlotDataQueryHandler::new(
        state.statistic_repository.clone(),
        state.plot_repository.clone(),
    );

    let plot_data_result = handler
        .handle(query)
        .map_err(|e| ApiError::InternalError(format!("Failed to get plot data: {}", e)))?;

    // 3. Load statistic metadata for transformations
    let statistic = state
        .statistic_repository
        .find_metadata_by_id(plot_spec.statistic_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to load statistic: {}", e)))?;

    // 4. Transform PlotPointDto to DataPoint (apply presentation logic)
    let data_points: Vec<DataPoint> = plot_data_result
        .points
        .iter()
        .filter_map(|point| {
            // Convert value to DrawableData
            let drawable_value =
                helpers::json_to_drawable_data(&statistic, &point.value, plot_spec.plot_type)
                    .ok()?;

            // Convert percentage to color
            let color = helpers::percentage_to_color(point.percentage_achieved);

            // Convert goal to DrawableGoal (if present)
            let drawable_goal = point
                .goal
                .as_ref()
                .and_then(|g| helpers::json_to_drawable_goal(&statistic, g).ok());

            Some(DataPoint::new(
                point.date,
                drawable_value,
                color,
                drawable_goal,
            ))
        })
        .collect();

    // 5. Calculate Y-axis from transformed data
    let y_axis = YAxis::from_data_points(&data_points, plot_spec.plot_type)
        .map_err(|e| ApiError::InternalError(format!("Failed to calculate Y-axis: {}", e)))?;

    // 6. Build PlotData
    let plot_data = PlotData::new(
        plot_spec.title.clone(),
        data_points,
        date_range,
        plot_spec.plot_type,
        y_axis,
    );

    // 7. Render (renderer is an infrastructure component, safe for adapter to depend on)
    let png_bytes = state
        .renderer
        .render(&plot_data, &config)
        .map_err(|e| ApiError::InternalError(format!("Failed to render plot: {}", e)))?;

    // Return the PNG image with appropriate headers
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        png_bytes,
    ))
}

/// GET /plots/:id/data
/// Returns plot data as JSON (data points, axis config, etc.)
pub async fn get_plot_data<R, P, Rend, E, I>(
    State(state): State<AppState<R, P, Rend, E, I>>,
    Path(id): Path<u32>,
    Query(params): Query<PlotImageQuery>,
) -> Result<Json<PlotDataResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use crate::dto::plot::DataPointResponse;
    use crate::helpers;
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_plot_data::{
        GetPlotDataQuery, GetPlotDataQueryHandler,
    };
    use plot_domain::plot::axis::YAxis;
    use plot_domain::plot::data::DataPoint;

    let plot_id = PlotId(id);

    // Parse date range or use defaults
    let start = params
        .start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            // Default to last 14 days
            chrono::Local::now().date_naive() - chrono::Duration::days(14)
        });

    let end = params
        .end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let date_range = DateRange::new(start, end)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;

    // 1. Get plot specification to determine plot type
    let plot_spec = state
        .plot_repository
        .find_by_id(plot_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to load plot spec: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Plot {} not found", plot_id.0)))?;

    // 2. Create and execute GetPlotDataQuery for pure data
    let query = GetPlotDataQuery {
        plot_id,
        date_range,
    };

    let handler = GetPlotDataQueryHandler::new(
        state.statistic_repository.clone(),
        state.plot_repository.clone(),
    );

    let plot_data_result = handler
        .handle(query)
        .map_err(|e| ApiError::InternalError(format!("Failed to get plot data: {}", e)))?;

    // 3. Load statistic metadata for transformations
    let statistic = state
        .statistic_repository
        .find_metadata_by_id(plot_spec.statistic_id)
        .map_err(|e| ApiError::InternalError(format!("Failed to load statistic: {}", e)))?;

    // 4. Transform PlotPointDto to DataPoint (apply presentation logic)
    let data_points: Vec<DataPoint> = plot_data_result
        .points
        .iter()
        .filter_map(|point| {
            // Convert value to DrawableData
            let drawable_value =
                helpers::json_to_drawable_data(&statistic, &point.value, plot_spec.plot_type)
                    .ok()?;

            // Convert percentage to color
            let color = helpers::percentage_to_color(point.percentage_achieved);

            // Convert goal to DrawableGoal (if present)
            let drawable_goal = point
                .goal
                .as_ref()
                .and_then(|g| helpers::json_to_drawable_goal(&statistic, g).ok());

            Some(DataPoint::new(
                point.date,
                drawable_value,
                color,
                drawable_goal,
            ))
        })
        .collect();

    // 5. Calculate Y-axis from the transformed data points
    let y_axis = YAxis::from_data_points(&data_points, plot_spec.plot_type)
        .map_err(|e| ApiError::InternalError(format!("Failed to calculate Y-axis: {}", e)))?;

    // 6. Build the response DTO
    let response = PlotDataResponse {
        title: plot_data_result.title,
        points: data_points
            .into_iter()
            .map(DataPointResponse::from)
            .collect(),
        date_range: plot_data_result.date_range.into(),
        plot_type: plot_spec.plot_type,
        y_axis: y_axis.into(),
    };

    Ok(Json(response))
}
