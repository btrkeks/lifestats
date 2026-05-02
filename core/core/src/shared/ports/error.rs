use thiserror::Error;

/// Repository-level errors for persistence operations.
///
/// This error type is part of the public contract for all persistence ports.
/// It decouples the application from specific persistence implementation errors.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// The requested resource was not found.
    #[error("Resource not found")]
    NotFound,

    /// A persistence operation failed.
    #[error("Operation failed: {0}")]
    OperationFailed(#[from] anyhow::Error),
}
