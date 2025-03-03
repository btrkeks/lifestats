use crate::guards::Context;
use chrono::NaiveDate;
use lifestats_core::app_context::{build_application, AppContext};
use thiserror::Error;
use web_api::types::{LogRequest, LogResponse};

#[derive(Error, Debug)]
pub enum LoggingError {
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Statistic with id '{0}' not found.")]
    IDNotFound(String),
}

pub async fn log_statistic(
    app_context: &AppContext,
    log_data: LogRequest,
) -> Result<LogResponse, LoggingError> {
    debug!("Processing log request: {:?}", log_data);

    // FIXME: This is retarded
    let date = NaiveDate::from_ymd_opt(
        log_data.date.year(),
        log_data.date.month() as u32,
        log_data.date.day() as u32,
    )
    .ok_or_else(|| LoggingError::ValidationError("Invalid date".to_string()))?;

    lifestats_core::log::log_statistic(
        &app_context,
        log_data.statistic_id,
        date,
        &log_data.value,
        log_data.update_policy.as_deref(),
    )
    .map_err(|e| LoggingError::InternalError(e.to_string()))?;

    Ok(LogResponse::success(format!(
        "Successfully logged {} for id {} on {}",
        log_data.value,
        log_data.statistic_id,
        date.format("%Y-%m-%d")
    )))
}
