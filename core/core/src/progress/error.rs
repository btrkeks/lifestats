use crate::statistic::error::DomainError;

pub type ProgressResult<T> = Result<T, DomainError>;

pub(crate) fn validation_error(message: impl Into<String>) -> DomainError {
    DomainError::ValidationError(message.into())
}
