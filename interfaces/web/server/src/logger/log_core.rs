use api_client::types::{LogRequest, LogResponse};
use thiserror::Error;
use backend::statistics::statistic_repository::get_statistic_by_id;
use backend::statistics::type_definitions::Statistic;
use chrono::NaiveDate;
use backend::database::db_ops;

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
    log_data: LogRequest,
) -> Result<LogResponse, LoggingError> {
    debug!("Processing log request: {:?}", log_data);

    let statistic = get_statistic_by_id(log_data.statistic_id)
        .map_err(|e| {
            error!("Failed to retrieve statistic {}: {}", log_data.statistic_id, e);
            LoggingError::InternalError(e.to_string())
        })?
        .ok_or_else(|| {
            error!("Statistic not found: {}", log_data.statistic_id);
            LoggingError::IDNotFound(log_data.statistic_id.to_string())
        })?;

    let date = NaiveDate::from_ymd_opt(
        log_data.date.year(),
        log_data.date.month() as u32,
        log_data.date.day() as u32,
    ).ok_or_else(|| LoggingError::ValidationError("Invalid date".to_string()))?;

    let conn = db_ops::open_db().map_err(|e| LoggingError::DatabaseError(e.to_string()))?;

    statistic
        .log_from_string(
            &conn,
            date,
            &log_data.value,
            log_data.update_policy.as_deref(),
        )
        .map_err(|e| LoggingError::InternalError(e.to_string()))?;

    Ok(LogResponse::success(format!(
        "Successfully logged {} for {} on {}",
        log_data.value,
        statistic.get_name(),
        date.format("%Y-%m-%d")
    )))
}