//! HTTP route handlers for heartbeat session management.
//!
//! Provides REST endpoints to start, interact with, and monitor active
//! heartbeat sessions for individual statistics.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::{renderer::PlotRenderer, repository::PlotRepository};
use validator::Validate;

use crate::{
    dto::session::{SessionStatusResponse, StartSessionRequest},
    error::ApiError,
    state::AppState,
};

/// POST /sessions/:statistic_id/start - Start a new heartbeat session
///
/// Starts a new heartbeat session for the specified statistic.
/// Only one active session per statistic is allowed.
///
/// Returns the initial session status with elapsed time 0.
pub async fn start_session<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Json(payload): Json<StartSessionRequest>,
) -> Result<(StatusCode, Json<SessionStatusResponse>), ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // Validate the request payload
    payload.validate()?;

    let stat_id = StatisticId(statistic_id);

    // Build HeartbeatConfig from request (use defaults for None values)
    let config =
        if payload.idle_timeout_seconds.is_some() || payload.flush_interval_seconds.is_some() {
            let idle_timeout = payload.idle_timeout_seconds.unwrap_or(60).clamp(1, 86400); // Clamp between 1s and 1 day

            let flush_interval = payload.flush_interval_seconds.unwrap_or(60).clamp(1, 86400);

            Some(lifestats_application::HeartbeatConfig {
                idle_timeout: chrono::Duration::seconds(idle_timeout),
                flush_interval: chrono::Duration::seconds(flush_interval),
            })
        } else {
            None
        };

    let status = app_state
        .session_manager
        .start_session(stat_id, config)
        .map_err(ApiError::from)?;

    let response = SessionStatusResponse {
        statistic_id: status.statistic_id.0,
        status: if status.is_running {
            "running".to_string()
        } else {
            "stopped".to_string()
        },
        is_running: status.is_running,
        total_elapsed_seconds: status.total_elapsed.num_seconds(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /sessions/:statistic_id/beat - Send a heartbeat
///
/// Records a heartbeat for an active session, extending the idle timeout.
/// Creates the session on first call if it doesn't exist.
pub async fn send_heartbeat<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let stat_id = StatisticId(statistic_id);

    app_state
        .session_manager
        .beat(stat_id)
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /sessions/:statistic_id/stop - Stop a session
///
/// Stops an active session and finalizes any pending time frames.
pub async fn stop_session<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let stat_id = StatisticId(statistic_id);

    app_state
        .session_manager
        .stop_session(stat_id)
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /sessions/:statistic_id - Get session status
///
/// Returns the current status of a session, including whether it's running
/// and total elapsed time.
pub async fn get_session_status<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<Json<SessionStatusResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let stat_id = StatisticId(statistic_id);

    let status = app_state
        .session_manager
        .get_status(stat_id)
        .map_err(ApiError::from)?;

    let response = SessionStatusResponse {
        statistic_id: status.statistic_id.0,
        status: if status.is_running {
            "running".to_string()
        } else {
            "stopped".to_string()
        },
        is_running: status.is_running,
        total_elapsed_seconds: status.total_elapsed.num_seconds(),
    };

    Ok(Json(response))
}

/// GET /sessions - List all active sessions
///
/// Returns a list of all currently running heartbeat sessions.
pub async fn list_sessions<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<Vec<SessionStatusResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let sessions = app_state.session_manager.list_active_sessions();

    let responses: Vec<SessionStatusResponse> = sessions
        .into_iter()
        .map(|status| SessionStatusResponse {
            statistic_id: status.statistic_id.0,
            status: if status.is_running {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
            is_running: status.is_running,
            total_elapsed_seconds: status.total_elapsed.num_seconds(),
        })
        .collect();

    Ok(Json(responses))
}
