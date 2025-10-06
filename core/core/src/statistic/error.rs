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

// Create type alias
pub type DomainResult<T> = std::result::Result<T, DomainError>;
