use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::use_cases::logging_service::LogValueCommand;
use lifestats_domain::{
    plot::renderer::PlotRenderer, plot::repository::PlotRepository, shared::ids::StatisticId,
    statistic::ports::statistic_repository::StatisticRepository,
};
use validator::Validate;

use crate::{
    dto::statistic::{LogTimeFrameRequest, LogValueRequest, StatisticMetadataResponse},
    error::ApiError,
    state::AppState,
};

/// GET /statistics - Get all statistics metadata
pub async fn get_all_statistics<R, P, Rend, E>(
    State(app_state): State<AppState<R, P, Rend, E>>,
) -> Result<Json<Vec<StatisticMetadataResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    let statistics = app_state.query_service.get_all_statistics()?;
    let response: Vec<StatisticMetadataResponse> =
        statistics.iter().map(|stat| stat.into()).collect();
    Ok(Json(response))
}

/// POST /statistics/:id/log/:date - Log a value for a statistic on a specific date
pub async fn log_value<R, P, Rend, E>(
    State(app_state): State<AppState<R, P, Rend, E>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<LogValueRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
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

/// POST /statistics/:id/log_time_frame/:date - Log a time frame for a TimeSpans statistic
pub async fn log_time_frame<R, P, Rend, E>(
    State(app_state): State<AppState<R, P, Rend, E>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<LogTimeFrameRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    use chrono::NaiveTime;
    use lifestats_application::use_cases::logging_service::LogTimeFrameCommand;
    use lifestats_domain::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;
    use std::str::FromStr;

    // Validate the request payload
    payload.validate()?;

    // Parse time strings into NaiveTime
    let start_time = NaiveTime::parse_from_str(&payload.start_time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(&payload.start_time, "%H:%M"))
        .map_err(|e| ApiError::BadRequest(format!("Invalid start_time format: {}", e)))?;

    let end_time = NaiveTime::parse_from_str(&payload.end_time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(&payload.end_time, "%H:%M"))
        .map_err(|e| ApiError::BadRequest(format!("Invalid end_time format: {}", e)))?;

    // Parse update policy if provided
    let update_policy = payload
        .update_policy
        .as_deref()
        .map(|policy_str| {
            TimeFrameListUpdatePolicy::from_str(policy_str)
                .map_err(|_| ApiError::BadRequest(format!("Invalid update_policy: {}", policy_str)))
        })
        .transpose()?;

    // Create the command
    let command = LogTimeFrameCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        start_time,
        end_time,
        update_policy,
    };

    // Execute the command
    app_state.logging_service.log_time_frame(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}
