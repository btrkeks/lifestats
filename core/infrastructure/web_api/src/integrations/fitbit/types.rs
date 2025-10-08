use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// OAuth state stored in cache during authorization flow
/// This entry is consumed (removed) once the callback is processed
#[derive(Debug, Clone)]
pub struct OAuthState {
    /// Unique state identifier (UUID v4)
    pub state: String,

    /// PKCE code verifier (must be kept secret until callback)
    pub pkce_verifier: String,

    /// User ID (optional, for future multi-tenancy support)
    pub user_id: Option<String>,

    /// When this state was created (for debugging expired states)
    pub created_at: DateTime<Utc>,
}

impl OAuthState {
    pub fn new(state: String, pkce_verifier: String, user_id: Option<String>) -> Self {
        Self {
            state,
            pkce_verifier,
            user_id,
            created_at: Utc::now(),
        }
    }
}

/// Response for GET /integrations/fitbit/auth/begin
#[derive(Debug, Serialize)]
pub struct AuthBeginResponse {
    /// Full Fitbit authorization URL to redirect user to
    pub authorization_url: String,
}

/// Query parameters for GET /integrations/fitbit/auth/callback
#[derive(Debug, Deserialize)]
pub struct AuthCallbackQuery {
    /// Authorization code from Fitbit
    pub code: String,

    /// State parameter to match against cached value
    pub state: String,

    /// Optional error from Fitbit (e.g., user denied)
    #[serde(default)]
    pub error: Option<String>,

    /// Optional error description
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Request body for POST /integrations/fitbit/sync
#[derive(Debug, Deserialize)]
pub struct SyncRequestDto {
    /// Start date (inclusive)
    pub since: NaiveDate,

    /// End date (inclusive)
    pub until: NaiveDate,
}
