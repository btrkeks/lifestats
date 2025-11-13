use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::{
    shared::ids::StatisticId, statistic::ports::statistic_repository::StatisticRepository,
};
use plot_domain::plot::{renderer::PlotRenderer, repository::PlotRepository};
use serde::Deserialize;
use std::str::FromStr;
use validator::Validate;

use crate::{
    dto::statistic::{
        CreateStatisticRequest, LogTimeFrameRequest, LogValueRequest, StatisticMetadataResponse,
    },
    error::ApiError,
    state::AppState,
};

/// GET /statistics - Get all statistics metadata
pub async fn get_all_statistics<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<Vec<StatisticMetadataResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_all_metadata::{
        GetAllMetadataQuery, GetAllMetadataQueryHandler,
    };

    let handler = GetAllMetadataQueryHandler::new(app_state.statistic_repository.clone());
    let summaries = handler
        .handle(GetAllMetadataQuery)
        .map_err(ApiError::from)?;

    let response: Vec<StatisticMetadataResponse> = summaries
        .iter()
        .map(StatisticMetadataResponse::from)
        .collect();
    Ok(Json(response))
}

/// GET /statistics/loggable - Get statistics eligible to be logged directly
pub async fn get_loggable_statistics<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<Vec<StatisticMetadataResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_loggable_metadata::{
        GetLoggableMetadataQuery, GetLoggableMetadataQueryHandler,
    };

    let handler = GetLoggableMetadataQueryHandler::new(app_state.statistic_repository.clone());
    let summaries = handler
        .handle(GetLoggableMetadataQuery)
        .map_err(ApiError::from)?;

    let response: Vec<StatisticMetadataResponse> = summaries
        .iter()
        .map(StatisticMetadataResponse::from)
        .collect();
    Ok(Json(response))
}

/// GET /statistics/:id - Get a single statistic's metadata
pub async fn get_statistic<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<Json<StatisticMetadataResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_metadata::{
        GetMetadataQuery, GetMetadataQueryHandler,
    };

    let query = GetMetadataQuery {
        statistic_id: StatisticId(statistic_id),
    };

    let handler = GetMetadataQueryHandler::new(app_state.statistic_repository.clone());
    let summary = handler
        .handle(query)
        .map_err(|e| ApiError::InternalError(format!("Failed to get statistic: {}", e)))?;

    let response = StatisticMetadataResponse::from(&summary);
    Ok(Json(response))
}

/// POST /statistics/:id/log/:date - Log a value for a statistic on a specific date
pub async fn log_value<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<LogValueRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::CommandHandler;
    use lifestats_application::use_cases::commands::log_value::{
        LogValueCommand as NewLogValueCommand, LogValueCommandHandler, LogValuePayload,
    };
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::error::RepositoryError;
    use lifestats_core::statistic::value_objects::update_policies::{
        amount::AmountUpdatePolicy, fixed_value::FixedValueUpdatePolicy,
        time_frame_list::TimeFrameListUpdatePolicy,
    };
    use std::str::FromStr;

    // Validate the request payload
    payload.validate()?;

    // THIS IS THE ADAPTER'S JOB: Parse the request string into a strongly-typed payload
    // The application core should never see a string

    // 1. Load statistic to determine its type
    let stat = match app_state
        .statistic_repository
        .find_metadata_by_id(StatisticId(statistic_id))
    {
        Ok(stat) => stat,
        Err(RepositoryError::NotFound) => {
            return Err(ApiError::NotFound(
                "The requested resource was not found.".to_string(),
            ));
        }
        Err(RepositoryError::OperationFailed(err)) => {
            return Err(ApiError::InternalServerError(err));
        }
    };

    // 2. Parse value and policy based on statistic type
    let log_payload = match &stat {
        AnyStatistic::IntegerAmount(_) => {
            let value: i64 = payload.value.parse().map_err(|_| {
                ApiError::BadRequest(format!("Expected integer value, got: {}", payload.value))
            })?;

            let update_policy = payload
                .update_policy
                .as_deref()
                .map(|s| {
                    AmountUpdatePolicy::from_str(s).map_err(|_| {
                        ApiError::BadRequest(format!(
                            "Invalid update policy for IntegerAmount: {}",
                            s
                        ))
                    })
                })
                .transpose()?
                .unwrap_or(AmountUpdatePolicy::Sum);

            LogValuePayload::Integer {
                value,
                update_policy,
            }
        }
        AnyStatistic::Habit(_) => {
            let value: bool = payload.value.parse().map_err(|_| {
                ApiError::BadRequest(format!(
                    "Expected boolean value (true/false), got: {}",
                    payload.value
                ))
            })?;

            let update_policy = payload
                .update_policy
                .as_deref()
                .map(|s| {
                    FixedValueUpdatePolicy::from_str(s).map_err(|_| {
                        ApiError::BadRequest(format!("Invalid update policy for Habit: {}", s))
                    })
                })
                .transpose()?
                .unwrap_or(FixedValueUpdatePolicy::Overwrite);

            LogValuePayload::Habit {
                value,
                update_policy,
            }
        }
        AnyStatistic::PointInTime(_) => {
            use chrono::NaiveTime;

            let value = NaiveTime::parse_from_str(&payload.value, "%H:%M:%S")
                .or_else(|_| NaiveTime::parse_from_str(&payload.value, "%H:%M"))
                .map_err(|e| {
                    ApiError::BadRequest(format!(
                        "Invalid time format (expected HH:MM:SS or HH:MM): {}",
                        e
                    ))
                })?;

            let update_policy = payload
                .update_policy
                .as_deref()
                .map(|s| {
                    FixedValueUpdatePolicy::from_str(s).map_err(|_| {
                        ApiError::BadRequest(format!(
                            "Invalid update policy for PointInTime: {}",
                            s
                        ))
                    })
                })
                .transpose()?
                .unwrap_or(FixedValueUpdatePolicy::Overwrite);

            LogValuePayload::PointInTime {
                value,
                update_policy,
            }
        }
        AnyStatistic::TimeSpans(_) => {
            use lifestats_core::shared::time_frame_list::TimeFrameList;

            // For TimeSpans, the value should be a JSON-encoded TimeFrameList
            let value: TimeFrameList = serde_json::from_str(&payload.value)
                .map_err(|e| ApiError::BadRequest(format!("Invalid TimeFrameList JSON: {}", e)))?;

            let update_policy = payload
                .update_policy
                .as_deref()
                .map(|s| {
                    TimeFrameListUpdatePolicy::from_str(s).map_err(|_| {
                        ApiError::BadRequest(format!("Invalid update policy for TimeSpans: {}", s))
                    })
                })
                .transpose()?
                .unwrap_or(TimeFrameListUpdatePolicy::Append);

            LogValuePayload::TimeFrames {
                value,
                update_policy,
            }
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            return Err(ApiError::BadRequest(
                "Cannot log values to derived statistics (Summing, TimeDifference)".to_string(),
            ));
        }
    };

    // 3. Create the strongly-typed command
    let command = NewLogValueCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        payload: log_payload,
        statistic: stat,
    };

    // 4. Instantiate and execute the handler
    let handler = LogValueCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to log value: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// POST /statistics/:id/log_time_frame/:date - Log a time frame for a TimeSpans statistic
pub async fn log_time_frame<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<LogTimeFrameRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use chrono::NaiveTime;
    use lifestats_application::cqrs::CommandHandler;
    use lifestats_application::use_cases::commands::log_value::{
        LogValueCommand, LogValueCommandHandler, LogValuePayload,
    };
    use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::error::RepositoryError;
    use lifestats_core::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;
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
        .transpose()?
        .unwrap_or(TimeFrameListUpdatePolicy::Append);

    // Load statistic to ensure it exists and determine type
    let stat = match app_state
        .statistic_repository
        .find_metadata_by_id(StatisticId(statistic_id))
    {
        Ok(stat) => stat,
        Err(RepositoryError::NotFound) => {
            return Err(ApiError::NotFound(
                "The requested resource was not found.".to_string(),
            ));
        }
        Err(RepositoryError::OperationFailed(err)) => {
            return Err(ApiError::InternalServerError(err));
        }
    };

    match &stat {
        AnyStatistic::TimeSpans(_) => {}
        _ => {
            return Err(ApiError::BadRequest(
                "Cannot log time frames to non-TimeSpans statistics".to_string(),
            ));
        }
    }

    // Create TimeFrame and TimeFrameList
    let time_frame = TimeFrame::new(start_time, end_time)
        .map_err(|e| ApiError::BadRequest(format!("Invalid time frame: {}", e)))?;

    let mut time_frame_list = TimeFrameList::default();
    time_frame_list
        .append_frame(time_frame)
        .map_err(|e| ApiError::BadRequest(format!("Failed to create time frame list: {}", e)))?;

    // Create the command using LogValueCommand
    let command = LogValueCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        payload: LogValuePayload::TimeFrames {
            value: time_frame_list,
            update_policy,
        },
        statistic: stat,
    };

    // Instantiate and execute handler
    let handler = LogValueCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to log time frame: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// POST /statistics - Create a new statistic
pub async fn create_statistic<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Json(payload): Json<CreateStatisticRequest>,
) -> Result<(StatusCode, Json<StatisticMetadataResponse>), ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::CommandHandler;
    use lifestats_application::use_cases::commands::create_statistic::{
        CreateStatisticCommand, CreateStatisticCommandHandler, StatisticTypeDto,
    };

    // Validate the request payload
    payload.validate()?;

    // Convert source_statistic_ids from Vec<u32> to Vec<StatisticId>
    let source_statistic_ids = payload
        .source_statistic_ids
        .map(|ids| ids.into_iter().map(StatisticId).collect());

    let statistic_type = StatisticTypeDto::from_str(&payload.statistic_type)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    // Create the command
    let command = CreateStatisticCommand {
        name: payload.name,
        category: payload.category,
        statistic_type,
        source_statistic_ids,
    };

    // Execute the command using the CQRS handler
    let handler = CreateStatisticCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    let created_statistic = handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to create statistic: {}", e)))?;

    // Convert to response DTO
    let response = StatisticMetadataResponse::from(&created_statistic);

    // Return 201 Created with the created statistic
    Ok((StatusCode::CREATED, Json(response)))
}

/// DELETE /statistics/:id - Delete a statistic
pub async fn delete_statistic<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::CommandHandler;
    use lifestats_application::use_cases::commands::delete_statistic::{
        DeleteStatisticCommand, DeleteStatisticCommandHandler,
    };

    // Create the command
    let command = DeleteStatisticCommand {
        statistic_id: StatisticId(statistic_id),
    };

    // Execute the command using the CQRS handler
    let handler = DeleteStatisticCommandHandler::new(app_state.statistic_repository.clone());
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to delete statistic: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DateRangeParams {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// GET /statistics/:id/time_spans/total - Get total duration for a TimeSpans statistic in a date range
pub async fn get_time_spans_total<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Query(params): Query<DateRangeParams>,
) -> Result<Json<crate::dto::time_spans::TimeSpansTotalResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_time_spans_total::{
        GetTimeSpansTotalQuery, GetTimeSpansTotalQueryHandler,
    };
    use lifestats_core::shared::date_range::DateRange;

    let date_range = DateRange::new(params.start_date, params.end_date)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;

    let handler = GetTimeSpansTotalQueryHandler::new(app_state.statistic_repository.clone());

    let result = handler
        .handle(GetTimeSpansTotalQuery {
            statistic_id: StatisticId(statistic_id),
            date_range,
        })
        .map_err(|e| {
            if e.to_string().contains("not a TimeSpans type") {
                ApiError::BadRequest(e.to_string())
            } else if e.to_string().contains("not found") {
                ApiError::NotFound(e.to_string())
            } else {
                ApiError::InternalError(format!("Failed to get time spans total: {}", e))
            }
        })?;

    Ok(Json(crate::dto::time_spans::TimeSpansTotalResponse::from(
        result,
    )))
}

/// GET /statistics/:id/consistency - Get consistency score for a statistic
pub async fn get_consistency_score<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Query(params): Query<crate::dto::consistency::ConsistencyParams>,
) -> Result<Json<crate::dto::consistency::ConsistencyScoreResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::services::DefaultDailyAchievementEvaluator;
    use lifestats_application::use_cases::queries::get_consistency_score::{
        GetConsistencyScoreQuery, GetConsistencyScoreQueryHandler,
    };
    use std::sync::Arc;

    // Validate query parameters
    params.validate()?;

    // Instantiate dependencies on-demand
    let clock = Arc::new(lifestats_system_clock::SystemClock);
    let evaluator = Arc::new(DefaultDailyAchievementEvaluator::new(
        app_state.statistic_repository.clone(),
    ));

    let handler = GetConsistencyScoreQueryHandler::new(
        app_state.statistic_repository.clone(),
        clock,
        evaluator,
    );

    let result = handler
        .handle(GetConsistencyScoreQuery {
            statistic_id: StatisticId(statistic_id),
            days: params.days,
        })
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::NotFound(e.to_string())
            } else {
                ApiError::InternalError(format!("Failed to get consistency score: {}", e))
            }
        })?;

    // Calculate percentage with round-half-up
    let consistency_score_percent: u8 = if result.days_with_goal == 0 {
        0
    } else {
        let numerator = result.days_achieved * 100 + result.days_with_goal / 2;
        ((numerator / result.days_with_goal).min(100)) as u8
    };

    Ok(Json(crate::dto::consistency::ConsistencyScoreResponse {
        statistic_id,
        days_evaluated: result.days_with_goal,
        days_achieved: result.days_achieved,
        consistency_score_percent,
    }))
}
