use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::use_cases::logging_service::LogValueCommand;
use lifestats_domain::{
    shared::ids::StatisticId, statistic::ports::statistic_repository::StatisticRepository,
};
use validator::Validate;

use crate::{
    dto::statistic::{LogValueRequest, StatisticMetadataResponse},
    error::ApiError,
    state::AppState,
};

/// GET /statistics - Get all statistics metadata
pub async fn get_all_statistics<R, E>(
    State(app_state): State<AppState<R, E>>,
) -> Result<Json<Vec<StatisticMetadataResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    let statistics = app_state.query_service.get_all_statistics()?;
    let response: Vec<StatisticMetadataResponse> =
        statistics.iter().map(|stat| stat.into()).collect();
    Ok(Json(response))
}

/// POST /statistics/:id/log/:date - Log a value for a statistic on a specific date
pub async fn log_value<R, E>(
    State(app_state): State<AppState<R, E>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<LogValueRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    // Validate the request payload
    payload.validate()?;

    // Create the command
    let command = LogValueCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        value_str: payload.value,
        update_policy_str: payload.update_policy,
    };

    // Execute the command
    app_state.logging_service.log_value(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}
