use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

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

/// POST /integrations/:source
///
/// Provisions a bundle of statistics for a given integration source.
/// This endpoint is idempotent: calling it multiple times with the same
/// source and context will return the same statistic IDs.
pub async fn provision_integration<R, P, Rend, E, I>(
    State(state): State<AppState<R, P, Rend, E, I>>,
    Path(source): Path<String>,
    Json(request): Json<ProvisionBundleRequest>,
) -> Result<Json<ProvisionBundleResponse>, ApiError>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
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
