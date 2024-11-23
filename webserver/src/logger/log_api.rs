use rocket::post;
use rocket::serde::json::Json;
use crate::logger::log_core::{log_statistic};
use rocket::response::status::Custom;
use rocket::http::Status;
use api_client::types::{LogRequest, LogResponse};

#[post("/api/log", format = "json", data = "<request>")]
pub async fn log_statistic_api(
    request: Json<LogRequest>,
) -> Result<Json<LogResponse>, Custom<Json<LogResponse>>> {
    debug!("Received log request for statistic {}: {:?}", request.statistic_id, request);

    match log_statistic(request.into_inner()).await {
        Ok(response) => {
            info!("{}", response.message);
            Ok(Json(response))
        }
        Err(e) => {
            warn!("Failed to log statistic: {}", e);
            Err(Custom(
                Status::BadRequest,
                Json(LogResponse::error(e.to_string()))
            ))
        }
    }
}