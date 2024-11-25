pub mod http_logger;

// logging/heart_beat_logger/src/lib.rs
use anyhow::{anyhow, Result};
use backend::statistics::objects::{StatisticId, TimeFrame};
use backend::statistics::objects::update_policy::TimeFrameListUpdatePolicy;
use chrono::{Local, NaiveTime};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{mpsc, Arc};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use backend::database::db_ops::open_db;
use backend::statistics::statistic_repository::get_statistic_by_id;
use backend::statistics::type_definitions::{AnyStatistic, StatisticTypeSpecific};

const LOG_FREQUENCY: Duration = Duration::from_secs(60);

enum HeartbeatMessage {
    Beat,
    Stop,
}

pub struct HeartbeatLogger {
    stat_id: StatisticId, // Could be put into the logger
    interval: Duration,
    logger: Arc<dyn TimeFrameLogger>,
    sender: Option<Sender<HeartbeatMessage>>, // Note: doesn't get detached
    heart_handle: Option<JoinHandle<()>>,
}

impl HeartbeatLogger {
    pub fn new(statistic_id: StatisticId, timeout_interval: Duration, logger: impl TimeFrameLogger + 'static) -> Self {
        HeartbeatLogger {
            stat_id: statistic_id,
            interval: timeout_interval,
            logger: Arc::new(logger),
            sender: None,
            heart_handle: None,
        }
    }

    pub fn new_with_local_logger(statistic_id: StatisticId, interval: std::time::Duration) -> Self {
        Self::new(statistic_id, interval, LocalLogger)
    }

    pub fn add_state_watcher(&self) -> Self {
        unimplemented!();
    }

    pub fn add_duration_watcher(&self) -> Self {
        unimplemented!();
    }

    pub fn beat(&mut self) {
        match &self.sender {
            Some(sender) if sender.send(HeartbeatMessage::Beat).is_ok() => {}
            _ => self.create_heart(),
        }
    }

    pub fn stop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(HeartbeatMessage::Stop);
        }

        if let Some(heart_handle) = self.heart_handle.take() {
            let _ = heart_handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(heart_handle) = &self.heart_handle {
            !heart_handle.is_finished()
        } else {
            false
        }
    }

    fn create_heart(&mut self) {
        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);

        let stat_id = self.stat_id;
        let logger = Arc::clone(&self.logger);
        let interval = self.interval;
        self.heart_handle = Some(thread::spawn(move || {
            Self::heart_thread(receiver, interval, stat_id, logger);
        }));
    }

    fn heart_thread(
        receiver: Receiver<HeartbeatMessage>,
        time_out_interval: Duration,
        statistic_id: StatisticId,
        logger: Arc<dyn TimeFrameLogger>,
    ) {
        let start_time = get_current_time();
        let mut last_beat = get_current_instant();
        let mut last_log = last_beat; // value doesn't matter

        Self::log_until_now(statistic_id, start_time, &logger, &mut last_log);

        loop {
            let wait_duration =
                Self::calculate_next_wait_duration(last_beat, time_out_interval, last_log);
            match receiver.recv_timeout(wait_duration) {
                Ok(HeartbeatMessage::Stop) => {
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    if Self::timer_ran_out(last_beat, time_out_interval) {
                        break;
                    }
                    Self::log_until_now(statistic_id, start_time, &logger, &mut last_log);
                }
                Ok(HeartbeatMessage::Beat) => {
                    last_beat = get_current_instant();
                }
                Err(RecvTimeoutError::Disconnected) => {
                    Self::wait_until_timer_runs_out(last_beat, time_out_interval);
                    break;
                }
            }
        }

        Self::log_until_now(statistic_id, start_time, &logger, &mut last_log);
    }

    fn log_until_now(
        statistic_id: StatisticId,
        start_time: NaiveTime,
        logger: &Arc<dyn TimeFrameLogger>,
        last_log: &mut Instant,
    ) {
        logger
            .log(
                TimeFrame::new(start_time, get_current_time()).unwrap(),
                statistic_id,
            )
            .unwrap();
        *last_log = get_current_instant();
    }

    fn calculate_next_wait_duration(
        last_beat: Instant,
        interval: Duration,
        last_log: Instant,
    ) -> Duration {
        let timer_expiration_time = last_beat + interval;
        let next_log = last_log + LOG_FREQUENCY;

        next_log
            .min(timer_expiration_time)
            .duration_since(get_current_instant())
    }

    fn wait_until_timer_runs_out(last_beat: Instant, interval: Duration) {
        let expiration_time = last_beat + interval;
        thread::sleep(expiration_time.duration_since(last_beat));
    }

    fn timer_ran_out(last_beat: Instant, interval: Duration) -> bool {
        let expiration_time = last_beat + interval;
        get_current_instant() >= expiration_time
    }
}

pub trait TimeFrameLogger: Send + Sync {
    fn log(&self, time_frame: TimeFrame, statistic_id: StatisticId) -> Result<()>;
}

pub struct LocalLogger;
impl TimeFrameLogger for LocalLogger {
    fn log(&self, time_frame: TimeFrame, statistic_id: StatisticId) -> Result<()> {
        let stat = get_statistic_by_id(statistic_id)?
            .ok_or_else(|| anyhow!("Statistic not found: {}", statistic_id))?;

        let conn = open_db()?;
        let date = Local::now().date_naive();

        match stat {
            AnyStatistic::TimeSpans(s) => {
                s.log_value(
                    &conn,
                    date,
                    time_frame,
                    TimeFrameListUpdatePolicy::OverwriteLast,
                )?;
            }
            AnyStatistic::TimeAmount(_s) => {
                unimplemented!();
                // TODO: need to sum in this case instead,
                //       because there might already be logged something outside of this time_frame
                // let value_to_log = time_frame.duration();
                // s.log_value(&conn, date, value_to_log, AmountUpdatePolicy::Overwrite)?;
            }
            // TODO: Consider creating an enum with allowed statistic types and then passing the
            //       upcasted statistic
            _ => return Err(anyhow!("Statistic is not a TimeSpansStatistic!")),
        }

        Ok(())
    }
}

pub fn get_current_instant() -> Instant {
    Instant::now()
}
fn get_current_time() -> NaiveTime {
    Local::now().time()
}
