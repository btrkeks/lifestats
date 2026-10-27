use crate::state::AppState;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    Json,
};
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::{
    beurer::{BeurerDayError, BeurerDayReading},
    statistic::ports::statistic_repository::StatisticRepository,
};
use plot_domain::plot::renderer::PlotRenderer;
use serde::Deserialize;
use serde_json::value::RawValue;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BeurerDayRequest {
    #[schema(minimum = 1, maximum = 500000)]
    pub weight_grams: i64,
    #[schema(minimum = 0, maximum = 1000)]
    pub body_fat_permille: Option<i64>,
    #[schema(minimum = 0, maximum = 1000)]
    pub muscle_permille: Option<i64>,
    #[schema(minimum = 0, maximum = 1000)]
    pub body_water_permille: Option<i64>,
}

impl BeurerDayRequest {
    fn parse(value: &RawValue) -> Result<Self, DayApiError> {
        let object = super::integration_error::parse_strict_object(value)?;
        for field in [
            "weight_grams",
            "body_fat_permille",
            "muscle_permille",
            "body_water_permille",
        ] {
            let raw = object.get(field).map(|value| value.get());
            if field != "weight_grams" && matches!(raw, None | Some("null")) {
                continue;
            }
            if raw
                .and_then(|raw| serde_json::from_str::<i64>(raw).ok())
                .is_none()
            {
                return Err(DayApiError::invalid(
                    Some(field),
                    "Expected an integer in the supported range",
                ));
            }
        }
        serde_json::from_str(value.get())
            .map_err(|_| DayApiError::invalid(None, "Unexpected or duplicate request fields"))
    }
}

pub use super::integration_error::{
    ErrorCode as DayErrorCode, ErrorEnvelope as DayErrorEnvelope,
    IntegrationApiError as DayApiError,
};

impl From<BeurerDayError> for DayApiError {
    fn from(error: BeurerDayError) -> Self {
        match error {
            BeurerDayError::InvalidField { field } => {
                Self::invalid(Some(field), "Integer is outside the supported range")
            }
            BeurerDayError::BundleConflict => Self::new(
                DayErrorCode::BundleConflict,
                None,
                "Beurer statistics require correction",
            ),
            BeurerDayError::Storage(error) => {
                tracing::error!(error=?error,"Failed to replace Beurer daily totals");
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
    path = "/integrations/beurer/days/{day}",
    params(("day" = String, Path, description = "Strict valid YYYY-MM-DD calendar date")),
    request_body = BeurerDayRequest,
    responses(
        (status = 204, description = "Daily readings and all four statistics committed"),
        (status = 400, description = "Malformed JSON or unreadable body", body = DayErrorEnvelope),
        (status = 409, description = "Duplicate or wrongly typed tagged statistics", body = DayErrorEnvelope),
        (status = 415, description = "Unsupported media type", body = DayErrorEnvelope),
        (status = 422, description = "Invalid date or field", body = DayErrorEnvelope),
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
    let request = BeurerDayRequest::parse(&value)?;
    let day = super::integration_error::parse_day(&day)?;
    let reading = BeurerDayReading::new(
        request.weight_grams,
        request.body_fat_permille,
        request.muscle_permille,
        request.body_water_permille,
    )?;
    let result = state.integration_service.replace_beurer_day(day, reading)?;
    for id in result.changed_statistics {
        state
            .event_publisher
            .publish_event_async(Event::ValueUpdated(id));
    }
    Ok(StatusCode::NO_CONTENT)
}
