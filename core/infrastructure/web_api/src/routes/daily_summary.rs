use axum::{
    extract::{Path, State},
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::{
    cqrs::QueryHandler,
    dto::summary::DailySummaryResponse,
    use_cases::{
        integration::IntegrationServiceTrait, queries::GetDailySummaryQuery,
        queries::GetDailySummaryQueryHandler,
    },
};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;

use crate::{error::ApiError, state::AppState};

/// GET /daily_summary/:date - Get daily summary for a specific date
pub async fn get_daily_summary<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(date): Path<NaiveDate>,
) -> Result<Json<DailySummaryResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = GetDailySummaryQueryHandler::new(app_state.statistic_repository.clone());
    let daily_summary = handler
        .handle(GetDailySummaryQuery { date })
        .map_err(|e| ApiError::InternalError(format!("Failed to get daily summary: {}", e)))?;

    Ok(Json(daily_summary))
}
