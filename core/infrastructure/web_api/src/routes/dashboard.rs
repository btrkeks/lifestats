//! Dashboard route handler.

use std::sync::Arc;

use axum::{extract::State, Json};
use events::event_port::EventPublisher;
use lifestats_application::cqrs::QueryHandler;
use lifestats_application::dto::DashboardResponseDto;
use lifestats_application::services::{DefaultDailyAchievementEvaluator, GoalPerformanceAnalyzer};
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_application::use_cases::queries::{GetDashboardQuery, GetDashboardQueryHandler};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use validator::Validate;

use crate::dto::dashboard::DashboardRequest;
use crate::error::ApiError;
use crate::state::AppState;

/// POST /statistics/dashboard - Get dashboard data for multiple statistics.
///
/// Returns today's value, 7-day average, and performance metrics for each statistic.
pub async fn get_dashboard<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Json(payload): Json<DashboardRequest>,
) -> Result<Json<DashboardResponseDto>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    payload.validate()?;

    let statistic_ids: Vec<StatisticId> = payload
        .statistic_ids
        .iter()
        .map(|&id| StatisticId(id))
        .collect();

    let evaluator = Arc::new(DefaultDailyAchievementEvaluator::new(
        app_state.statistic_repository.clone(),
    ));

    let analyzer = Arc::new(GoalPerformanceAnalyzer::new(
        evaluator,
        app_state.statistic_repository.clone(),
    ));

    let handler = GetDashboardQueryHandler::new(app_state.statistic_repository.clone(), analyzer);

    let query = GetDashboardQuery {
        statistic_ids,
        today: payload.today,
        performance_window_days: payload.performance_window_days,
    };

    let result = handler.handle(query).map_err(ApiError::from)?;

    Ok(Json(result))
}
