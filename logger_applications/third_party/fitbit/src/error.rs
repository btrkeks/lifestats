use thiserror::Error;

#[derive(Error, Debug)]
pub enum FitbitError {
    #[error("Request failed: {0}")]
    RequestError(#[from] ureq::Error),

    #[error("JSON parsing failed")]
    JsonError(String),
}
