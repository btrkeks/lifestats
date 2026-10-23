use super::{
    ebook,
    integration_error::{ErrorBody, ErrorCode, ErrorEnvelope},
    yomikata,
};
use axum::Json;
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    paths(
        yomikata::replace_day,
        ebook::descriptor,
        ebook::put_day,
        ebook::retired_provision,
        ebook::retired_events
    ),
    components(schemas(
        yomikata::YomikataDayRequest,
        ebook::EbookDescriptor,
        ebook::EbookDayRequest,
        ErrorEnvelope,
        ErrorBody,
        ErrorCode
    )),
    info(title = "Lifestats integrations API", version = "2")
)]
pub struct IntegrationsApi;
pub async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(IntegrationsApi::openapi())
}
