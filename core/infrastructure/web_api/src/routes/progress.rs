use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use events::event_port::EventPublisher;
use lifestats_application::cqrs::{CommandHandler, QueryHandler};
use lifestats_application::use_cases::commands::{
    CreateProgressItemCommand, CreateProgressItemCommandHandler, LogProgressCheckpointCommand,
    LogProgressCheckpointCommandHandler, UpdateProgressTargetCommand,
    UpdateProgressTargetCommandHandler,
};
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_application::use_cases::queries::{
    GetProgressCheckpointsQuery, GetProgressCheckpointsQueryHandler, GetProgressDailyDeltasQuery,
    GetProgressDailyDeltasQueryHandler, GetProgressItemQuery, GetProgressItemQueryHandler,
    GetProgressSummaryQuery, GetProgressSummaryQueryHandler, ListProgressItemsQuery,
    ListProgressItemsQueryHandler,
};
use lifestats_core::progress::ports::progress_repository::SaveCheckpointResult;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::ProgressItemId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use validator::Validate;

use crate::dto::progress::{
    CreateProgressItemRequest, LogProgressCheckpointRequest, LogProgressCheckpointResponse,
    ProgressCheckpointResponse, ProgressCheckpointsParams, ProgressDailyDeltaResponse,
    ProgressDailyDeltasParams, ProgressItemResponse, ProgressSummaryResponse,
    UpdateProgressTargetRequest,
};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn create_progress_item<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Json(payload): Json<CreateProgressItemRequest>,
) -> Result<(StatusCode, Json<ProgressItemResponse>), ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    payload.validate()?;
    payload
        .unit
        .validate()
        .map_err(lifestats_application::error::ApplicationError::from)?;

    let handler = CreateProgressItemCommandHandler::new(app_state.progress_repository.clone());
    let item = handler.handle(CreateProgressItemCommand {
        name: payload.name,
        category: payload.category,
        target_value: payload.target_value,
        unit: payload.unit,
        tags: payload.tags,
        created_at: Utc::now(),
    })?;

    Ok((StatusCode::CREATED, Json(ProgressItemResponse::from(&item))))
}

pub async fn list_progress_items<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<Vec<ProgressItemResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = ListProgressItemsQueryHandler::new(app_state.progress_repository.clone());
    let items = handler.handle(ListProgressItemsQuery::default())?;
    Ok(Json(items.iter().map(ProgressItemResponse::from).collect()))
}

pub async fn get_progress_item<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
) -> Result<Json<ProgressItemResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = GetProgressItemQueryHandler::new(app_state.progress_repository.clone());
    let item = handler.handle(GetProgressItemQuery {
        item_id: ProgressItemId(id),
    })?;
    Ok(Json(ProgressItemResponse::from(&item)))
}

pub async fn update_progress_target<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
    Json(payload): Json<UpdateProgressTargetRequest>,
) -> Result<StatusCode, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    payload.validate()?;
    let handler = UpdateProgressTargetCommandHandler::new(app_state.progress_repository.clone());
    handler.handle(UpdateProgressTargetCommand {
        item_id: ProgressItemId(id),
        target_value: payload.target_value,
        updated_at: Utc::now(),
    })?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn log_progress_checkpoint<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
    Json(payload): Json<LogProgressCheckpointRequest>,
) -> Result<(StatusCode, Json<LogProgressCheckpointResponse>), ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    payload.validate()?;
    let recorded_at = payload.recorded_at.unwrap_or_else(Utc::now);

    let handler = LogProgressCheckpointCommandHandler::new(app_state.progress_repository.clone());
    let result = handler.handle(LogProgressCheckpointCommand {
        item_id: ProgressItemId(id),
        recorded_at,
        effective_date: payload.effective_date,
        position: payload.position,
        source: payload.source,
        source_event_id: payload.source_event_id,
        note: payload.note,
    })?;

    let (status_code, status, checkpoint) = match result {
        SaveCheckpointResult::Inserted(checkpoint) => {
            (StatusCode::CREATED, "inserted".to_string(), checkpoint)
        }
        SaveCheckpointResult::Duplicate(checkpoint) => {
            (StatusCode::OK, "duplicate".to_string(), checkpoint)
        }
    };

    Ok((
        status_code,
        Json(LogProgressCheckpointResponse {
            status,
            checkpoint: ProgressCheckpointResponse::from(&checkpoint),
        }),
    ))
}

pub async fn get_progress_checkpoints<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
    Query(params): Query<ProgressCheckpointsParams>,
) -> Result<Json<Vec<ProgressCheckpointResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = GetProgressCheckpointsQueryHandler::new(app_state.progress_repository.clone());
    let checkpoints = handler.handle(GetProgressCheckpointsQuery {
        item_id: ProgressItemId(id),
        recorded_at_start: params.recorded_at_start,
        recorded_at_end: params.recorded_at_end,
    })?;

    Ok(Json(
        checkpoints
            .iter()
            .map(ProgressCheckpointResponse::from)
            .collect(),
    ))
}

pub async fn get_progress_summary<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
) -> Result<Json<ProgressSummaryResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let handler = GetProgressSummaryQueryHandler::new(app_state.progress_repository.clone());
    let summary = handler.handle(GetProgressSummaryQuery {
        item_id: ProgressItemId(id),
    })?;
    Ok(Json(ProgressSummaryResponse::from(summary)))
}

pub async fn get_progress_daily_deltas<R, Rend, E, I>(
    State(app_state): State<AppState<R, Rend, E, I>>,
    Path(id): Path<u32>,
    Query(params): Query<ProgressDailyDeltasParams>,
) -> Result<Json<Vec<ProgressDailyDeltaResponse>>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    params.validate()?;
    let date_range = DateRange::new(params.start, params.end)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;
    let handler = GetProgressDailyDeltasQueryHandler::new(app_state.progress_repository.clone());
    let deltas = handler.handle(GetProgressDailyDeltasQuery {
        item_id: ProgressItemId(id),
        date_range,
    })?;

    Ok(Json(
        deltas
            .into_iter()
            .map(|dated| ProgressDailyDeltaResponse {
                date: dated.date,
                value: dated.value,
            })
            .collect(),
    ))
}
