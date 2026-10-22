use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidField,
    UnsupportedMediaType,
    MalformedJson,
    InvalidBody,
    ProjectionOverflow,
    BundleConflict,
    StorageError,
    LegacyBindingConflict,
    LegacyAdoptionPending,
    BookLengthMismatch,
}
impl ErrorCode {
    pub(super) fn status(self) -> StatusCode {
        match self {
            Self::InvalidField | Self::ProjectionOverflow => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::MalformedJson | Self::InvalidBody => StatusCode::BAD_REQUEST,
            Self::BundleConflict
            | Self::LegacyBindingConflict
            | Self::LegacyAdoptionPending
            | Self::BookLengthMismatch => StatusCode::CONFLICT,
            Self::StorageError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
