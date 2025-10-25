//! Session management for heartbeat-based tracking.
//!
//! This module provides `SessionManager` which orchestrates multiple `HeartbeatSession`
//! instances with thread-safe concurrent access. It manages the lifecycle of sessions
//! and provides a statistic-keyed interface for starting, sending beats, and stopping sessions.
//!
//! # Thread Safety
//!
//! SessionManager uses `RwLock` to allow concurrent HTTP handlers to safely access
//! different sessions. Queries (get_status, list) acquire read locks, while mutations
//! (start, beat, stop) acquire write locks.

use crate::use_cases::{HeartbeatConfig, HeartbeatSession, RealTimeProvider};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Error type for session management operations.
#[derive(Debug, Clone)]
pub enum SessionError {
    /// A session is already running for this statistic.
    SessionAlreadyExists(StatisticId),
    /// No active session exists for this statistic.
    SessionNotFound(StatisticId),
    /// The requested statistic was not found in the repository.
    StatisticNotFound(StatisticId),
    /// Invalid configuration provided (e.g., negative timeout).
    InvalidConfiguration(String),
}

/// Status information about an active or recently active session.
#[derive(Debug, Clone)]
pub struct SessionStatus {
    /// The statistic this session is tracking.
    pub statistic_id: StatisticId,
    /// Whether the session is currently running.
    pub is_running: bool,
    /// Total elapsed time tracked by this session.
    pub total_elapsed: chrono::Duration,
}

/// Manages multiple concurrent heartbeat sessions with thread-safe access.
///
/// SessionManager is the orchestration layer for HeartbeatSession instances.
/// It maintains a map of statistic IDs to sessions and provides methods to
/// start, interact with, and stop sessions.
pub struct SessionManager<R: StatisticRepository + Send + Sync + 'static> {
    sessions: Arc<RwLock<HashMap<StatisticId, HeartbeatSession<R, RealTimeProvider>>>>,
    repository: Arc<R>,
}

impl<R: StatisticRepository + Send + Sync + 'static> SessionManager<R> {
    /// Create a new SessionManager with a reference to the statistic repository.
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            repository,
        }
    }

    /// Start a new heartbeat session for a statistic.
    ///
    /// # Errors
    ///
    /// - `SessionAlreadyExists`: A session is already running for this statistic
    /// - `StatisticNotFound`: The statistic doesn't exist in the repository
    /// - `InvalidConfiguration`: Invalid idle_timeout or flush_interval
    pub fn start_session(
        &self,
        statistic_id: StatisticId,
        config: Option<HeartbeatConfig>,
    ) -> Result<SessionStatus, SessionError> {
        // Verify statistic exists
        if self.repository.find_metadata_by_id(statistic_id).is_err() {
            return Err(SessionError::StatisticNotFound(statistic_id));
        }

        // Use provided config or default
        let config = config.unwrap_or_default();

        // Validate configuration
        if config.idle_timeout.num_seconds() <= 0 {
            return Err(SessionError::InvalidConfiguration(
                "idle_timeout must be positive".to_string(),
            ));
        }
        if config.flush_interval.num_seconds() <= 0 {
            return Err(SessionError::InvalidConfiguration(
                "flush_interval must be positive".to_string(),
            ));
        }

        // Acquire write lock and check for existing session
        let mut sessions = self.sessions.write().unwrap();

        if sessions.contains_key(&statistic_id) {
            return Err(SessionError::SessionAlreadyExists(statistic_id));
        }

        // Create new session
        let session = HeartbeatSession::new(
            statistic_id,
            self.repository.clone(),
            Arc::new(RealTimeProvider),
            config,
        );

        let is_running = session.is_running();
        let total_elapsed = session.total_time_elapsed();

        sessions.insert(statistic_id, session);

        Ok(SessionStatus {
            statistic_id,
            is_running,
            total_elapsed,
        })
    }

    /// Send a heartbeat for an active session.
    ///
    /// # Errors
    ///
    /// - `SessionNotFound`: No active session for this statistic
    pub fn beat(&self, statistic_id: StatisticId) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().unwrap();

        let session = sessions
            .get_mut(&statistic_id)
            .ok_or(SessionError::SessionNotFound(statistic_id))?;

        session.beat();
        Ok(())
    }

    /// Stop a session, finalizing any pending time frames.
    ///
    /// # Errors
    ///
    /// - `SessionNotFound`: No active session for this statistic
    pub fn stop_session(&self, statistic_id: StatisticId) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().unwrap();

        let mut session = sessions
            .remove(&statistic_id)
            .ok_or(SessionError::SessionNotFound(statistic_id))?;

        session.stop();
        Ok(())
    }

    /// Get the current status of a session.
    ///
    /// Returns status even if the session has been stopped (as long as it was started).
    ///
    /// # Errors
    ///
    /// - `SessionNotFound`: No session exists for this statistic (never started)
    pub fn get_status(&self, statistic_id: StatisticId) -> Result<SessionStatus, SessionError> {
        let sessions = self.sessions.read().unwrap();

        sessions
            .get(&statistic_id)
            .map(|session| SessionStatus {
                statistic_id,
                is_running: session.is_running(),
                total_elapsed: session.total_time_elapsed(),
            })
            .ok_or(SessionError::SessionNotFound(statistic_id))
    }

    /// List all currently active sessions.
    ///
    /// Returns only sessions that are currently running. Stopped sessions are not included.
    pub fn list_active_sessions(&self) -> Vec<SessionStatus> {
        let sessions = self.sessions.read().unwrap();

        sessions
            .iter()
            .filter(|(_, session)| session.is_running())
            .map(|(id, session)| SessionStatus {
                statistic_id: *id,
                is_running: true,
                total_elapsed: session.total_time_elapsed(),
            })
            .collect()
    }
}
