use crate::shared::time_frame_list::TimeFrameError;
use chrono::NaiveDate;

/// Domain-level errors that represent business rule violations
///
/// These errors help distinguish between client errors (user input problems)
/// and server errors (infrastructure/unexpected issues).
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Validation failed for user input
    #[error("Validation failed: {0}")]
    ValidationError(String),

    /// Could not parse value from string
    #[error("Could not parse value: {0}")]
    ParsingError(String),

    /// Invalid operation attempted
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Goal not found for specific date
    #[error("Goal not found for date {0}")]
    GoalNotFound(NaiveDate),

    /// Time frame specific error
    #[error("Time frame error: {0}")]
    TimeFrame(#[from] TimeFrameError),

    /// Update policy parsing error
    #[error("Update policy error: {0}")]
    UpdatePolicyError(String),
}

/// Legacy error type for backward compatibility during migration
#[derive(Debug, thiserror::Error)]
pub enum StatisticError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Goal not found for date {0}")]
    GoalNotFound(NaiveDate),

    #[error("Time frame error: {0}")]
    TimeFrame(#[from] TimeFrameError),

    #[error("Update policy error: {0}")]
    UpdatePolicy(String),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

// Create type aliases
pub type Result<T> = std::result::Result<T, StatisticError>;
pub type DomainResult<T> = std::result::Result<T, DomainError>;
