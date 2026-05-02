use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::{
    EbookEventIngestionRequest, IntegrationServiceTrait,
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
