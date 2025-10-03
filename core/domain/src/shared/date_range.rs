use anyhow::anyhow;
use chrono::NaiveDate;

/// Represents the closed interval [start, end]
/// Ensures the invariant start <= end
#[derive(Debug, Copy, Clone)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

impl DateRange {
    pub fn start(&self) -> NaiveDate {
        self.start
    }
    pub fn end(&self) -> NaiveDate {
        self.end
    }
    pub fn new(start: NaiveDate, end: NaiveDate) -> anyhow::Result<Self> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(anyhow!("invalid date range"))
        }
    }
}
