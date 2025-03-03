use anyhow::Result;
use chrono::NaiveTime;

pub trait TimeFrameLogger: Clone + Send + Sync {
    fn log(&self, start_time: NaiveTime, end_time: NaiveTime) -> Result<()>;
}
