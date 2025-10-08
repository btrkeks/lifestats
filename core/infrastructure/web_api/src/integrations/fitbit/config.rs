/// Configuration for Fitbit OAuth2 integration
#[derive(Debug, Clone)]
pub struct FitbitConfig {
    /// OAuth2 client ID from Fitbit application settings
    pub client_id: String,

    /// OAuth2 client secret (sensitive - do not log)
    pub client_secret: String,

    /// Redirect URI registered with Fitbit
    pub redirect_uri: String,

    /// Required OAuth scopes (e.g., ["activity", "sleep"])
    pub scopes: Vec<String>,

    /// Fitbit OAuth authorization endpoint
    pub authorize_url: String,
}

impl FitbitConfig {
    /// Create a new FitbitConfig with validation
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            scopes,
            authorize_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
        }
    }

    /// Build the full authorization URL with state and PKCE challenge
    pub fn build_authorize_url(
        &self,
        state: &str,
        code_challenge: &str,
    ) -> Result<String, url::ParseError> {
        let mut url = url::Url::parse(&self.authorize_url)?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");

        Ok(url.to_string())
    }
}
