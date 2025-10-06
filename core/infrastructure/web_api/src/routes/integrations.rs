use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use events::event_port::EventPublisher;
use lifestats_application::error::ApplicationError;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

use crate::state::AppState;

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

/// POST /integrations/:source
///
/// Provisions a bundle of statistics for a given integration source.
/// This endpoint is idempotent: calling it multiple times with the same
/// source and context will return the same statistic IDs.
pub async fn provision_integration<R, P, Rend, E>(
    State(state): State<AppState<R, P, Rend, E>>,
    Path(source): Path<String>,
    Json(request): Json<ProvisionBundleRequest>,
) -> Result<Json<ProvisionBundleResponse>, (StatusCode, String)>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    // Call the integration service to provision the bundle
    let result = state
        .integration_service
        .provision_bundle(&source, request.context)
        .map_err(|e| match e {
            ApplicationError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            ApplicationError::Domain(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ApplicationError::Repository(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            ApplicationError::Infrastructure(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
        })?;

    // Convert StatisticId to u32 for JSON response
    let statistic_ids: HashMap<String, u32> =
        result.into_iter().map(|(key, id)| (key, id.0)).collect();

    Ok(Json(ProvisionBundleResponse { statistic_ids }))
}
