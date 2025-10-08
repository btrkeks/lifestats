use std::time::Duration;

use anyhow::Error;
use thiserror::Error;

use crate::error::ApplicationError;

#[derive(Debug, Error)]
pub enum FitbitUpstreamError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited")]
    RateLimited { retry_after: Option<Duration> },
    #[error("invalid grant")]
    InvalidGrant,
    #[error("upstream service error: {0}")]
    UpstreamServiceError(Error),
}

impl From<FitbitUpstreamError> for ApplicationError {
    fn from(error: FitbitUpstreamError) -> Self {
        ApplicationError::FitbitSyncFailed(error)
    }
}
