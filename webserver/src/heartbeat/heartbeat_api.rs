// use std::time::Duration;
// use rocket::serde::Deserialize;
// use rocket::serde::json::Json;
// use rocket::State;
// use crate::heartbeat::heartbeat_manager::{HeartbeatAction, HeartbeatManager};
// use anyhow::{Context, Result};
// use reqwest::Client;
//
// #[derive(Deserialize)]
// #[serde(crate = "rocket::serde")]
// pub struct HeartbeatRequest {
//     stat_id: u32,
//     #[serde(default)]
//     action: HeartbeatAction,
// }
//
// #[post("/api/heartbeat", format = "json", data = "<request>")]
// pub fn handle_heartbeat(
//     request: Json<HeartbeatRequest>,
//     manager: &State<HeartbeatManager>,
// ) -> Result<Json<String>, String> {
//     match request.action {
//         HeartbeatAction::Beat => {
//             manager.beat(request.stat_id)?;
//             Ok(Json("Heartbeat recorded".to_string()))
//         }
//         HeartbeatAction::Stop => {
//             manager.stop(request.stat_id)?;
//             Ok(Json("Heartbeat stopped".to_string()))
//         }
//     }
// }
//
// // --------------------- API ----------------------------------
// const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
//
// #[derive(Clone)]
// pub struct HeartbeatConfig {
//     /// Base URL of the heartbeat API (e.g., "http://localhost:8000")
//     pub base_url: String,
//     /// Request timeout
//     pub timeout: Duration,
// }
//
// impl Default for HeartbeatConfig {
//     fn default() -> Self {
//         Self {
//             base_url: "http://localhost:8000".to_string(),
//             timeout: DEFAULT_TIMEOUT,
//         }
//     }
// }
//
// /// Send a heartbeat for the given statistic ID
// ///
// /// # Arguments
// ///
// /// * `stat_id` - The ID of the statistic to send heartbeat for
// /// * `action` - The heartbeat action (Beat or Stop)
// /// * `config` - Configuration for the heartbeat client
// ///
// /// # Example
// ///
// /// ```rust,no_run
// /// use heartbeat::client::{send_heartbeat, HeartbeatConfig, HeartbeatAction};
// ///
// /// # fn main() -> anyhow::Result<()> {
// /// let config = HeartbeatConfig::default();
// ///
// /// // Send a regular heartbeat
// /// send_heartbeat(123, HeartbeatAction::Beat, &config)?;
// ///
// /// // Stop a heartbeat
// /// send_heartbeat(123, HeartbeatAction::Stop, &config)?;
// ///
// /// // Using default action (Beat)
// /// send_heartbeat(123, Default::default(), &config)?;
// /// # Ok(())
// /// # }
// /// ```
// pub async fn send_heartbeat(
//     stat_id: u32,
//     action: HeartbeatAction,
//     config: &HeartbeatConfig
// ) -> Result<()> {
//     let client = Client::builder()
//         .timeout(config.timeout)
//         .build()
//         .context("Failed to create HTTP client")?;
//
//     let url = format!("{}/api/heartbeat", config.base_url);
//
//     let response = client
//         .post(&url)
//         .json(&serde_json::json!({
//             "stat_id": stat_id,
//             "action": action,
//         }))
//         .send()
//         .await
//         .context("Failed to send heartbeat request")?;
//
//     if !response.status().is_success() {
//         return Err(anyhow::anyhow!(
//             "Unexpected status code: {}",
//             response.status()
//         ));
//     }
//
//     Ok(())
// }