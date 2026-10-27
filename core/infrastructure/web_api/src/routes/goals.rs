use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::{
    shared::ids::StatisticId, statistic::ports::statistic_repository::StatisticRepository,
};
use plot_domain::plot::renderer::PlotRenderer;
use validator::Validate;

use crate::{
    dto::goal::{
        DailyGoalResponse, GoalResponse, SetDailyGoalRangeRequest, SetDailyGoalRequest,
        SetDefaultGoalRequest,
    },
    error::ApiError,
    state::AppState,
};

use chrono::NaiveTime;
use lifestats_application::cqrs::CommandHandler;
use lifestats_application::cqrs::QueryHandler;
use lifestats_application::use_cases::commands::set_daily_goal::{
    DailyGoalOverride, GoalPayload, SetDailyGoalCommand, SetDailyGoalCommandHandler,
};
use lifestats_application::use_cases::queries::get_daily_goal::{
    GetDailyGoalQuery, GetDailyGoalQueryHandler,
};
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::AnyStatistic;

/// GET /statistics/:id/goals/default - Get the default goal for a statistic
pub async fn get_default_goal<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<Json<Option<GoalResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::QueryHandler;
    use lifestats_application::use_cases::queries::get_default_goal::{
        GetDefaultGoalQuery, GetDefaultGoalQueryHandler,
    };

    let handler = GetDefaultGoalQueryHandler::new(app_state.statistic_repository.clone());
    let goal_json = handler
        .handle(GetDefaultGoalQuery {
            statistic_id: StatisticId(statistic_id),
        })
        .map_err(|e| ApiError::InternalError(format!("Failed to get default goal: {}", e)))?;

    let response = goal_json.map(|goal| GoalResponse {
        goal_data: goal.goal_data,
        valid_since: goal.valid_since,
    });

    Ok(Json(response))
}

/// PUT /statistics/:id/goals/default - Set the default goal for a statistic, from
/// `valid_since` (default: today) onwards
pub async fn set_default_goal<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Json(payload): Json<SetDefaultGoalRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::commands::set_default_goal::{
        SetDefaultGoalCommand, SetDefaultGoalCommandHandler,
    };

    payload.validate()?;

    // THIS IS THE ADAPTER'S JOB: parse the request HashMap into a strongly-typed
    // payload based on the statistic's type. The application core never sees a HashMap.
    let stat = app_state
        .statistic_repository
        .find_metadata_by_id(StatisticId(statistic_id))
        .map_err(|e| ApiError::InternalError(format!("Failed to load statistic: {}", e)))?;
    let goal_payload = parse_hashmap_into_goal_payload(payload.goal_data, stat)?;

    let command = SetDefaultGoalCommand {
        statistic_id: StatisticId(statistic_id),
        goal: goal_payload,
        valid_since: payload.valid_since,
    };

    let handler = SetDefaultGoalCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to set default goal: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /statistics/:id/goals/default - Delete the default goal for a statistic
pub async fn delete_default_goal<R, Rend, E, I>(
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
    use lifestats_application::use_cases::commands::delete_default_goal::{
        DeleteDefaultGoalCommand, DeleteDefaultGoalCommandHandler,
    };

    let command = DeleteDefaultGoalCommand {
        statistic_id: StatisticId(statistic_id),
    };

    let handler = DeleteDefaultGoalCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to delete default goal: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// GET /statistics/:id/goals/:date - Get a goal for a specific date
pub async fn get_daily_goal<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
) -> Result<Json<DailyGoalResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = GetDailyGoalQueryHandler::new(app_state.statistic_repository.clone());
    let goal_result = handler
        .handle(GetDailyGoalQuery {
            statistic_id: StatisticId(statistic_id),
            date,
        })
        .map_err(|e| ApiError::InternalError(format!("Failed to get goal: {}", e)))?;

    Ok(Json(DailyGoalResponse::from(goal_result)))
}

/// PUT /statistics/{id}/goals/{date} - Set a goal for a specific date
pub async fn set_daily_goal<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<SetDailyGoalRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let stat = app_state
        .statistic_repository
        .find_metadata_by_id(StatisticId(statistic_id))
        .map_err(|e| ApiError::InternalError(format!("Failed to load statistic: {}", e)))?;

    let daily_goal_override = match payload {
        SetDailyGoalRequest::Goal { goal_data } => {
            DailyGoalOverride::Goal(parse_hashmap_into_goal_payload(goal_data, stat)?)
        }
        SetDailyGoalRequest::Suppressed => DailyGoalOverride::Suppressed,
    };

    // Create command
    let command = SetDailyGoalCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        override_kind: daily_goal_override,
    };

    // Instantiate and execute handler
    let handler = SetDailyGoalCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to set goal: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

fn parse_hashmap_into_goal_payload(
    payload: HashMap<String, String>,
    stat: lifestats_core::statistic::any_statistic::AnyStatistic,
) -> Result<lifestats_application::use_cases::commands::GoalPayload, ApiError> {
    let goal_payload = match stat {
        AnyStatistic::IntegerAmount(_) => {
            let target = payload
                .get("target")
                .ok_or_else(|| ApiError::BadRequest("Missing 'target' field".to_string()))?
                .parse::<i64>()
                .map_err(|_| {
                    ApiError::BadRequest("'target' must be a valid integer".to_string())
                })?;

            let is_at_least = payload
                .get("is_at_least")
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|_| {
                    ApiError::BadRequest("'is_at_least' must be 'true' or 'false'".to_string())
                })?
                .unwrap_or(true);

            GoalPayload::IntegerAmount {
                target,
                is_at_least,
            }
        }
        AnyStatistic::Habit(_) => {
            let occurred = payload
                .get("occurred")
                .ok_or_else(|| ApiError::BadRequest("Missing 'occurred' field".to_string()))?
                .parse::<bool>()
                .map_err(|_| {
                    ApiError::BadRequest("'occurred' must be 'true' or 'false'".to_string())
                })?;

            GoalPayload::Habit { occurred }
        }
        AnyStatistic::PointInTime(_) => {
            let deadline_str = payload
                .get("deadline")
                .ok_or_else(|| ApiError::BadRequest("Missing 'deadline' field".to_string()))?;

            let deadline = NaiveTime::parse_from_str(deadline_str, "%H:%M:%S")
                .or_else(|_| NaiveTime::parse_from_str(deadline_str, "%H:%M"))
                .map_err(|e| {
                    ApiError::BadRequest(format!(
                        "Invalid deadline format (expected HH:MM:SS or HH:MM): {}",
                        e
                    ))
                })?;

            GoalPayload::FinishedInTime { deadline }
        }
        AnyStatistic::TimeSpans(_) => {
            let target_str = payload
                .get("target")
                .ok_or_else(|| ApiError::BadRequest("Missing 'target' field".to_string()))?;

            let target: TimeFrameList = serde_json::from_str(target_str)
                .map_err(|e| ApiError::BadRequest(format!("Invalid TimeFrameList JSON: {}", e)))?;

            let efficiency = payload
                .get("efficiency")
                .ok_or_else(|| ApiError::BadRequest("Missing 'efficiency' field".to_string()))?
                .parse::<u8>()
                .map_err(|_| {
                    ApiError::BadRequest("'efficiency' must be a number 0-100".to_string())
                })?;

            let efficiency = Percentage::new(efficiency);

            GoalPayload::TimeSpans { target, efficiency }
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            let target = payload
                .get("target")
                .ok_or_else(|| ApiError::BadRequest("Missing 'target' field".to_string()))?
                .parse::<i64>()
                .map_err(|_| ApiError::BadRequest("'target' must be a valid number".to_string()))?;

            let is_at_least = payload
                .get("is_at_least")
                .map(|v| v.parse::<bool>())
                .transpose()
                .map_err(|_| {
                    ApiError::BadRequest("'is_at_least' must be 'true' or 'false'".to_string())
                })?
                .unwrap_or(true);

            GoalPayload::SecondsAmount {
                target: Seconds::from_i64(target),
                is_at_least,
            }
        }
    };
    Ok(goal_payload)
}

/// Longest range `PUT /statistics/{id}/goals/range` accepts, to catch runaway selections.
const MAX_GOAL_RANGE_DAYS: i64 = 366;

/// PUT /statistics/{id}/goals/range - Give every day from `start` to `end` the same
/// daily override (a goal or a suppression), all or nothing
pub async fn set_daily_goal_range<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Json(payload): Json<SetDailyGoalRangeRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::commands::set_daily_goal_range::{
        SetDailyGoalRangeCommand, SetDailyGoalRangeCommandHandler,
    };
    use lifestats_core::shared::date_range::DateRange;

    let date_range = DateRange::new(payload.start, payload.end)
        .map_err(|e| ApiError::BadRequest(format!("Invalid date range: {}", e)))?;
    let days = (payload.end - payload.start).num_days() + 1;
    if days > MAX_GOAL_RANGE_DAYS {
        return Err(ApiError::BadRequest(format!(
            "Date range covers {} days; at most {} are allowed",
            days, MAX_GOAL_RANGE_DAYS
        )));
    }

    let stat = app_state
        .statistic_repository
        .find_metadata_by_id(StatisticId(statistic_id))
        .map_err(lifestats_application::error::ApplicationError::from)?;

    let override_kind = match payload.goal {
        SetDailyGoalRequest::Goal { goal_data } => {
            DailyGoalOverride::Goal(parse_hashmap_into_goal_payload(goal_data, stat)?)
        }
        SetDailyGoalRequest::Suppressed => DailyGoalOverride::Suppressed,
    };

    let handler = SetDailyGoalRangeCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler.handle(SetDailyGoalRangeCommand {
        statistic_id: StatisticId(statistic_id),
        date_range,
        override_kind,
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /statistics/:id/goals/:date - Delete a goal for a specific date
pub async fn delete_daily_goal<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,

    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::cqrs::CommandHandler;
    use lifestats_application::use_cases::commands::delete_daily_goal::{
        DeleteDailyGoalCommand, DeleteDailyGoalCommandHandler,
    };

    let command = DeleteDailyGoalCommand {
        statistic_id: StatisticId(statistic_id),
        date,
    };

    let handler = DeleteDailyGoalCommandHandler::new(
        app_state.statistic_repository.clone(),
        app_state.event_publisher.clone(),
    );
    handler
        .handle(command)
        .map_err(|e| ApiError::InternalError(format!("Failed to delete goal: {}", e)))?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}
