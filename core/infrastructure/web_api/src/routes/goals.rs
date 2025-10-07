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
use plot_domain::plot::{renderer::PlotRenderer, repository::PlotRepository};
use validator::Validate;

use crate::{
    dto::goal::{GoalResponse, SetGoalRequest},
    error::ApiError,
    state::AppState,
};

/// GET /statistics/:id/goals/default - Get the default goal for a statistic
pub async fn get_default_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<Json<Option<GoalResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let goal_json = app_state
        .query_service
        .get_default_goal(StatisticId(statistic_id))?;

    let response = goal_json.map(|goal_data| GoalResponse { goal_data });

    Ok(Json(response))
}

/// PUT /statistics/:id/goals/default - Set the default goal for a statistic
pub async fn set_default_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
    Json(payload): Json<SetGoalRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::goal_management_service::SetDefaultGoalGenericCommand;

    // Validate the request payload
    payload.validate()?;

    // Create the command
    let command = SetDefaultGoalGenericCommand {
        statistic_id: StatisticId(statistic_id),
        goal_data: payload.goal_data,
    };

    // Execute the command
    app_state.goal_service.set_default_goal_generic(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /statistics/:id/goals/default - Delete the default goal for a statistic
pub async fn delete_default_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path(statistic_id): Path<u32>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::goal_management_service::DeleteDefaultGoalCommand;

    // Create the command
    let command = DeleteDefaultGoalCommand {
        statistic_id: StatisticId(statistic_id),
    };

    // Execute the command
    app_state.goal_service.delete_default_goal(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// GET /statistics/:id/goals/:date - Get a goal for a specific date
pub async fn get_daily_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
) -> Result<Json<Option<GoalResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let goal_json = app_state
        .query_service
        .get_goal(StatisticId(statistic_id), date)?;

    let response = goal_json.map(|goal_data| GoalResponse { goal_data });

    Ok(Json(response))
}

/// PUT /statistics/:id/goals/:date - Set a goal for a specific date
pub async fn set_daily_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
    Json(payload): Json<SetGoalRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::goal_management_service::SetGoalCommand;

    // Validate the request payload
    payload.validate()?;

    // Create the command
    let command = SetGoalCommand {
        statistic_id: StatisticId(statistic_id),
        date,
        goal_data: payload.goal_data,
    };

    // Execute the command
    app_state.goal_service.set_goal(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /statistics/:id/goals/:date - Delete a goal for a specific date
pub async fn delete_daily_goal<R, P, Rend, E, I>(
    State(app_state): State<AppState<R, P, Rend, E, I>>,
    Path((statistic_id, date)): Path<(u32, NaiveDate)>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    use lifestats_application::use_cases::goal_management_service::DeleteGoalCommand;

    // Create the command
    let command = DeleteGoalCommand {
        statistic_id: StatisticId(statistic_id),
        date,
    };

    // Execute the command
    app_state.goal_service.delete_goal(command)?;

    // Return 204 No Content on success
    Ok(StatusCode::NO_CONTENT)
}
