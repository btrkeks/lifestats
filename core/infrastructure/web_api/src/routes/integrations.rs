use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::{
    EbookEventIngestionRequest, IntegrationServiceTrait, LiftLogWorkout,
};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;

use crate::{error::ApiError, state::AppState};

/// Request body for provisioning an integration bundle
#[derive(Debug, Deserialize)]
pub struct ProvisionBundleRequest {
    /// Context for the integration (e.g., "deck_name": "Japanese")
    #[serde(flatten)]
    pub context: HashMap<String, String>,
}

/// Response body for a successfully provisioned bundle
#[derive(Debug, Serialize)]
pub struct ProvisionBundleResponse {
    /// Map from logical key to statistic ID
    #[serde(flatten)]
    pub statistic_ids: HashMap<String, u32>,
}

#[derive(Debug, Deserialize)]
pub struct EbookEventRequest {
    pub event_id: String,
    pub date: NaiveDate,
    pub metric: String,
    pub delta: i64,
    pub book_id: Option<u32>,
    pub book_title: String,
    pub confirmed_position_after: i64,
    pub book_target_characters: i64,
}

#[derive(Debug, Serialize)]
pub struct EbookEventResponse {
    pub accepted: bool,
    pub duplicate: bool,
}

#[derive(Debug, Deserialize)]
pub struct LiftLogDaysRequest {
    pub days: Vec<LiftLogDayDto>,
}

#[derive(Debug, Deserialize)]
pub struct LiftLogDayDto {
    pub date: NaiveDate,
    pub workouts: Vec<LiftLogWorkoutDto>,
}

/// `start`/`end` are RFC 3339 timestamps with the device's UTC offset.
#[derive(Debug, Deserialize)]
pub struct LiftLogWorkoutDto {
    pub id: String,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct LiftLogDaysResponse {
    pub statistic_id: u32,
    pub days_updated: usize,
    pub days_cleared: usize,
    pub days_unchanged: usize,
}

const MAX_LIFTLOG_DAYS: usize = 500;
const MAX_LIFTLOG_WORKOUTS_PER_DAY: usize = 100;

/// Validates a LiftLog days request into one entry per date. Timestamps keep
/// the device's wall-clock time (`naive_local`), which is how every other
/// Lifestats time frame is recorded.
fn parse_liftlog_days(
    request: LiftLogDaysRequest,
) -> Result<BTreeMap<NaiveDate, Vec<LiftLogWorkout>>, String> {
    if request.days.is_empty() {
        return Err("days must not be empty".to_string());
    }
    if request.days.len() > MAX_LIFTLOG_DAYS {
        return Err(format!(
            "at most {MAX_LIFTLOG_DAYS} days per request, got {}",
            request.days.len()
        ));
    }

    let mut days = BTreeMap::new();
    for day in request.days {
        if day.workouts.len() > MAX_LIFTLOG_WORKOUTS_PER_DAY {
            return Err(format!(
                "date {} has {} workouts, at most {MAX_LIFTLOG_WORKOUTS_PER_DAY} allowed",
                day.date,
                day.workouts.len()
            ));
        }
        let workouts = day
            .workouts
            .into_iter()
            .map(|workout| {
                let parse = |field: &str, value: &str| {
                    DateTime::parse_from_rfc3339(value).map_err(|_| {
                        format!(
                            "workout {}: {field} is not an RFC 3339 timestamp: {value}",
                            workout.id
                        )
                    })
                };
                let start = parse("start", &workout.start)?;
                let end = parse("end", &workout.end)?;
                // Compared as instants: across a DST change the local end can
                // precede the local start.
                if end < start {
                    return Err(format!("workout {}: end is before start", workout.id));
                }
                Ok(LiftLogWorkout {
                    started_at: start.naive_local(),
                    finished_at: end.naive_local(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if days.insert(day.date, workouts).is_some() {
            return Err(format!("duplicate date {}", day.date));
        }
    }
    Ok(days)
}

/// POST /integrations/:source
///
/// Provisions a bundle of statistics for a given integration source.
/// This endpoint is idempotent: calling it multiple times with the same
/// source and context will return the same statistic IDs.
pub async fn provision_integration<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(source): Path<String>,
    Json(request): Json<ProvisionBundleRequest>,
) -> Result<Json<ProvisionBundleResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // Call the integration service to provision the bundle
    let result = state
        .integration_service
        .provision_bundle(&source, request.context)?;

    // Convert StatisticId to u32 for JSON response
    let statistic_ids: HashMap<String, u32> = result
        .into_iter()
        .map(|(key, id)| (format!("{}_id", key), id.0))
        .collect();

    Ok(Json(ProvisionBundleResponse { statistic_ids }))
}

pub async fn ingest_ebook_event<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Json(request): Json<EbookEventRequest>,
) -> Result<(StatusCode, Json<EbookEventResponse>), ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let event_id = request.event_id.clone();
    let date = request.date;
    let metric = request.metric.clone();
    let delta = request.delta;
    let book_id = request
        .book_id
        .ok_or_else(|| ApiError::BadRequest("ebook book_id is required".to_string()))?;
    if book_id == 0 {
        return Err(ApiError::BadRequest(
            "ebook book_id must be positive".to_string(),
        ));
    }
    let book_title = request.book_title.clone();
    let confirmed_position_after = request.confirmed_position_after;
    let book_target_characters = request.book_target_characters;

    let result = state
        .integration_service
        .ingest_ebook_event(EbookEventIngestionRequest {
            event_id: event_id.clone(),
            date,
            metric: metric.clone(),
            delta,
            book_id: book_id.to_string(),
            book_title: book_title.clone(),
            confirmed_position_after,
            book_target_characters,
        })
        .map_err(|error| {
            tracing::error!(
                event_id = %event_id,
                date = %date,
                metric = %metric,
                delta,
                book_id = book_id,
                book_title = %book_title,
                confirmed_position_after,
                book_target_characters,
                error = %error,
                error_debug = ?error,
                "Failed to ingest ebook sync event"
            );
            ApiError::from(error)
        })?;

    tracing::debug!(
        event_id = %event_id,
        date = %date,
        metric = %metric,
        delta,
        book_id = book_id,
        book_title = %book_title,
        confirmed_position_after,
        book_target_characters,
        accepted = result.accepted,
        statistic_id = result.statistic_id.0,
        "Processed ebook sync event"
    );

    if result.accepted {
        state
            .event_publisher
            .publish_event_async(Event::ValueUpdated(result.statistic_id));
    }

    Ok((
        StatusCode::OK,
        Json(EbookEventResponse {
            accepted: result.accepted,
            duplicate: !result.accepted,
        }),
    ))
}

/// PUT /integrations/liftlog/days
///
/// Replaces `[LiftLog] Workout Time` on exactly the named days with the given
/// finished workouts; `workouts: []` removes the day's value. Days not named
/// are untouched and resending is a no-op.
pub async fn sync_liftlog_days<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    body: Result<Json<LiftLogDaysRequest>, JsonRejection>,
) -> Result<Json<LiftLogDaysResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let Json(request) = body.map_err(|rejection| ApiError::BadRequest(rejection.body_text()))?;
    let days = parse_liftlog_days(request).map_err(|message| {
        tracing::warn!(error = %message, "Rejected liftlog days request");
        ApiError::BadRequest(message)
    })?;
    let day_count = days.len();

    let result = state
        .integration_service
        .sync_liftlog_days(days)
        .map_err(|error| {
            tracing::error!(
                days = day_count,
                error = %error,
                error_debug = ?error,
                "Failed to sync liftlog days"
            );
            ApiError::from(error)
        })?;

    tracing::info!(
        days = day_count,
        days_updated = result.days_updated,
        days_cleared = result.days_cleared,
        days_unchanged = result.days_unchanged,
        "Synced liftlog days"
    );

    if result.changed() {
        state
            .event_publisher
            .publish_event_async(Event::ValueUpdated(result.statistic_id));
    }

    Ok(Json(LiftLogDaysResponse {
        statistic_id: result.statistic_id.0,
        days_updated: result.days_updated,
        days_cleared: result.days_cleared,
        days_unchanged: result.days_unchanged,
    }))
}
