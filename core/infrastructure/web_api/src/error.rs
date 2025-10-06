use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Centralized API error handling
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    ValidationError(String),
    InternalError(String),
    InternalServerError(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
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

impl From<lifestats_application::error::ApplicationError> for ApiError {
    fn from(err: lifestats_application::error::ApplicationError) -> Self {
        use lifestats_application::error::ApplicationError;
        use lifestats_core::statistic::error::DomainError;
        use lifestats_core::statistic::ports::error::RepositoryError;

        match err {
            // Domain errors are user-fixable → 400 Bad Request
            ApplicationError::Domain(domain_err) => match domain_err {
                DomainError::ValidationError(msg) => ApiError::BadRequest(msg),
                DomainError::ParsingError(msg) => ApiError::BadRequest(msg),
                DomainError::UpdatePolicyError(msg) => ApiError::BadRequest(msg),
                DomainError::InvalidOperation(msg) => ApiError::BadRequest(msg),
                DomainError::TimeFrame(e) => ApiError::BadRequest(e.to_string()),
                DomainError::GoalNotFound(date) => {
                    ApiError::BadRequest(format!("Goal not found for date {}", date))
                }
            },

            // Repository NotFound → 404 Not Found
            ApplicationError::Repository(RepositoryError::NotFound) => {
                ApiError::NotFound("The requested resource was not found.".to_string())
            }

            // Other repository errors → 500 Internal Server Error
            ApplicationError::Repository(err) => ApiError::InternalServerError(err.into()),

            // Infrastructure errors → 500 Internal Server Error
            ApplicationError::Infrastructure(err) => ApiError::InternalServerError(err),
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
