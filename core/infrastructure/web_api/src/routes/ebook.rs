use super::integration_error::ErrorEnvelope;
use super::integration_error::{parse_day, parse_strict_object, IntegrationApiError};
use crate::state::AppState;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Local, Utc};
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::ebook::{BookKey, BookTitle, EbookDayReport, ProducerId, SpanSet};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct EbookDescriptor {
    #[schema(format = "uuid")]
    pub server_id: String,
    #[schema(minimum = 1, maximum = 1)]
    pub protocol: u8,
}

#[utoipa::path(get, path = "/integrations/ebook", responses(
    (status = 200, description = "Persistent database identity and ebook protocol version", body = EbookDescriptor),
    (status = 500, description = "Storage unavailable", body = ErrorEnvelope)
))]
pub async fn descriptor<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
) -> Result<Json<EbookDescriptor>, IntegrationApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    Ok(Json(EbookDescriptor {
        server_id: state.integration_service.ebook_server_id()?,
        protocol: 1,
    }))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EbookDayRequest {
    #[schema(min_length = 1, max_length = 1000)]
    pub title: String,
    #[schema(minimum = 1, maximum = 9007199254740992_i64)]
    pub length: u64,
    /// Normalized half-open [start,end) pairs, 0 <= start < end <= length; sorted, disjoint and non-adjacent.
    #[schema(value_type = Vec<Vec<u64>>, max_items = 4096)]
    pub spans: Vec<(u64, u64)>,
    #[schema(minimum = 0, maximum = 9007199254740992_i64)]
    pub last_position: u64,
    #[schema(format = DateTime)]
    pub observed_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EbookLegacyBookRequest {
    #[schema(pattern = "^ttu-text-v1:[0-9a-f]{64}$")]
    pub book_key: String,
    #[schema(min_length = 1, max_length = 1000)]
    pub title: String,
    #[schema(minimum = 1, maximum = 9007199254740992_i64)]
    pub length: u64,
}

#[utoipa::path(
    put,
    path = "/integrations/ebook/producers/{producer}/books/{book_key}/days/{day}",
    params(
        ("producer" = String, Path, description = "Canonical lowercase UUID of the reporting ttu origin"),
        ("book_key" = String, Path, description = "ttu-text-v1: followed by 64 lowercase hex characters"),
        ("day" = String, Path, description = "Strict YYYY-MM-DD reading day (client local, 04:00 rollover)")
    ),
    request_body = EbookDayRequest,
    responses(
        (status = 204, description = "Merged (or already contained) and every projection committed"),
        (status = 400, description = "Malformed JSON or unreadable body", body = ErrorEnvelope),
        (status = 409, description = "Bundle conflict, book length mismatch, or legacy adoption pending", body = ErrorEnvelope),
        (status = 415, description = "Unsupported media type", body = ErrorEnvelope),
        (status = 422, description = "Invalid path or field, or aggregate overflow", body = ErrorEnvelope),
        (status = 500, description = "Storage failed without committing", body = ErrorEnvelope)
    )
)]
pub async fn put_day<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path((producer, book_key, day)): Path<(String, String, String)>,
    body: Result<Json<Box<RawValue>>, JsonRejection>,
) -> Result<StatusCode, IntegrationApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let Json(raw) = body.map_err(IntegrationApiError::json)?;
    let request = EbookDayRequest::parse(&raw)?;
    let producer = ProducerId::parse(&producer)?;
    let book = BookKey::parse(&book_key)?;
    let day = parse_day(&day)?;
    if day > Local::now().date_naive() + chrono::Duration::days(2) {
        return Err(IntegrationApiError::invalid(
            Some("day"),
            "Day must be at most two days after the server local date",
        ));
    }
    let report = EbookDayReport::new(
        BookTitle::parse(&request.title)?,
        request.length,
        SpanSet::from_normalized(request.spans)?,
        request.last_position,
        DateTime::parse_from_rfc3339(&request.observed_at)
            .map_err(|_| {
                IntegrationApiError::invalid(Some("observed_at"), "Expected an RFC 3339 timestamp")
            })?
            .with_timezone(&Utc),
    )?;
    let outcome = state
        .integration_service
        .record_ebook_day(producer, book, day, report)?;
    for id in outcome.changed_statistics {
        state
            .event_publisher
            .publish_event_async(events::event_port::Event::ValueUpdated(id));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/integrations/ebook/legacy-books/{reader_data_id}",
    params(("reader_data_id" = u32, Path, description = "ttu data.id used by the pre-2026-10 event API")),
    request_body = EbookLegacyBookRequest,
    responses(
        (status = 204, description = "Bound (or already bound to the same key)"),
        (status = 400, description = "Malformed JSON or unreadable body", body = ErrorEnvelope),
        (status = 415, description = "Unsupported media type", body = ErrorEnvelope),
        (status = 409, description = "Bound to another key, title mismatch, or book already bound to another item", body = ErrorEnvelope),
        (status = 422, description = "Invalid path or field", body = ErrorEnvelope),
        (status = 500, description = "Storage failed without committing", body = ErrorEnvelope)
    )
)]
pub async fn put_legacy_book<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(reader_data_id): Path<String>,
    body: Result<Json<Box<RawValue>>, JsonRejection>,
) -> Result<StatusCode, IntegrationApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let Json(raw) = body.map_err(IntegrationApiError::json)?;
    let object = parse_strict_object(&raw)?;
    let _: String = required(&object, "book_key")?;
    let _: String = required(&object, "title")?;
    let _: u64 = required(&object, "length")?;
    let request: EbookLegacyBookRequest = serde_json::from_str(raw.get()).map_err(|_| {
        IntegrationApiError::invalid(None, "Unexpected or duplicate request fields")
    })?;
    let reader_data_id = reader_data_id
        .parse::<u32>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| {
            IntegrationApiError::invalid(
                Some("reader_data_id"),
                "Expected a positive u32 reader ID",
            )
        })?;
    let outcome = state.integration_service.bind_legacy_ebook_book(
        reader_data_id,
        BookKey::parse(&request.book_key)?,
        BookTitle::parse(&request.title)?,
        lifestats_core::ebook::parse_length(request.length)?,
    )?;
    for id in outcome.changed_statistics {
        state
            .event_publisher
            .publish_event_async(events::event_port::Event::ValueUpdated(id));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn provision_legacy<R, Rend, E, I>(
    state: State<AppState<R, Rend, E, I>>,
    request: Json<super::integrations::ProvisionBundleRequest>,
) -> Result<Json<super::integrations::ProvisionBundleResponse>, crate::error::ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    super::integrations::provision_integration(state, Path("ebook".into()), request).await
}

fn required<T: serde::de::DeserializeOwned>(
    object: &BTreeMap<String, Box<RawValue>>,
    field: &'static str,
) -> Result<T, IntegrationApiError> {
    object
        .get(field)
        .and_then(|raw| serde_json::from_str(raw.get()).ok())
        .ok_or_else(|| {
            IntegrationApiError::invalid(Some(field), "Required field with the documented type")
        })
}
impl EbookDayRequest {
    fn parse(raw: &RawValue) -> Result<Self, IntegrationApiError> {
        let object = parse_strict_object(raw)?;
        let _: String = required(&object, "title")?;
        let _: u64 = required(&object, "length")?;
        let _: Vec<(u64, u64)> = required(&object, "spans")?;
        let _: u64 = required(&object, "last_position")?;
        let _: String = required(&object, "observed_at")?;
        serde_json::from_str(raw.get()).map_err(|_| {
            IntegrationApiError::invalid(None, "Unexpected or duplicate request fields")
        })
    }
}
