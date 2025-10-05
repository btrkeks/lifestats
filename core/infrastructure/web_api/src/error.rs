use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Centralized API error handling
#[derive(Debug)]
pub enum ApiError {
    // NotFound(String),
    BadRequest(String),
    ValidationError(String),
    InternalError(String),
    InternalServerError(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            // ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            ApiError::InternalServerError(err) => {
                tracing::error!("Internal server error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalServerError(err)
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errors: validator::ValidationErrors) -> Self {
        ApiError::ValidationError(errors.to_string())
    }
}

impl From<lifestats_domain::statistic::error::DomainError> for ApiError {
    fn from(err: lifestats_domain::statistic::error::DomainError) -> Self {
        use lifestats_domain::statistic::error::DomainError;

        match err {
            // Map domain validation and parsing errors to 400 Bad Request
            DomainError::ValidationError(msg) => ApiError::BadRequest(msg),
            DomainError::ParsingError(msg) => ApiError::BadRequest(msg),
            DomainError::UpdatePolicyError(msg) => ApiError::BadRequest(msg),
            DomainError::InvalidOperation(msg) => ApiError::BadRequest(msg),
            DomainError::TimeFrame(e) => ApiError::BadRequest(e.to_string()),
            DomainError::GoalNotFound(date) => {
                ApiError::BadRequest(format!("Goal not found for date {}", date))
            }
        }
    }
}

impl From<lifestats_domain::statistic::error::StatisticError> for ApiError {
    fn from(err: lifestats_domain::statistic::error::StatisticError) -> Self {
        use lifestats_domain::statistic::error::StatisticError;

        match err {
            // Map domain errors to appropriate API errors
            StatisticError::Domain(domain_err) => domain_err.into(),

            // Map other errors
            StatisticError::InvalidValue(msg) => ApiError::BadRequest(msg),
            StatisticError::InvalidOperation(msg) => ApiError::BadRequest(msg),
            StatisticError::UpdatePolicy(msg) => ApiError::BadRequest(msg),
            StatisticError::TimeFrame(e) => ApiError::BadRequest(e.to_string()),
            StatisticError::GoalNotFound(date) => {
                ApiError::BadRequest(format!("Goal not found for date {}", date))
            }

            // Infrastructure errors should be 500
            StatisticError::Repository(msg) => ApiError::InternalError(msg),
            StatisticError::General(err) => ApiError::InternalServerError(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_validation_error_response() {
        let error = ApiError::ValidationError("Invalid input".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("Invalid input"));
        assert!(body_str.contains("error"));
    }

    #[tokio::test]
    async fn test_internal_server_error_response() {
        let error = ApiError::InternalServerError(anyhow::anyhow!("Database connection failed"));
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains("Internal server error"));
        assert!(body_str.contains("error"));
        // Should NOT leak internal error details
        assert!(!body_str.contains("Database connection failed"));
    }
}
