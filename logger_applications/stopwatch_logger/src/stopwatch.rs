use heart_beat_logger::logger::local_logger::LocalLogger;
use heart_beat_logger::{HeartbeatConfig, HeartbeatLogger, HeartbeatUpdate};
use lifestats_core::app_context::AppContext;
use std::sync::Arc;
use std::time;
use testing_utilities::time_provider::TimeProvider;
use tokio::sync::mpsc as tmpsc;
use transfer_objects::statistic_id::StatisticId;

pub struct Stopwatch<TP: TimeProvider + 'static> {
    heartbeat_logger: HeartbeatLogger<LocalLogger, TP>,
}

pub struct StopwatchConfig {
    pub(crate) statistic_id: StatisticId,
    pub log_flush_interval: chrono::Duration,
}

impl<TP: TimeProvider + 'static> Stopwatch<TP> {
    pub fn new(app_context: AppContext, time_provider: Arc<TP>, config: StopwatchConfig) -> Self {
        let heartbeat_logger = HeartbeatLogger::<LocalLogger, TP>::new_with_local_logger(
            time_provider,
            Arc::new(app_context),
            config.statistic_id,
            HeartbeatConfig {
                idle_timeout: chrono::Duration::days(1),
                flush_interval: config.log_flush_interval,
            },
        );

        Self { heartbeat_logger }
    }

    pub fn subscribe_to_updates(&mut self) -> tmpsc::Receiver<HeartbeatUpdate> {
        self.heartbeat_logger.subscribe_to_updates()
    }

    pub fn is_running(&self) -> bool {
        self.heartbeat_logger.is_running()
    }

    pub fn toggle(&mut self) {
        if self.is_running() {
            self.stop()
        } else {
            self.start()
        }
    }

    pub fn start(&mut self) {
        self.heartbeat_logger.beat();
    }

    pub fn stop(&mut self) {
        self.heartbeat_logger.stop();
    }

    pub fn get_elapsed_time(&self) -> chrono::Duration {
        self.heartbeat_logger.total_time_elapsed()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::stopwatch::StopwatchConfig;
//     use lifestats_core::testing;
//     use lifestats_core::testing::mock_app_context_builder::MockAppContextBuilder;
//     use std::sync::Arc;
//     use std::time;
//     use log::debug;
//     use testing_utilities::time_provider::{MockTimeProvider, TimeProvider};
//     use transfer_objects::statistic_id::StatisticId;
//
//     struct TestContext {
//         time_provider: Arc<MockTimeProvider>,
//         stopwatch: Stopwatch<MockTimeProvider>,
//     }
//
//     impl TestContext {
//         fn new() -> Self {
//             let time_provider = Arc::new(MockTimeProvider::new());
//
//             let test_statistic = testing::mock_statistics::MockTimeSpansStatisticBuilder::new()
//                 .with_id(StatisticId(1))
//                 .with_log(time_provider.now().date_naive(), Default::default())
//                 .build();
//             let app_context = MockAppContextBuilder::new()
//                 .with_statistics(vec![test_statistic])
//                 .build();
//
//             let stopwatch = Stopwatch::new(
//                 app_context,
//                 Arc::clone(&time_provider),
//                 StopwatchConfig {
//                     statistic_id: StatisticId(1),
//                     log_flush_interval: chrono::Duration::milliseconds(50),
//                 },
//             );
//
//             Self {
//                 time_provider,
//                 // app_context,
//                 stopwatch,
//             }
//         }
//
//         fn advance_time(&self, duration: time::Duration) {
//             self.time_provider.advance(duration);
//         }
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_initial_state() {
//         let context = TestContext::new();
//         assert!(!context.stopwatch.is_running());
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_start_stop() {
//         let mut context = TestContext::new();
//
//         // Start the stopwatch
//         context.stopwatch.start();
//         assert!(context.stopwatch.is_running());
//
//         // Advance time by 1 minute
//         context.advance_time(time::Duration::from_mins(1));
//
//         // Stop the stopwatch
//         context.stopwatch.stop();
//         assert!(!context.stopwatch.is_running());
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_updates() {
//         let mut context = TestContext::new();
//         let mut updates = context.stopwatch.subscribe_to_updates();
//
//         // Start the stopwatch
//         context.stopwatch.start();
//
//         // We should receive a Started update
//         match updates.recv().await {
//             Some(HeartbeatUpdate::Started) => (),
//             other => panic!("Expected Started update, got {:?}", other),
//         }
//
//         // Advance time and check for duration updates
//         context.advance_time(time::Duration::from_secs(12938));
//
//         // We should receive a DurationChanged update
//         match updates.recv().await {
//             Some(HeartbeatUpdate::DurationChanged(duration)) => {
//                 assert_eq!(duration.num_seconds(), 12938);
//             }
//             other => panic!("Expected DurationChanged update, got {:?}", other),
//         }
//
//         // Stop the stopwatch
//         context.stopwatch.stop();
//
//         // We should receive a Stopped update
//         match updates.recv().await {
//             Some(HeartbeatUpdate::Stopped) => (),
//             other => panic!("Expected Stopped update, got {:?}", other),
//         }
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_toggle() {
//         let mut context = TestContext::new();
//
//         // Initially stopped
//         assert!(!context.stopwatch.is_running());
//
//         // Toggle to start
//         context.stopwatch.toggle();
//         assert!(context.stopwatch.is_running());
//
//         // Toggle to stop
//         context.stopwatch.toggle();
//         assert!(!context.stopwatch.is_running());
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_elapsed_time_single_run() {
//         env_logger::init();
//         debug!("Debug messages working!");
//         let mut context = TestContext::new();
//         let mut updates = context.stopwatch.subscribe_to_updates();
//
//         // Start the stopwatch
//         context.stopwatch.start();
//
//         let _ = updates.recv().await; // Ignore the start event
//
//         // Advance time in increments
//         let increments = &[30, 45, 90]; // seconds
//         let mut total_elapsed = 0;
//
//         for &seconds in increments {
//             context.advance_time(time::Duration::from_secs(seconds));
//             total_elapsed += seconds;
//
//             // We should receive a DurationChanged update
//             match updates.recv().await {
//                 Some(HeartbeatUpdate::DurationChanged(duration)) => {
//                     assert_eq!(duration.num_seconds(), total_elapsed as i64);
//                 }
//                 other => panic!("Expected DurationChanged update, got {:?}", other),
//             }
//         }
//
//         context.stopwatch.stop();
//     }
//
//     #[tokio::test]
//     async fn test_stopwatch_elapsed_time_multiple_runs() {
//         let mut context = TestContext::new();
//         let mut updates = context.stopwatch.subscribe_to_updates();
//
//         // Start the stopwatch
//         context.stopwatch.start();
//
//         let _ = updates.recv().await; // Ignore the start event
//
//         // Advance time in increments
//         let increments = &[100, 128, 397]; // seconds
//         let mut total_elapsed = 0;
//
//         for &seconds in increments {
//             context.advance_time(time::Duration::from_secs(seconds));
//             total_elapsed += seconds;
//
//             // We should receive a DurationChanged update
//             match updates.recv().await {
//                 Some(HeartbeatUpdate::DurationChanged(duration)) => {
//                     assert_eq!(duration.num_seconds(), total_elapsed as i64);
//                 }
//                 other => panic!("Expected DurationChanged update, got {:?}", other),
//             }
//
//             context.stopwatch.stop();
//             let _ = updates.recv().await;
//             context.advance_time(time::Duration::from_secs(50));
//             context.stopwatch.start();
//             let _ = updates.recv().await;
//         }
//
//         context.stopwatch.stop();
//     }
// }
