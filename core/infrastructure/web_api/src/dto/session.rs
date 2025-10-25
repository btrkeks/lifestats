//! Data transfer objects for session endpoints.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request body for starting a new heartbeat session.
///
/// All fields are optional. If not provided, default `HeartbeatConfig` values are used.
#[derive(Debug, Deserialize, Validate)]
pub struct StartSessionRequest {
    /// Idle timeout in seconds. Session will auto-stop after this duration without heartbeats.
    #[validate(range(min = 1))]
    pub idle_timeout_seconds: Option<i64>,

    /// Flush interval in seconds. Session will flush pending data to database at this interval.
    #[validate(range(min = 1))]
    pub flush_interval_seconds: Option<i64>,
}

/// Response containing session status information.
///
/// Returned by all session query and mutation endpoints.
#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    /// The statistic ID this session is tracking.
    pub statistic_id: u32,

    /// Human-readable status: "running" or "stopped"
    pub status: String,

    /// Whether the session is currently active.
    pub is_running: bool,

    /// Total elapsed time tracked by this session in seconds.
    pub total_elapsed_seconds: i64,
}
