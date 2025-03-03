use crate::guards::Context;
use rocket::get;
use rocket::serde::json::Json;
use transfer_objects::statistic_id::StatisticId;
use web_api::types::LogResponse;
use rocket::State;
use rocket::yansi::Paint;
use crate::heartbeat::manager::HeartbeatManager;

#[post("/api/heartbeatlogger/<id>/ping")]
pub async fn ping_heartbeat(
    context: Context,
    manager: &State<HeartbeatManager>,
    id: u32,
) -> Json<LogResponse> {
    // Convert the raw ID to our domain type
    let stat_id = StatisticId(id);

    match manager.get_or_create_logger(stat_id).await {
        Ok(logger) => {
            // Get mutex lock and send beat
            if let Ok(mut logger) = logger.lock() {
                logger.beat();
                Json(LogResponse::success("Heartbeat logged successfully".into()))
            } else {
                Json(LogResponse::error("Failed to acquire logger lock".into()))
            }
        }
        Err(e) => Json(LogResponse::error(format!("Failed to get logger: {}", e))),
    }
}

#[post("/api/heartbeatlogger/<id>/stop")]
pub async fn stoping_heartbeat(
    context: Context,
    manager: &State<HeartbeatManager>,
    id: u32,
) -> Json<LogResponse> {
    // Convert the raw ID to our domain type
    let stat_id = StatisticId(id);

    match manager.get_or_create_logger(stat_id).await {
        Ok(logger) => {
            // Get mutex lock and send beat
            if let Ok(mut logger) = logger.lock() {
                logger.stop();
                Json(LogResponse::success("Heart stopped successfully".into()))
            } else {
                Json(LogResponse::error("Failed to acquire logger lock".into()))
            }
        }
        Err(e) => Json(LogResponse::error(format!("Failed to get logger: {}", e))),
    }
}