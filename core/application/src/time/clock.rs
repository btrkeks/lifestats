use chrono::{DateTime, NaiveDate, Utc};

/// Clock abstraction for getting current date/time.
/// Enables deterministic testing by allowing time to be controlled in tests.
pub trait Clock: Send + Sync {
    /// Returns the current UTC time.
    fn now_utc(&self) -> DateTime<Utc>;

    /// Returns the current date in the local timezone.
    /// This is the canonical way to get "today" for date-based operations.
    fn today(&self) -> NaiveDate {
        self.now_utc().naive_local().date()
    }
}
