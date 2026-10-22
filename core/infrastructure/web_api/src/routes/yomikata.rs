use crate::state::AppState;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::{
    statistic::ports::statistic_repository::StatisticRepository,
    yomikata::{YomikataDayError, YomikataDayTotals},
};
use plot_domain::plot::renderer::PlotRenderer;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct YomikataDayRequest {
    #[schema(minimum = 0, maximum = 9223372036854775807_i64)]
    pub cards_reviewed: i64,
    /// Exact engaged seconds. Existing study_minutes projects seconds / 60.
    #[schema(minimum = 0, maximum = 9223372036854775807_i64)]
    pub study_time_seconds: i64,
}
impl YomikataDayRequest {
    fn parse(value: &RawValue) -> Result<Self, DayApiError> {
        let object: BTreeMap<String, Box<RawValue>> = serde_json::from_str(value.get())
            .map_err(|_| DayApiError::invalid(None, "Expected a JSON object"))?;
        let integer = |field: &'static str| {
            object
                .get(field)
                .and_then(|value| serde_json::from_str::<i64>(value.get()).ok())
                .filter(|value| *value >= 0)
                .ok_or_else(|| {
                    DayApiError::invalid(
                        Some(field),
                        "Required nonnegative integer, at most 9223372036854775807",
                    )
                })
        };
        integer("cards_reviewed")?;
        integer("study_time_seconds")?;
        serde_json::from_str(value.get())
            .map_err(|_| DayApiError::invalid(None, "Unexpected or duplicate request fields"))
    }
}

pub use super::integration_error::{
    ErrorBody as DayErrorBody, ErrorCode as DayErrorCode, ErrorEnvelope as DayErrorEnvelope,
};

pub struct DayApiError {
    status: StatusCode,
    body: DayErrorEnvelope,
}
impl DayApiError {
    fn new(code: DayErrorCode, field: Option<&str>, message: &str) -> Self {
        Self {
            status: code.status(),
            body: DayErrorEnvelope {
                error: DayErrorBody {
                    code,
                    field: field.map(str::to_owned),
                    message: message.into(),
                },
            },
        }
    }
    fn invalid(field: Option<&str>, message: &str) -> Self {
        Self::new(DayErrorCode::InvalidField, field, message)
    }
    fn json(error: JsonRejection) -> Self {
        match error {
            JsonRejection::MissingJsonContentType(_) => Self::new(
                DayErrorCode::UnsupportedMediaType,
                None,
                "Content-Type must be application/json",
            ),
            JsonRejection::JsonSyntaxError(_) => {
                Self::new(DayErrorCode::MalformedJson, None, "Malformed JSON")
            }
            JsonRejection::JsonDataError(_) => Self::invalid(None, "Invalid JSON data"),
            _ => Self::new(
                DayErrorCode::InvalidBody,
                None,
                "Unable to read request body",
            ),
        }
    }
}
impl IntoResponse for DayApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
impl From<YomikataDayError> for DayApiError {
    fn from(error: YomikataDayError) -> Self {
        match error {
            YomikataDayError::InvalidTotal { field } => {
                Self::invalid(Some(field), "Required nonnegative integer")
            }
            YomikataDayError::ProjectionOverflow => Self::new(
                DayErrorCode::ProjectionOverflow,
                None,
                "Resulting aggregate exceeds the supported integer range",
            ),
            YomikataDayError::BundleConflict => Self::new(
                DayErrorCode::BundleConflict,
                None,
                "Yomikata statistics or legacy values require correction",
            ),
            YomikataDayError::Storage(error) => {
                tracing::error!(error=?error,"Failed to replace Yomikata daily totals");
                Self::new(
                    DayErrorCode::StorageError,
                    None,
                    "Unable to store daily totals",
                )
            }
        }
    }
}

#[utoipa::path(
    put,
    path = "/integrations/yomikata/days/{day}",
    params(("day" = String, Path, description = "Strict valid YYYY-MM-DD calendar date")),
    request_body = YomikataDayRequest,
    responses(
        (status = 204, description = "Daily totals and both projections committed"),
        (status = 400, description = "Malformed JSON or unreadable body", body = DayErrorEnvelope),
        (status = 409, description = "Ambiguous or invalid legacy bundle", body = DayErrorEnvelope),
        (status = 415, description = "Unsupported media type", body = DayErrorEnvelope),
        (status = 422, description = "Invalid date, field, or aggregate overflow", body = DayErrorEnvelope),
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
    let request = YomikataDayRequest::parse(&value)?;
    let parsed = NaiveDate::parse_from_str(&day, "%Y-%m-%d")
        .ok()
        .filter(|date| date.format("%Y-%m-%d").to_string() == day && day.len() == 10);
    let day = parsed.ok_or_else(|| {
        DayApiError::invalid(Some("day"), "Expected a valid YYYY-MM-DD calendar date")
    })?;
    let totals = YomikataDayTotals::new(request.cards_reviewed, request.study_time_seconds)?;
    let result = state
        .integration_service
        .replace_yomikata_day(day, totals)?;
    for id in result.changed_statistics {
        state
            .event_publisher
            .publish_event_async(Event::ValueUpdated(id));
    }
    Ok(StatusCode::NO_CONTENT)
}
