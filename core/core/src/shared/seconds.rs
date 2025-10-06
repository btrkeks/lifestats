use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seconds(u64);

impl Seconds {
    /// Create a new Seconds value from i64
    pub fn from_i64(seconds: i64) -> Self {
        Self(seconds.max(0) as u64)
    }
}

impl From<Seconds> for i64 {
    fn from(val: Seconds) -> Self {
        val.0 as i64
    }
}

impl From<chrono::Duration> for Seconds {
    fn from(value: chrono::Duration) -> Self {
        Self(value.num_seconds() as u64)
    }
}
