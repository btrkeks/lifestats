// use std::collections::HashMap;
// use std::sync::Mutex;
// use rocket::serde::{Deserialize, Serialize};
// use heart_beat_logger::HeartbeatLogger;
//
// #[derive(Serialize, Deserialize, Default)]
// #[serde(crate = "rocket::serde")]
// #[serde(rename_all = "lowercase")]
// pub enum HeartbeatAction {
//     #[default]
//     Beat,
//     Stop,
// }
//
// pub struct HeartbeatManager {
//     loggers: Mutex<HashMap<u32, HeartbeatLogger>>,
// }
//
// impl HeartbeatManager {
//     pub fn new() -> Self {
//         HeartbeatManager {
//             loggers: Mutex::new(HashMap::new()),
//         }
//     }
//
//     pub fn beat(&self, stat_id: u32) -> Result<(), String> {
//         let mut loggers = self.loggers.lock().map_err(|e| e.to_string())?;
//
//         if let Some(logger) = loggers.get_mut(&stat_id) {
//             logger.beat();
//         } else {
//             let mut logger = HeartbeatLogger::new_with_local_logger(stat_id, 60.0);
//             logger.beat();
//             loggers.insert(stat_id, logger);
//         }
//
//         Ok(())
//     }
//
//     pub fn stop(&self, stat_id: u32) -> Result<(), String> {
//         // TODO: Doesn't remove from hashmap. Investigate if this can lead to problems (data being outdated)
//         let mut loggers = self.loggers.lock().map_err(|e| e.to_string())?;
//
//         if let Some(mut logger) = loggers.remove(&stat_id) {
//             logger.stop();
//             Ok(())
//         } else {
//             Err("No heartbeat logger found for this statistic".to_string())
//         }
//     }
// }