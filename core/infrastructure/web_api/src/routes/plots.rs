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
use plot_domain::plot::axis::AxisLabelFormat;
use plot_domain::plot::data::{ScalarUnit, ScalarValue};
use plot_domain::plot::plot_type::PlotType;
use plot_domain::plot::renderer::{PlotRenderer, RenderConfig};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PlotImageQuery {
    pub plot_type: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
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
    pub value: Option<DrawableValueDto>,
    pub color: [u8; 3],
    pub goal: Option<DrawableValueDto>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum YAxisDto {
    Scalar {
        min: i64,
        max: i64,
        label_format: String,
    },
    TimeOfDay {
        min_seconds: i64,
        max_seconds: i64,
        label_format: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum DrawableValueDto {
    Scalar(ScalarValueDto),
    TimeFrames(serde_json::Value),
}

#[derive(Debug, Serialize)]
pub struct ScalarValueDto {
    pub value: i64,
    pub unit: String,
}

// ============================================================================
// New Stateless Plot Routes
// ============================================================================

/// GET /statistics/{id}/plot/image
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

    let date_to = if let Some(ref dt) = params.date_to {
        NaiveDate::parse_from_str(dt, "%Y-%m-%d").map_err(|e| {
            ApiError::BadRequest(format!(
                "Invalid date_to format: {}. Expected YYYY-MM-DD",
                e
            ))
        })?
    } else {
        chrono::Local::now().date_naive()
    };

    let date_from = if let Some(ref df) = params.date_from {
        NaiveDate::parse_from_str(df, "%Y-%m-%d").map_err(|e| {
            ApiError::BadRequest(format!(
                "Invalid date_from format: {}. Expected YYYY-MM-DD",
                e
            ))
        })?
    } else {
        date_to - chrono::Duration::days(14)
    };

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
            let value = point.value.map(|value| match value {
                DrawableData::Scalar(value) => DrawableValueDto::Scalar(value.into()),
                DrawableData::TimeFrames(tf) => DrawableValueDto::TimeFrames(
                    serde_json::to_value(tf).unwrap_or_else(|_| serde_json::json!([])),
                ),
            });

            let goal = point.goal.map(|goal| match goal {
                DrawableGoal::Scalar(value) => DrawableValueDto::Scalar(value.into()),
                DrawableGoal::TimeFrames(tf) => DrawableValueDto::TimeFrames(
                    serde_json::to_value(tf).unwrap_or_else(|_| serde_json::json!([])),
                ),
            });

            DataPointDto {
                date: point.date.format("%Y-%m-%d").to_string(),
                value,
                color: point.color,
                goal,
            }
        })
        .collect();

    let y_axis_dto = match plot_data.y_axis {
        YAxis::Scalar {
            min,
            max,
            label_format,
        } => YAxisDto::Scalar {
            min,
            max,
            label_format: axis_label_format_to_string(label_format).to_string(),
        },
        YAxis::TimeOfDay {
            min_seconds,
            max_seconds,
            label_format,
        } => YAxisDto::TimeOfDay {
            min_seconds,
            max_seconds,
            label_format: axis_label_format_to_string(label_format).to_string(),
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

impl From<ScalarValue> for ScalarValueDto {
    fn from(value: ScalarValue) -> Self {
        Self {
            value: value.value(),
            unit: scalar_unit_to_string(value.unit()).to_string(),
        }
    }
}

fn scalar_unit_to_string(unit: ScalarUnit) -> &'static str {
    match unit {
        ScalarUnit::Number => "number",
        ScalarUnit::Binary => "binary",
        ScalarUnit::DurationSeconds => "duration_seconds",
        ScalarUnit::TimeOfDaySeconds => "time_of_day_seconds",
        ScalarUnit::Percent => "percent",
    }
}

fn axis_label_format_to_string(format: AxisLabelFormat) -> &'static str {
    match format {
        AxisLabelFormat::Number => "number",
        AxisLabelFormat::DurationHhMm => "duration_hh_mm",
        AxisLabelFormat::TimeOfDayHhMm => "time_of_day_hh_mm",
        AxisLabelFormat::Percent => "percent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plot_domain::plot::axis::YAxis;
    use plot_domain::plot::data::{DataPoint, DrawableData, DrawableGoal, PlotData};

    #[test]
    fn plot_data_dto_exposes_scalar_units_and_axis_label_format() {
        let start = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let date_range = DateRange::new(start, start).unwrap();
        let plot_data = PlotData::new(
            "Deep Work".to_string(),
            vec![DataPoint::new(
                start,
                Some(DrawableData::Scalar(ScalarValue::duration_seconds(7200))),
                [169, 252, 177],
                Some(DrawableGoal::Scalar(ScalarValue::duration_seconds(3600))),
            )],
            date_range,
            PlotType::BarChart,
            YAxis::Scalar {
                min: 0,
                max: 7560,
                label_format: AxisLabelFormat::DurationHhMm,
            },
        );

        let response = plot_data_to_dto(plot_data);
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["points"][0]["value"]["type"], "Scalar");
        assert_eq!(json["points"][0]["value"]["value"]["value"], 7200);
        assert_eq!(
            json["points"][0]["value"]["value"]["unit"],
            "duration_seconds"
        );
        assert_eq!(json["points"][0]["goal"]["value"]["value"], 3600);
        assert_eq!(json["y_axis"]["type"], "Scalar");
        assert_eq!(json["y_axis"]["label_format"], "duration_hh_mm");
    }
}
