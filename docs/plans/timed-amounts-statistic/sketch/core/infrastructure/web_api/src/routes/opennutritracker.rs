//! NEW FILE. Strict HTTP adapter for PUT /integrations/opennutritracker/days/{day}.
//! Pattern: routes/beurer.rs (raw-token validation BEFORE typed parsing, so the
//! field-precise errors in contract.md are reachable).
use axum::{extract::{rejection::JsonRejection, Path, State}, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::value::RawValue;
use utoipa::ToSchema;
use lifestats_core::opennutritracker::{OpenNutriTrackerDay, OpenNutriTrackerDayError};
use super::integration_error::IntegrationApiError;

/// HTTP-only mg vocabulary; never exported to application/domain callers.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimedMilligramsRequest {
    /// Exact HH:MM:SS local clock from the source entry, not server receipt time.
    /// `null` = day known, time unknown. The key is REQUIRED (missing -> 422); the
    /// OpenAPI schema must say required + nullable, which `Option` alone does not.
    #[schema(required = true, nullable = true)]
    pub time: Option<String>,
    #[schema(minimum=1, maximum=9007199254740991_i64)]
    pub amount_mg: i64,
}
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenNutriTrackerDayRequest {
    #[schema(max_items=1000)]
    pub caffeine: Vec<TimedMilligramsRequest>,
    #[schema(max_items=1000)]
    pub nicotine: Vec<TimedMilligramsRequest>,
}
impl OpenNutriTrackerDayRequest {
    /// Preserve raw integer lexemes down to EACH entry. No serde_json::Value
    /// round-trip before integer checks. Strict struct pass rejects duplicates,
    /// including escaped names; a BTreeMap parse alone is not sufficient.
    fn parse(raw: &RawValue) -> Result<Self, IntegrationApiError> {
        // TODO: validate root structure/duplicates, and typed raw arrays/entry objects.
        // Apply contract.md's validation order; reject floats/exponents/unknowns/nulls.
        // One private raw visitor/helper may own path-aware error reporting.
        todo!()
    }
    fn into_day(self) -> Result<OpenNutriTrackerDay, IntegrationApiError> {
        // TODO: exact clock strings -> TimedAmount::new; TimedAmountList::new.
        // Attach caffeine[i].time / .amount_mg paths; total error => ProjectionOverflow.
        // Domain constructors own invariant rules; adapter owns fields/messages.
        todo!()
    }
}
impl From<OpenNutriTrackerDayError> for IntegrationApiError {
    fn from(error: OpenNutriTrackerDayError) -> Self {
        // TODO: BundleConflict ->409 bundle_conflict; Storage ->500 storage_error.
        // Sanitize public messages exactly as contract.md; log storage details only.
        todo!()
    }
}
/// utoipa::path PUT /integrations/opennutritracker/days/{day}.
/// Responses 204,400,409,415,422,500 and ErrorEnvelope per contract.md.
pub async fn replace_day<R, Rend, E, I>(
    State(state): State<crate::state::AppState<R,Rend,E,I>>,
    Path(day): Path<String>,
    body: Result<Json<Box<RawValue>>,JsonRejection>,
) -> Result<StatusCode,IntegrationApiError>
where
    R: lifestats_core::statistic::ports::statistic_repository::StatisticRepository + Send + Sync + 'static,
    Rend: plot_domain::plot::renderer::PlotRenderer + 'static,
    E: events::event_port::EventPublisher + 'static,
    I: lifestats_application::use_cases::integration::IntegrationServiceTrait + 'static,
{
    // TODO: map JsonRejection via shared json(); validate root, parse_day(&day),
    // parse/convert all entries; service.replace_opennutritracker_day(day,values).
    // Publish Event::ValueUpdated for changed_statistics AFTER successful commit.
    // Return NO_CONTENT, including unchanged snapshots; never serialize a JSON body.
    todo!()
}
