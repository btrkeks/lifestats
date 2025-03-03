use crate::guards::Context;
use crate::logger::log_core::log_statistic;
use rocket::http::Status;
use rocket::post;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use web_api::types::{LogRequest, LogResponse};

#[post("/api/log", format = "json", data = "<request>")]
pub async fn log_statistic_api(
    context: Context,
    request: Json<LogRequest>,
) -> Result<Json<LogResponse>, Custom<Json<LogResponse>>> {
    debug!(
        "Received log request for statistic {}: {:?}",
        request.statistic_id, request
    );

    match log_statistic(&context.0, request.into_inner()).await {
        Ok(response) => {
            info!("{}", response.message);
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Failed to log statistic: {}", e);
            Err(Custom(
                Status::BadRequest,
                Json(LogResponse::error(e.to_string())),
            ))
        }
    }
}
