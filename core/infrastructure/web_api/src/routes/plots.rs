use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use chrono::NaiveDate;
use lifestats_domain::plot::renderer::PlotRenderer;
use lifestats_domain::plot::repository::PlotRepository;
use lifestats_domain::shared::date_range::DateRange;
use lifestats_domain::shared::ids::PlotId;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use serde::Deserialize;

use crate::{error::ApiError, state::AppState};
use events::event_port::EventPublisher;
use lifestats_domain::plot::renderer::RenderConfig;

#[derive(Debug, Deserialize)]
pub struct PlotImageQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// GET /plots/:id/image
/// Returns a rendered plot image as PNG
pub async fn get_plot_image<R, P, Rend, E>(
    State(state): State<AppState<R, P, Rend, E>>,
    Path(id): Path<u32>,
    Query(params): Query<PlotImageQuery>,
) -> Result<impl IntoResponse, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
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

    // Generate the plot image
    let png_bytes = state
        .plotting_service
        .generate_plot_image(plot_id, date_range, config)
        .map_err(|e| ApiError::InternalError(format!("Failed to generate plot: {}", e)))?;

    // Return the PNG image with appropriate headers
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        png_bytes,
    ))
}
