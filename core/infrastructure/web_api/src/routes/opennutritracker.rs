use crate::state::AppState;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::{
    opennutritracker::{OpenNutriTrackerDay, OpenNutriTrackerDayError},
    shared::timed_amount_list::{
        parse_entry_time, TimedAmount, TimedAmountError, TimedAmountList, MAX_DAILY_ENTRIES,
    },
    statistic::ports::statistic_repository::StatisticRepository,
};
use plot_domain::plot::renderer::PlotRenderer;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use utoipa::ToSchema;

pub use super::integration_error::{
    ErrorCode as DayErrorCode, ErrorEnvelope as DayErrorEnvelope,
    IntegrationApiError as DayApiError,
};

/// One intake as the app recorded it.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimedMilligramsRequest {
    /// Exactly HH:MM:SS, the entry's local wall-clock time. `null` when the app knows
    /// the day but not the time.
    #[serde(deserialize_with = "Option::deserialize")]
    #[schema(required = true, nullable = true, value_type = Option<String>, example = "08:12:00")]
    pub time: Option<String>,
    #[schema(minimum = 1, maximum = 9007199254740991_i64)]
    pub amount_mg: i64,
}

/// The app's complete day for both metrics. `[]` is a confirmed zero.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenNutriTrackerDayRequest {
    #[schema(max_items = 1000)]
    pub caffeine: Vec<TimedMilligramsRequest>,
    #[schema(max_items = 1000)]
    pub nicotine: Vec<TimedMilligramsRequest>,
}

impl OpenNutriTrackerDayRequest {
    /// Validates raw tokens field by field (so errors name the exact entry), then
    /// strictly parses the whole body to reject unknown and duplicate keys.
    fn parse(value: &RawValue) -> Result<OpenNutriTrackerDay, DayApiError> {
        let object = super::integration_error::parse_strict_object(value)?;
        let caffeine = parse_metric(&object, "caffeine")?;
        let nicotine = parse_metric(&object, "nicotine")?;
        serde_json::from_str::<Self>(value.get())
            .map_err(|_| DayApiError::invalid(None, "Unexpected or duplicate request fields"))?;
        Ok(OpenNutriTrackerDay::new(caffeine, nicotine))
    }
}

fn parse_metric(
    object: &BTreeMap<String, Box<RawValue>>,
    field: &str,
) -> Result<TimedAmountList, DayApiError> {
    let entries = object
        .get(field)
        .and_then(|raw| serde_json::from_str::<Vec<Box<RawValue>>>(raw.get()).ok())
        .ok_or_else(|| {
            DayApiError::invalid(Some(field), "Expected a required array of timed amounts")
        })?;
    if entries.len() > MAX_DAILY_ENTRIES {
        return Err(DayApiError::invalid(
            Some(field),
            "Expected at most 1000 entries",
        ));
    }
    let entries = entries
        .iter()
        .enumerate()
        .map(|(index, raw)| parse_entry(raw, &format!("{field}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    TimedAmountList::new(entries).map_err(|error| match error {
        TimedAmountError::TotalOverflow => DayApiError::new(
            DayErrorCode::ProjectionOverflow,
            Some(field),
            "Daily amount exceeds the supported integer range",
        ),
        _ => DayApiError::invalid(Some(field), &error.to_string()),
    })
}

fn parse_entry(raw: &RawValue, path: &str) -> Result<TimedAmount, DayApiError> {
    let entry =
        serde_json::from_str::<BTreeMap<String, Box<RawValue>>>(raw.get()).map_err(|_| {
            DayApiError::invalid(Some(path), "Expected an object with time and amount_mg")
        })?;
    let time_field = format!("{path}.time");
    let time = match entry.get("time").map(|raw| raw.get()) {
        Some("null") => None,
        Some(raw) => Some(
            serde_json::from_str::<String>(raw)
                .ok()
                .and_then(|text| parse_entry_time(&text).ok())
                .ok_or_else(|| {
                    DayApiError::invalid(Some(&time_field), "Expected HH:MM:SS local time or null")
                })?,
        ),
        None => {
            return Err(DayApiError::invalid(
                Some(&time_field),
                "Expected HH:MM:SS local time or null",
            ))
        }
    };
    let amount_field = format!("{path}.amount_mg");
    let invalid_amount = || {
        DayApiError::invalid(
            Some(&amount_field),
            "Expected a positive integer mg amount within the supported range",
        )
    };
    // Parsing the raw token as i64 rejects 120.0, 1e2, strings and oversized numbers.
    let amount = entry
        .get("amount_mg")
        .and_then(|raw| serde_json::from_str::<i64>(raw.get()).ok())
        .ok_or_else(invalid_amount)?;
    serde_json::from_str::<TimedMilligramsRequest>(raw.get())
        .map_err(|_| DayApiError::invalid(Some(path), "Unexpected or duplicate entry fields"))?;
    TimedAmount::new(time, amount).map_err(|_| invalid_amount())
}

impl From<OpenNutriTrackerDayError> for DayApiError {
    fn from(error: OpenNutriTrackerDayError) -> Self {
        match error {
            OpenNutriTrackerDayError::BundleConflict => Self::new(
                DayErrorCode::BundleConflict,
                None,
                "OpenNutriTracker statistics require correction",
            ),
            OpenNutriTrackerDayError::Storage(error) => {
                tracing::error!(error=?error, "Failed to replace OpenNutriTracker day");
                Self::new(
                    DayErrorCode::StorageError,
                    None,
                    "Unable to store OpenNutriTracker day",
                )
            }
        }
    }
}

#[utoipa::path(
    put,
    path = "/integrations/opennutritracker/days/{day}",
    params(("day" = String, Path, description = "Strict valid YYYY-MM-DD date the app counts the entries toward")),
    request_body = OpenNutriTrackerDayRequest,
    responses(
        (status = 204, description = "Both lists stored (or already identical); statistics created on first sync"),
        (status = 400, description = "Malformed JSON or unreadable body", body = DayErrorEnvelope),
        (status = 409, description = "Duplicate, wrongly typed or non-mg tagged statistics", body = DayErrorEnvelope),
        (status = 415, description = "Unsupported media type", body = DayErrorEnvelope),
        (status = 422, description = "Invalid date, field or daily total", body = DayErrorEnvelope),
        (status = 500, description = "Storage failed without committing", body = DayErrorEnvelope)
    )
)]
pub async fn replace_day<R, Rend, E, I>(
    State(state): State<AppState<R, Rend, E, I>>,
    Path(day): Path<String>,
    body: Result<Json<Box<RawValue>>, JsonRejection>,
) -> Result<StatusCode, DayApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    let Json(value) = body.map_err(DayApiError::json)?;
    let values = OpenNutriTrackerDayRequest::parse(&value)?;
    let day = super::integration_error::parse_day(&day)?;
    let result = state
        .integration_service
        .replace_opennutritracker_day(day, values)?;
    for id in result.changed_statistics {
        state
            .event_publisher
            .publish_event_async(Event::ValueUpdated(id));
    }
    Ok(StatusCode::NO_CONTENT)
}
