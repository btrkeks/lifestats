use chrono::NaiveDate;
use crate::shared::time_frame_list::TimeFrameError;

#[derive(Debug, thiserror::Error)]
pub enum StatisticError {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Goal not found for date {0}")]
    GoalNotFound(NaiveDate),

    #[error("Time frame error: {0}")]
    TimeFrame(#[from] TimeFrameError),

    #[error("Update policy error: {0}")]
    UpdatePolicy(String),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

// Create a type alias
pub type Result<T> = std::result::Result<T, StatisticError>;