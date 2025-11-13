use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::cqrs::QueryHandler;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_application::use_cases::queries::get_plot_data::{
    GetPlotDataQuery, GetPlotDataQueryHandler,
};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::{PlotRenderer, RenderConfig};
use plot_domain::plot::specification::PlotType;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{error::ApiError, state::AppState};

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlotImageQuery {
    pub plot_type: String,
    pub date_from: String,
    pub date_to: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PlotDataQuery {
    pub plot_type: String,
    pub date_from: String,
    pub date_to: String,
}

#[derive(Debug, Serialize)]
pub struct PlotDataResponse {
    pub title: String,
    pub plot_type: String,
    pub date_range: DateRangeDto,
    pub points: Vec<DataPointDto>,
    pub y_axis: YAxisDto,
}

#[derive(Debug, Serialize)]
pub struct DateRangeDto {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct DataPointDto {
    pub date: String,
    pub value: String, // Serialized DrawableData
    pub color: [u8; 3],
    pub goal: Option<String>, // Serialized DrawableGoal
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum YAxisDto {
    Integer { min: i64, max: i64 },
    Time { min: String, max: String },
}

// ============================================================================
// New Stateless Plot Routes
// ============================================================================

/// GET /statistics/:id/plot/image
/// Generates and returns a plot image on-the-fly without stored configuration
pub async fn get_statistic_plot_image<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Query(params): Query<PlotImageQuery>,
) -> Result<impl IntoResponse, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // 1. Parse and validate inputs
    let statistic_id = lifestats_core::shared::ids::StatisticId(statistic_id);

    let plot_type = parse_plot_type(&params.plot_type)?;

    let date_from = NaiveDate::parse_from_str(&params.date_from, "%Y-%m-%d").map_err(|e| {
        ApiError::BadRequest(format!(
            "Invalid date_from format: {}. Expected YYYY-MM-DD",
            e
        ))
    })?;

    let date_to = NaiveDate::parse_from_str(&params.date_to, "%Y-%m-%d").map_err(|e| {
        ApiError::BadRequest(format!(
            "Invalid date_to format: {}. Expected YYYY-MM-DD",
            e
        ))
    })?;

    let date_range = DateRange::new(date_from, date_to)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;

    // 2. Build and execute GetPlotDataQuery
    let query = GetPlotDataQuery {
        statistic_id,
        plot_type,
        date_range,
    };

    let handler = GetPlotDataQueryHandler::new(state.statistic_repository.clone());
    let plot_data = handler.handle(query).map_err(|e| {
        if e.to_string().contains("not found") {
            ApiError::NotFound(format!("Statistic {} not found", statistic_id.0))
        } else {
            ApiError::InternalError(format!("Failed to generate plot data: {}", e))
        }
    })?;

    // 3. Build RenderConfig
    let width = params.width.unwrap_or(800);
    let height = params.height.unwrap_or(600);
    let render_config = RenderConfig { width, height };

    // 4. Render the plot
    let png_bytes = state
        .renderer
        .render(&plot_data, &render_config)
        .map_err(|e| ApiError::InternalError(format!("Failed to render plot: {}", e)))?;

    // 5. Return PNG image
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        png_bytes,
    ))
}

/// GET /statistics/:id/plot/data
/// Returns plot data as JSON without rendering an image
pub async fn get_statistic_plot_data<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Query(params): Query<PlotDataQuery>,
) -> Result<Json<PlotDataResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // 1. Parse and validate inputs
    let statistic_id = lifestats_core::shared::ids::StatisticId(statistic_id);

    let plot_type = parse_plot_type(&params.plot_type)?;

    let date_from = NaiveDate::parse_from_str(&params.date_from, "%Y-%m-%d").map_err(|e| {
        ApiError::BadRequest(format!(
            "Invalid date_from format: {}. Expected YYYY-MM-DD",
            e
        ))
    })?;

    let date_to = NaiveDate::parse_from_str(&params.date_to, "%Y-%m-%d").map_err(|e| {
        ApiError::BadRequest(format!(
            "Invalid date_to format: {}. Expected YYYY-MM-DD",
            e
        ))
    })?;

    let date_range = DateRange::new(date_from, date_to)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;

    // 2. Build and execute GetPlotDataQuery
    let query = GetPlotDataQuery {
        statistic_id,
        plot_type,
        date_range,
    };

    let handler = GetPlotDataQueryHandler::new(state.statistic_repository.clone());
    let plot_data = handler.handle(query).map_err(|e| {
        if e.to_string().contains("not found") {
            ApiError::NotFound(format!("Statistic {} not found", statistic_id.0))
        } else {
            ApiError::InternalError(format!("Failed to generate plot data: {}", e))
        }
    })?;

    // 3. Convert PlotData to DTO
    let response = plot_data_to_dto(plot_data);

    Ok(Json(response))
}

/// GET /plots/timeline/:date
/// Returns a rendered timeline plot for a specific date showing all timeline events
pub async fn get_daily_timeline_plot<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(date_str): Path<String>,
) -> Result<impl IntoResponse, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::queries::get_daily_timeline_plot::{
        GetDailyTimelinePlotQuery, GetDailyTimelinePlotQueryHandler,
    };

    // Parse the date from path parameter
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
        ApiError::BadRequest(format!("Invalid date format: {}. Expected YYYY-MM-DD", e))
    })?;

    // Instantiate handler with repository dependency
    let handler = GetDailyTimelinePlotQueryHandler::new(state.statistic_repository.clone());

    // Execute the query
    let query = GetDailyTimelinePlotQuery { date };
    let png_bytes = handler
        .handle(query)
        .map_err(|e| ApiError::InternalError(format!("Failed to generate timeline plot: {}", e)))?;

    // Return the PNG image with appropriate headers
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        png_bytes,
    ))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn parse_plot_type(plot_type_str: &str) -> Result<PlotType, ApiError> {
    match plot_type_str.to_lowercase().as_str() {
        "bar_chart" | "bar" => Ok(PlotType::BarChart),
        "line_chart" | "line" => Ok(PlotType::LineChart),
        "scatter" => Ok(PlotType::Scatter),
        "time_frames" | "timeframes" => Ok(PlotType::TimeFrames),
        "circular_progress" | "circular" => Ok(PlotType::CircularProgress),
        _ => Err(ApiError::BadRequest(format!(
            "Invalid plot_type '{}'. Valid values: bar_chart, line_chart, scatter, time_frames, circular_progress",
            plot_type_str
        ))),
    }
}

fn plot_data_to_dto(plot_data: plot_domain::plot::data::PlotData) -> PlotDataResponse {
    use plot_domain::plot::axis::YAxis;
    use plot_domain::plot::data::{DrawableData, DrawableGoal};

    let points: Vec<DataPointDto> = plot_data
        .points
        .into_iter()
        .map(|point| {
            let value_str = match point.value {
                DrawableData::Integer(i) => format!("{}", i),
                DrawableData::TimeFrames(tf) => {
                    serde_json::to_string(&tf).unwrap_or_else(|_| "[]".to_string())
                }
            };

            let goal_str = point.goal.map(|g| match g {
                DrawableGoal::Integer(i) => format!("{}", i),
                DrawableGoal::TimeFrames(tf) => {
                    serde_json::to_string(&tf).unwrap_or_else(|_| "[]".to_string())
                }
            });

            DataPointDto {
                date: point.date.format("%Y-%m-%d").to_string(),
                value: value_str,
                color: point.color,
                goal: goal_str,
            }
        })
        .collect();

    let y_axis_dto = match plot_data.y_axis {
        YAxis::Integer { min, max } => YAxisDto::Integer { min, max },
        YAxis::Time { min, max } => YAxisDto::Time {
            min: min.format("%H:%M:%S").to_string(),
            max: max.format("%H:%M:%S").to_string(),
        },
    };

    let plot_type_str = match plot_data.plot_type {
        PlotType::BarChart => "bar_chart",
        PlotType::LineChart => "line_chart",
        PlotType::Scatter => "scatter",
        PlotType::TimeFrames => "time_frames",
        PlotType::CircularProgress => "circular_progress",
    };

    PlotDataResponse {
        title: plot_data.title,
        plot_type: plot_type_str.to_string(),
        date_range: DateRangeDto {
            start: plot_data.date_range.start().format("%Y-%m-%d").to_string(),
            end: plot_data.date_range.end().format("%Y-%m-%d").to_string(),
        },
        points,
        y_axis: y_axis_dto,
    }
}
