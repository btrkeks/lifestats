use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, Query},
    http::StatusCode,
    Extension, Json,
};
use lifestats_application::{
    integrations::fitbit::ports::{FitbitOAuthApi, TokenStorePort},
    use_cases::commands::sync_fitbit_data::{SyncFitbitDataCommand, SyncFitbitDataCommandHandler},
};

use super::{
    config::FitbitConfig,
    pkce::generate_pkce_pair,
    state_cache::OAuthStateCache,
    types::{AuthBeginResponse, AuthCallbackQuery, OAuthState, SyncRequestDto},
};
use crate::error::ApiError;

const FITBIT_SOURCE: &str = "fitbit";

/// GET /integrations/fitbit/auth/begin
///
/// Initiates the OAuth2 PKCE flow by:
/// 1. Generating a unique state identifier
/// 2. Generating PKCE code verifier and challenge
/// 3. Caching the state and verifier
/// 4. Returning the Fitbit authorization URL
#[tracing::instrument(skip_all)]
pub async fn auth_begin_handler(
    Extension(config): Extension<Arc<FitbitConfig>>,
    Extension(cache): Extension<Arc<OAuthStateCache>>,
) -> Result<Json<AuthBeginResponse>, ApiError> {
    // Generate unique state identifier (UUID v4 provides 128-bit entropy)
    let state = uuid::Uuid::new_v4().to_string();

    // Generate PKCE pair (RFC 7636)
    let (code_verifier, code_challenge) = generate_pkce_pair();

    // Create OAuth state entry with hardcoded user ID for single-user deployment
    let user_id = "default_user".to_string();
    let oauth_state = OAuthState::new(state.clone(), code_verifier, Some(user_id));

    // Cache the state with 5-minute TTL
    cache.insert(state.clone(), oauth_state).await;

    // Build the full authorization URL
    let authorization_url = config
        .build_authorize_url(&state, &code_challenge)
        .map_err(|e| ApiError::InternalError(format!("Failed to build OAuth URL: {}", e)))?;

    tracing::info!(
        state = %state,
        "OAuth flow initiated, redirecting to Fitbit"
    );

    Ok(Json(AuthBeginResponse { authorization_url }))
}

/// GET /integrations/fitbit/auth/callback
///
/// Handles the OAuth2 callback from Fitbit:
/// 1. Validates and consumes the state (prevents replay attacks)
/// 2. Verifies user session binding
/// 3. Exchanges authorization code for access token using PKCE verifier
/// 4. Saves the token to persistent storage
#[tracing::instrument(skip_all, fields(state = %query.state))]
pub async fn auth_callback_handler(
    Query(query): Query<AuthCallbackQuery>,
    Extension(cache): Extension<Arc<OAuthStateCache>>,
    Extension(oauth_api): Extension<Arc<dyn FitbitOAuthApi>>,
    Extension(token_store): Extension<Arc<dyn TokenStorePort>>,
) -> Result<StatusCode, ApiError> {
    // Check for OAuth errors from Fitbit
    if let Some(error) = query.error {
        tracing::warn!(
            error = %error,
            description = ?query.error_description,
            "OAuth authorization failed"
        );
        return Err(ApiError::BadRequest(format!(
            "OAuth authorization failed: {}",
            error
        )));
    }

    // Consume the state (atomic get-and-delete)
    let oauth_state = cache.consume(&query.state).await.ok_or_else(|| {
        tracing::error!(state = %query.state, "Invalid or expired OAuth state");
        ApiError::BadRequest(
            "Invalid or expired state. Please restart the authorization flow.".to_string(),
        )
    })?;

    // Verify user session binding (critical security check)
    // In a single-user deployment, we validate against the hardcoded user ID
    let current_user_id = "default_user";
    if let Some(ref stored_user_id) = oauth_state.user_id {
        if stored_user_id != current_user_id {
            tracing::error!(
                state = %query.state,
                stored_user = %stored_user_id,
                current_user = %current_user_id,
                "User ID mismatch - potential session fixation attack"
            );
            return Err(ApiError::Unauthorized(
                "Session mismatch. Please restart the authorization flow.".to_string(),
            ));
        }
    } else {
        tracing::warn!(
            state = %query.state,
            "OAuth state missing user_id - legacy state or migration scenario"
        );
    }

    tracing::debug!(
        state = %query.state,
        user_id = %current_user_id,
        age_seconds = (chrono::Utc::now() - oauth_state.created_at).num_seconds(),
        "OAuth state validated and consumed"
    );

    // Exchange authorization code for access token
    let token_data = oauth_api
        .exchange_code_for_token(&query.code, Some(oauth_state.pkce_verifier))
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to exchange code for token");
            e
        })?;

    tracing::info!("Successfully exchanged authorization code for token");

    // Save token to persistent storage
    token_store
        .save(FITBIT_SOURCE, &token_data)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to save token");
            e
        })?;

    tracing::info!("Fitbit token saved successfully");

    Ok(StatusCode::OK)
}

/// POST /integrations/fitbit/sync
///
/// Triggers a Fitbit data synchronization:
/// 1. Validates the date range
/// 2. Executes the sync command
/// 3. Returns 204 No Content on success
#[tracing::instrument(skip_all)]
pub async fn sync_handler(
    Extension(sync_handler): Extension<Arc<SyncFitbitDataCommandHandler>>,
    json_result: Result<Json<SyncRequestDto>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    // Handle JSON deserialization errors with detailed logging
    let Json(request) = json_result.map_err(|rejection| {
        tracing::error!(
            error = %rejection,
            status = ?rejection.status(),
            body = ?rejection.body_text(),
            "Failed to deserialize sync request JSON"
        );
        ApiError::BadRequest(format!(
            "Invalid request format: {}. Expected JSON with 'since' and 'until' fields in YYYY-MM-DD format.",
            rejection.body_text()
        ))
    })?;

    tracing::info!(
        since = %request.since,
        until = %request.until,
        "Received Fitbit sync request"
    );

    // Validate date range
    if request.since > request.until {
        tracing::warn!(
            since = %request.since,
            until = %request.until,
            "Invalid date range: since is after until"
        );
        return Err(ApiError::BadRequest(
            "since must be on or before until".to_string(),
        ));
    }

    tracing::info!(
        since = %request.since,
        until = %request.until,
        "Starting Fitbit data sync"
    );

    // Create and execute the sync command
    let command = SyncFitbitDataCommand {
        since: request.since,
        until: request.until,
    };

    sync_handler.handle(command).await.map_err(|e| {
        tracing::error!(error = ?e, "Fitbit sync failed");
        e
    })?;

    tracing::info!(
        since = %request.since,
        until = %request.until,
        "Fitbit data sync completed successfully"
    );

    Ok(StatusCode::NO_CONTENT)
}
