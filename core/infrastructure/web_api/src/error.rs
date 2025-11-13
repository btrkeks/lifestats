use std::time::Duration as StdDuration;

use axum::{
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Centralized API error handling
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    ValidationError(String),
    Conflict(String),
    InternalError(String),
    InternalServerError(anyhow::Error),
    BadGateway(String),
    RateLimited {
        message: String,
        retry_after: Option<StdDuration>,
    },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::ValidationError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": msg })),
            )
                .into_response(),
            ApiError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::Conflict(msg) => {
                (StatusCode::CONFLICT, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::InternalError(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": msg })),
                )
                    .into_response()
            }
            ApiError::InternalServerError(err) => {
                tracing::error!("Internal server error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "Internal server error" })),
                )
                    .into_response()
            }
            ApiError::BadGateway(msg) => {
                (StatusCode::BAD_GATEWAY, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::RateLimited {
                message,
                retry_after,
            } => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({ "error": message })),
                )
                    .into_response();
                if let Some(duration) = retry_after {
                    if let Ok(value) = HeaderValue::from_str(&duration.as_secs().to_string()) {
                        response.headers_mut().insert(RETRY_AFTER, value);
                    }
                }
                response
            }
        }
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

impl From<lifestats_application::SessionError> for ApiError {
    fn from(err: lifestats_application::SessionError) -> Self {
        use lifestats_application::SessionError;

        match err {
            SessionError::SessionAlreadyExists(id) => {
                ApiError::Conflict(format!("Session already running for statistic {}", id.0))
            }
            SessionError::SessionNotFound(id) => {
                ApiError::NotFound(format!("No active session for statistic {}", id.0))
            }
            SessionError::StatisticNotFound(id) => {
                ApiError::NotFound(format!("Statistic {} not found", id.0))
            }
            SessionError::InvalidConfiguration(msg) => ApiError::BadRequest(msg),
        }
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

            // Invalid input from user → 400 Bad Request
            ApplicationError::InvalidInput(msg) => ApiError::BadRequest(msg),

            // Repository NotFound → 404 Not Found
            ApplicationError::Repository(RepositoryError::NotFound) => {
                ApiError::NotFound("The requested resource was not found.".to_string())
            }

            // Other repository errors → 500 Internal Server Error
            ApplicationError::Repository(err) => ApiError::InternalServerError(err.into()),

            // Infrastructure errors → 500 Internal Server Error
            ApplicationError::Infrastructure(err) => ApiError::InternalServerError(err),

            ApplicationError::FitbitSyncFailed(upstream) => match upstream {
                lifestats_application::integrations::fitbit::error::FitbitUpstreamError::RateLimited {
                    retry_after,
                } => ApiError::RateLimited {
                    message: "Fitbit rate limited the request".to_string(),
                    retry_after,
                },
                lifestats_application::integrations::fitbit::error::FitbitUpstreamError::InvalidGrant => {
                    ApiError::Unauthorized(
                        "Invalid grant. Please re-authorize the Fitbit integration.".to_string(),
                    )
                }
                lifestats_application::integrations::fitbit::error::FitbitUpstreamError::Unauthorized => {
                    ApiError::BadGateway(
                        "Fitbit authentication failed. Token may have been revoked.".to_string(),
                    )
                }
                other => ApiError::BadGateway(other.to_string()),
            },
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

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

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
