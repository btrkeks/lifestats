use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroize;

#[derive(Clone)]
pub struct AccessToken(SecretString);

impl AccessToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(SecretString::new(token.into().into_boxed_str()))
    }

    pub fn as_str(&self) -> &str {
        self.0.expose_secret()
    }
}

#[derive(Clone)]
pub struct RefreshToken(SecretString);

impl RefreshToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(SecretString::new(token.into().into_boxed_str()))
    }

    pub fn as_str(&self) -> &str {
        self.0.expose_secret()
    }
}

#[derive(Clone)]
pub struct FitbitTokenData {
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

impl Drop for FitbitTokenData {
    fn drop(&mut self) {
        self.scopes.zeroize();
    }
}
