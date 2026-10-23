use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
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
    ClientUpgradeRequired,
    BookLengthMismatch,
}
impl ErrorCode {
    pub(super) fn status(self) -> StatusCode {
        match self {
            Self::InvalidField | Self::ProjectionOverflow => StatusCode::UNPROCESSABLE_ENTITY,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::MalformedJson | Self::InvalidBody => StatusCode::BAD_REQUEST,
            Self::BundleConflict | Self::BookLengthMismatch => StatusCode::CONFLICT,
            Self::ClientUpgradeRequired => StatusCode::GONE,
            Self::StorageError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
pub struct IntegrationApiError {
    status: StatusCode,
    body: ErrorEnvelope,
}
impl IntegrationApiError {
    pub fn new(code: ErrorCode, field: Option<&str>, message: &str) -> Self {
        Self {
            status: code.status(),
            body: ErrorEnvelope {
                error: ErrorBody {
                    code,
                    field: field.map(str::to_owned),
                    message: message.into(),
                },
            },
        }
    }
    pub fn invalid(field: Option<&str>, message: &str) -> Self {
        Self::new(ErrorCode::InvalidField, field, message)
    }
    pub fn json(error: JsonRejection) -> Self {
        match error {
            JsonRejection::MissingJsonContentType(_) => Self::new(
                ErrorCode::UnsupportedMediaType,
                None,
                "Content-Type must be application/json",
            ),
            JsonRejection::JsonSyntaxError(_) => {
                Self::new(ErrorCode::MalformedJson, None, "Malformed JSON")
            }
            JsonRejection::JsonDataError(_) => Self::invalid(None, "Invalid JSON data"),
            _ => Self::new(ErrorCode::InvalidBody, None, "Unable to read request body"),
        }
    }
}
impl IntoResponse for IntegrationApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

pub fn parse_strict_object(
    raw: &RawValue,
) -> Result<BTreeMap<String, Box<RawValue>>, IntegrationApiError> {
    serde_json::from_str(raw.get())
        .map_err(|_| IntegrationApiError::invalid(None, "Expected a JSON object"))
}

pub fn parse_day(raw: &str) -> Result<chrono::NaiveDate, IntegrationApiError> {
    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .filter(|date| raw.len() == 10 && date.format("%Y-%m-%d").to_string() == raw)
        .ok_or_else(|| {
            IntegrationApiError::invalid(Some("day"), "Expected a valid YYYY-MM-DD calendar date")
        })
}

impl From<lifestats_core::ebook::EbookError> for IntegrationApiError {
    fn from(error: lifestats_core::ebook::EbookError) -> Self {
        use lifestats_core::ebook::EbookError;
        match error {
            EbookError::InvalidField { field, reason } => Self::invalid(Some(field), reason),
            EbookError::ProjectionOverflow => Self::new(
                ErrorCode::ProjectionOverflow,
                None,
                "Resulting aggregate exceeds the supported integer range",
            ),
            EbookError::BundleConflict => Self::new(
                ErrorCode::BundleConflict,
                None,
                "Ebook statistics require correction",
            ),
            EbookError::BookLengthMismatch => Self::new(
                ErrorCode::BookLengthMismatch,
                Some("length"),
                "Book length differs from the stored length",
            ),
            EbookError::Storage(error) => {
                tracing::error!(error=?error,"Failed to store ebook reading");
                Self::new(
                    ErrorCode::StorageError,
                    None,
                    "Unable to store ebook reading",
                )
            }
        }
    }
}
