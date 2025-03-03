use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone};
use std::sync::Mutex;
use std::time;
use std::time::Instant;

pub trait TimeProvider: Send + Sync {
    fn now(&self) -> chrono::DateTime<Local>;
    fn current_instant(&self) -> Instant;
}

// Default implementation
pub struct RealTimeProvider;

impl TimeProvider for RealTimeProvider {
    fn now(&self) -> chrono::DateTime<Local> {
        Local::now()
    }
    fn current_instant(&self) -> Instant {
        Instant::now()
    }
}

pub struct MockTimeProvider {
    current_time: Mutex<NaiveDateTime>,
    start_instant: Instant,
    elapsed: Mutex<time::Duration>,
}

impl MockTimeProvider {
    pub fn new() -> Self {
        Self {
            current_time: Mutex::new(
                NaiveDate::from_ymd_opt(2020, 10, 10)
                    .unwrap()
                    .and_hms_opt(10, 0, 0)
                    .unwrap(),
            ),
            start_instant: Instant::now(),
            elapsed: Mutex::new(time::Duration::from_secs(0)),
        }
    }

    pub fn advance(&self, duration: time::Duration) {
        // Update DateTime
        let mut current_time = self.current_time.lock().unwrap();
        *current_time += duration;

        // Update elapsed time for Instant calculations
        let mut elapsed = self.elapsed.lock().unwrap();
        *elapsed += duration
    }
}

impl TimeProvider for MockTimeProvider {
    fn now(&self) -> DateTime<Local> {
        Local
            .from_local_datetime(&self.current_time.lock().unwrap())
            .unwrap()
    }

    fn current_instant(&self) -> Instant {
        self.start_instant + *self.elapsed.lock().unwrap()
    }
}

impl Default for MockTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}
