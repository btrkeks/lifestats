/// This library allows logging data by sending 'pings' or 'heartbeats'.
/// A heartbeat indicates, that the action is currently ongoing and
/// starts the data logging.
/// If there was no heartbeat for a certain period of time, it is
/// assumed that the action isn't ongoing anymore and logging will stop.
pub mod logger;

use chrono::{DateTime, Local};
use lifestats_core::app_context::AppContext;
use log::{debug, error};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{thread, time};
use transfer_objects::statistic_id::StatisticId;

use crate::logger::http_logger::HttpLogger;
use logger::local_logger::LocalLogger;
use logger::logger_trait::TimeFrameLogger;
use testing_utilities::time_provider::TimeProvider;

use tokio::sync::mpsc as tmpsc;

#[derive(Clone)]
pub struct HeartbeatConfig {
    /// Idle timeout after which logging is considered stopped if no beats are received
    pub idle_timeout: chrono::Duration,
    /// Interval at which the logger flushes logging data to the database
    pub flush_interval: chrono::Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            idle_timeout: chrono::Duration::seconds(60),
            flush_interval: chrono::Duration::seconds(60),
        }
    }
}

#[derive(Debug)]
pub enum HeartbeatUpdate {
    DurationChanged(chrono::Duration),
    Stopped,
    Started,
}

pub enum HeartMessage {
    Beat,
    Stop,
}

pub struct HeartbeatLogger<TL: TimeFrameLogger + 'static, TP: TimeProvider + 'static> {
    time_provider: Arc<TP>,
    logger: Arc<TL>,
    config: Arc<HeartbeatConfig>,
    heart_tx: Option<Sender<HeartMessage>>,
    heart_thread_handle: Option<JoinHandle<()>>,
    update_tx: Option<tmpsc::Sender<HeartbeatUpdate>>,
    total_duration_manager: Arc<TotalDurationManager>,
}

impl<TL: TimeFrameLogger + 'static, TP: TimeProvider + 'static> HeartbeatLogger<TL, TP> {
    pub fn new(time_provider: Arc<TP>, logger: TL, config: HeartbeatConfig) -> Self {
        HeartbeatLogger {
            time_provider,
            logger: Arc::new(logger),
            config: Arc::new(config),
            heart_tx: None,
            heart_thread_handle: None,
            update_tx: None,
            total_duration_manager: Arc::new(TotalDurationManager::new()),
        }
    }

    pub fn new_with_local_logger(
        time_provider: Arc<TP>,
        app_context: Arc<AppContext>,
        statistic_id: StatisticId,
        config: HeartbeatConfig,
    ) -> HeartbeatLogger<LocalLogger, TP> {
        HeartbeatLogger::new(
            time_provider,
            LocalLogger::new(statistic_id, app_context),
            config,
        )
    }

    pub fn new_with_http_logger(
        time_provider: Arc<TP>,
        statistic_id: StatisticId,
        server_url: &str,
        config: HeartbeatConfig,
    ) -> anyhow::Result<HeartbeatLogger<HttpLogger, TP>> {
        Ok(HeartbeatLogger::new(
            time_provider,
            HttpLogger::new(statistic_id, server_url)?,
            config,
        ))
    }

    // 操作

    pub fn beat(&mut self) {
        match &self.heart_tx {
            Some(sender) if sender.send(HeartMessage::Beat).is_ok() => {}
            _ => self.spawn_heart_thread(),
        }
    }

    pub fn stop(&mut self) {
        if let Some(sender) = self.heart_tx.take() {
            let _ = sender.send(HeartMessage::Stop);
        }

        if let Some(handle) = self.heart_thread_handle.take() {
            if let Err(e) = handle.join() {
                error!("Heartbeat thread panicked: {:?}", e);
            }
        }
    }

    // Status

    pub fn is_running(&self) -> bool {
        if let Some(heart_handle) = &self.heart_thread_handle {
            !heart_handle.is_finished()
        } else {
            false
        }
    }

    pub fn total_time_elapsed(&self) -> chrono::Duration {
        self.total_duration_manager.get_current_duration()
    }

    // Observing

    pub fn subscribe_to_updates(&mut self) -> tmpsc::Receiver<HeartbeatUpdate> {
        let (tx, rx) = tmpsc::channel(32);
        self.update_tx = Some(tx);
        rx
    }

    // ------------------------------------------------

    fn spawn_heart_thread(&mut self) {
        let (sender, receiver) = mpsc::channel();
        self.heart_tx = Some(sender);

        let time_provider = self.time_provider.clone();
        let logger = Arc::clone(&self.logger);
        let config = Arc::clone(&self.config);
        let update_tx = self.update_tx.clone();
        let total_time_manager = Arc::clone(&self.total_duration_manager);

        self.heart_thread_handle = Some(thread::spawn(move || {
            let mut heart =
                Heart::new(time_provider, logger, config, update_tx, total_time_manager);
            heart.run(receiver);
        }));
    }
}

impl<TL: TimeFrameLogger + 'static, TP: TimeProvider + 'static> Drop for HeartbeatLogger<TL, TP> {
    fn drop(&mut self) {
        self.stop();
    }
}

// =================================================================================================
//  Heart / Worker
// =================================================================================================

/// Internal worker struct that runs in a dedicated thread.
pub struct Heart<TL: TimeFrameLogger, TP: TimeProvider> {
    time_provider: Arc<TP>,
    logger: Arc<TL>,
    config: Arc<HeartbeatConfig>,
    update_tx: Option<tmpsc::Sender<HeartbeatUpdate>>,
    total_duration_manager: Arc<TotalDurationManager>,
}

impl<TL: TimeFrameLogger, TP: TimeProvider> Heart<TL, TP> {
    pub fn new(
        time_provider: Arc<TP>,
        logger: Arc<TL>,
        config: Arc<HeartbeatConfig>,
        update_tx: Option<tmpsc::Sender<HeartbeatUpdate>>,
        total_duration_manager: Arc<TotalDurationManager>,
    ) -> Self {
        Self {
            time_provider,
            logger,
            config,
            update_tx,
            total_duration_manager,
        }
    }

    pub fn run(&mut self, receiver: Receiver<HeartMessage>) {
        let heart_timing = HeartTiming::new(self.time_provider.clone(), self.config.clone());

        self.emit_start_event();
        let result = self.main_loop(receiver, heart_timing);
        self.emit_stop_event();

        if let Err(e) = result {
            error!("Error in Heart thread: {}", e);
        }
    }

    fn main_loop(
        &mut self,
        receiver: Receiver<HeartMessage>,
        mut heart_timing: HeartTiming<TP>,
    ) -> anyhow::Result<()> {
        // TODO: If nobody is listening, we could increase the next wait duration
        debug!("Starting heart thread main loop");
        self.flush_log_now(&heart_timing)?;
        loop {
            match receiver.recv_timeout(heart_timing.calculate_next_wait_duration()) {
                Ok(HeartMessage::Stop) => {
                    debug!("Heart thread received stop message");
                    let (start_time, end_time) = heart_timing.get_current_log_duration();
                    self.logger.log(start_time.time(), end_time.time())?;
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    let timeout_action = heart_timing.get_next_action();
                    match timeout_action {
                        TimeoutAction::FlushLog((start_time, end_time)) => {
                            debug!("Flushing log in heart thread");
                            self.logger.log(start_time.time(), end_time.time())?;
                        }
                        TimeoutAction::EmitDurationUpdate(elapsed_seconds) => {
                            debug!("Emitting duration update");
                            let new_total_duration =
                                self.total_duration_manager.add_and_fetch_total(elapsed_seconds);
                            self.emit_duration_update(new_total_duration);
                        }
                        TimeoutAction::IdleTimeout((start_time, end_time)) => {
                            self.logger.log(start_time.time(), end_time.time())?;
                            break;
                        }
                        TimeoutAction::Noop => {}
                    }
                }
                Ok(HeartMessage::Beat) => {
                    debug!("Heart thread received beat message");
                    heart_timing.record_beat();
                }
                Err(RecvTimeoutError::Disconnected) => {
                    // Something went wrong, just stop
                    // (should never happen anyway due to the drop implementation)
                    error!("HeartbeatLogger disconnected, stop logging");
                    self.flush_log_now(&heart_timing)?;
                    break;
                }
            }
        }
        Ok(())
    }

    fn flush_log_now(&self, heart_timing: &HeartTiming<TP>) -> anyhow::Result<()> {
        let (start_time, end_time) = heart_timing.get_current_log_duration();
        self.logger.log(start_time.time(), end_time.time())
    }

    fn emit_start_event(&self) {
        if let Some(tx) = &self.update_tx {
            let _ = tx.try_send(HeartbeatUpdate::Started);
        }
    }

    fn emit_stop_event(&self) {
        if let Some(tx) = &self.update_tx {
            let _ = tx.try_send(HeartbeatUpdate::Stopped);
        }
    }

    pub fn emit_duration_update(&mut self, current_duration: chrono::Duration) {
        if let Some(tx) = &self.update_tx {
            let _ = tx.try_send(HeartbeatUpdate::DurationChanged(current_duration));
        }
    }
}

// =================================================================================================
//  HeartTiming
// =================================================================================================

#[derive(Debug, Hash, Eq, PartialEq)]
enum TimeoutAction {
    /// start and end time that should be flushed
    FlushLog((DateTime<Local>, DateTime<Local>)),
    /// The amount of seconds that elapsed since the last update
    EmitDurationUpdate(u64),
    /// start and end time of the final log
    IdleTimeout((DateTime<Local>, DateTime<Local>)),
    Noop,
}

/// All timing-related state and logic for heartbeats.
pub struct HeartTiming<TP: TimeProvider> {
    time_provider: Arc<TP>,
    config: Arc<HeartbeatConfig>,

    /// The absolute time when logging started.
    start_time: DateTime<Local>,
    /// The last time a heartbeat (Beat) was received.
    last_beat: DateTime<Local>,
    /// The last time we flushed the log to the database
    last_log: DateTime<Local>,
    /// The last time we emitted a duration update to the listeners
    last_duration_update: Instant,
}

impl<TP: TimeProvider> HeartTiming<TP> {
    fn new(time_provider: Arc<TP>, config: Arc<HeartbeatConfig>) -> Self {
        let now_dt = time_provider.now();
        let now_instant = time_provider.current_instant();
        HeartTiming {
            time_provider,
            config,
            start_time: now_dt,
            last_beat: now_dt,
            last_log: now_dt,
            last_duration_update: now_instant,
        }
    }

    /// Marks a new heartbeat arrival.
    fn record_beat(&mut self) {
        self.last_beat = self.time_provider.now();
    }

    /// Determines how long to wait before the next check of timeouts or flushing.
    fn calculate_next_wait_duration(&self) -> time::Duration {
        // Next required updates happen at:
        // 1. next duration update (every 1s)
        // 2. next flush log
        // 3. next idle timeout
        let current_instant = self.time_provider.current_instant();
        let current_time = self.time_provider.now();

        // The duration should be flushed on every second boundary
        let duration_wait = (self.last_duration_update + Duration::from_secs(1))
            .saturating_duration_since(current_instant);

        let flush_wait = (self.last_log + self.config.flush_interval)
            .signed_duration_since(current_time)
            .to_std()
            .unwrap_or_default();

        let idle_wait = (self.last_beat + self.config.idle_timeout)
            .signed_duration_since(current_time)
            .to_std()
            .unwrap_or_default();

        duration_wait.min(flush_wait).min(idle_wait)
    }

    /// Signals if some timer elapsed and an action should be taken.
    /// Assumes the action will always be taken. Otherwise, we would end up in an infinite loop
    fn get_next_action(&mut self) -> TimeoutAction {
        let current_instant = self.time_provider.current_instant();
        let current_time = self.time_provider.now();

        if self.should_flush(current_time) {
            let start_time = self.start_time;
            let end_time = self.last_log + self.config.flush_interval;

            self.last_log = end_time;

            TimeoutAction::FlushLog((start_time, end_time))
        } else if self.should_update_duration(current_instant) {
            // Only update whole seconds
            let secs_since_last_update = current_instant
                .duration_since(self.last_duration_update)
                .as_secs();

            self.last_duration_update += Duration::from_secs(secs_since_last_update);

            TimeoutAction::EmitDurationUpdate(secs_since_last_update)
        } else if self.idle_timeout_elapsed(current_time) {
            let start_time: DateTime<Local> = self.start_time;
            // Here we need to know the exact time at which the heart should have expired.
            // That's why self.last_beat needs to be a DateTime<Local>
            let end_time: DateTime<Local> = self.last_beat + self.config.idle_timeout;

            self.last_beat = end_time;

            TimeoutAction::IdleTimeout((start_time, end_time))
        } else {
            TimeoutAction::Noop
        }
    }

    fn should_flush(&self, current_time: DateTime<Local>) -> bool {
        current_time.signed_duration_since(self.last_log) >= self.config.flush_interval
    }

    fn should_update_duration(&self, current_instant: Instant) -> bool {
        current_instant.duration_since(self.last_duration_update) >= Duration::from_secs(1)
    }

    fn idle_timeout_elapsed(&self, current_time: DateTime<Local>) -> bool {
        current_time.signed_duration_since(self.last_beat) >= self.config.idle_timeout
    }

    fn get_current_log_duration(&self) -> (DateTime<Local>, DateTime<Local>) {
        let now = self.time_provider.now();
        (self.start_time, now)
    }
}

struct TotalDurationManager {
    seconds: AtomicU64,
}

impl TotalDurationManager {
    pub fn new() -> Self {
        Self {
            seconds: AtomicU64::new(0),
        }
    }

    pub fn add_and_fetch_total(&self, seconds: u64) -> chrono::Duration {
        let total_seconds = self.seconds.fetch_add(seconds, Ordering::Relaxed) + seconds;
        chrono::Duration::seconds(total_seconds as i64)
    }

    pub fn get_current_duration(&self) -> chrono::Duration {
        let total_seconds = self.seconds.load(Ordering::Relaxed);
        chrono::Duration::seconds(total_seconds as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::time::Duration;
    use testing_utilities::time_provider::MockTimeProvider;

    fn setup_heart_timing() -> (Arc<MockTimeProvider>, HeartTiming<MockTimeProvider>) {
        let time_provider = Arc::new(MockTimeProvider::new());
        let config = Arc::new(HeartbeatConfig {
            idle_timeout: chrono::Duration::milliseconds(47700),
            flush_interval: chrono::Duration::milliseconds(30300),
        });
        let heart_timing = HeartTiming::new(Arc::clone(&time_provider), config);
        (time_provider, heart_timing)
    }

    fn get_all_pending_actions(
        heart_timing: &mut HeartTiming<MockTimeProvider>,
    ) -> Vec<TimeoutAction> {
        let mut all_actions = Vec::new();
        while heart_timing.calculate_next_wait_duration() == Duration::from_millis(0) {
            all_actions.push(heart_timing.get_next_action());
        }
        all_actions
    }

    #[test]
    fn test_initial_state() {
        let (provider, timing) = setup_heart_timing();
        let initial_time = provider.now();

        assert_eq!(timing.start_time, initial_time);
        assert_eq!(timing.last_beat, initial_time);
        assert_eq!(timing.last_log, initial_time);
    }

    #[test]
    fn test_record_beat_updates_last_beat() {
        let (provider, mut timing) = setup_heart_timing();

        provider.advance(Duration::from_secs(10));
        timing.record_beat();

        assert_eq!(timing.last_beat, provider.now());
    }

    #[test]
    fn test_should_flush_after_interval() {
        let (provider, timing) = setup_heart_timing();

        // Advance time just before flush interval
        provider.advance(Duration::from_millis(30000));
        assert!(!timing.should_flush(provider.now()));

        // Advance time to flush interval
        provider.advance(Duration::from_millis(300));
        assert!(timing.should_flush(provider.now()));
    }

    #[test]
    fn test_idle_timeout() {
        let (provider, timing) = setup_heart_timing();

        // Check before timeout
        provider.advance(Duration::from_millis(47699));
        assert!(!timing.idle_timeout_elapsed(provider.now()));

        // Check at timeout
        provider.advance(Duration::from_millis(1));
        assert!(timing.idle_timeout_elapsed(provider.now()));
    }

    #[test]
    fn test_should_update_duration() {
        let (provider, timing) = setup_heart_timing();

        // Check before 1 second
        provider.advance(Duration::from_millis(999));
        assert!(!timing.should_update_duration(provider.current_instant()));

        // Check after 1 second
        provider.advance(Duration::from_millis(1));
        assert!(timing.should_update_duration(provider.current_instant()));
    }

    #[test]
    fn test_calculate_next_wait_duration() {
        let (provider, mut timing) = setup_heart_timing();

        let wait = timing.calculate_next_wait_duration();
        let first_instant = provider.current_instant();
        assert_eq!(wait, Duration::from_secs(1));

        provider.advance(Duration::from_millis(30000));
        let second_instant = provider.current_instant();

        // Clear all elapsed timings
        while timing.calculate_next_wait_duration() == Duration::from_millis(0) {
            timing.get_next_action();
        }

        let wait = timing.calculate_next_wait_duration();
        assert_eq!(wait, Duration::from_millis(300));
    }

    #[test]
    fn test_should_always_update_duration_on_second_boundaries() {
        let (provider, mut timing) = setup_heart_timing();

        provider.advance(Duration::from_millis(999));
        assert_eq!(timing.get_next_action(), TimeoutAction::Noop);

        provider.advance(Duration::from_millis(1000));
        assert_eq!(
            timing.get_next_action(),
            TimeoutAction::EmitDurationUpdate(1)
        );

        assert_eq!(
            timing.calculate_next_wait_duration(),
            Duration::from_millis(1)
        );
    }

    #[test]
    fn test_should_take_action_flush() {
        let (provider, mut timing) = setup_heart_timing();

        // Advance to flush interval
        provider.advance(Duration::from_millis(30300));

        let all_pending_actions = get_all_pending_actions(&mut timing);
        assert!(all_pending_actions
            .iter()
            .any(|action| matches!(action, TimeoutAction::FlushLog(_))));
    }

    #[test]
    fn test_continuous_heartbeats_prevent_idle_timeout() {
        let (provider, mut timing) = setup_heart_timing();

        // Advance close to idle timeout
        provider.advance(Duration::from_millis(47000));
        timing.record_beat();

        // Advance past original timeout, but beat should have reset it
        provider.advance(Duration::from_millis(47000));

        let all_pending_actions = get_all_pending_actions(&mut timing);
        assert!(all_pending_actions
            .iter()
            .any(|action| !matches!(action, TimeoutAction::IdleTimeout(_))));
    }

    #[test]
    fn test_should_take_action_idle_timeout() {
        let (provider, mut timing) = setup_heart_timing();

        // Advance past idle timeout
        provider.advance(Duration::from_millis(47700));

        let all_pending_actions = get_all_pending_actions(&mut timing);
        assert!(all_pending_actions
            .iter()
            .any(|action| matches!(action, TimeoutAction::IdleTimeout(_))));
    }

    #[test]
    fn test_get_current_log_duration() {
        let (provider, timing) = setup_heart_timing();
        let initial_time = provider.now();

        provider.advance(Duration::from_secs(15));
        let (start, end) = timing.get_current_log_duration();

        assert_eq!(start, initial_time);
        assert_eq!(end, provider.now());
    }

    #[test]
    fn test_last_log_updates_after_flush() {
        let (provider, mut timing) = setup_heart_timing();

        let initial_last_log = timing.last_log;
        provider.advance(Duration::from_millis(30300));

        get_all_pending_actions(&mut timing);

        assert!(
            timing.last_log > initial_last_log,
            "last_log should update polling"
        );
    }

    #[test]
    fn test_sequential_duration_updates() {
        let (provider, mut timing) = setup_heart_timing();

        // Test multiple sequential duration updates
        for i in 1..=3 {
            provider.advance(Duration::from_secs(1));

            let mut found_update = false;
            while timing.calculate_next_wait_duration() == Duration::from_millis(0) {
                if let TimeoutAction::EmitDurationUpdate(secs) = timing.get_next_action() {
                    assert_eq!(secs, 1, "Should emit exactly one second for each update");
                    found_update = true;
                }
            }

            assert!(
                found_update,
                "Should find duration update for iteration {}",
                i
            );
        }
    }
}
