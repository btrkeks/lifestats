use lifestats_core::statistic::error::DomainError;
use lifestats_core::statistic::ports::error::RepositoryError;
use thiserror::Error;

/// Application-level errors that classify errors from lower layers
///
/// This enum enables the outer layers (like the Web API) to translate
/// errors into appropriate responses (e.g., 400 vs 500 status codes).
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// A business rule was violated (maps to 400 Bad Request)
    #[error("A business rule was violated: {0}")]
    Domain(#[from] DomainError),

    /// A persistence error occurred
    #[error("A persistence error occurred: {0}")]
    Repository(#[from] RepositoryError),

    /// An unexpected infrastructure error occurred (maps to 500 Internal Server Error)
    #[error("An unexpected infrastructure error occurred: {0}")]
    Infrastructure(#[from] anyhow::Error),
}

/// Type alias for Results that use ApplicationError
pub type ApplicationResult<T> = Result<T, ApplicationError>;
