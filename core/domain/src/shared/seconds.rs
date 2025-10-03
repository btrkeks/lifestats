#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Seconds(u64);

impl Into<i64> for Seconds {
    fn into(self) -> i64 {
        self.0 as i64
    }
}

impl From<chrono::Duration> for Seconds {
    fn from(value: chrono::Duration) -> Self {
        Self(value.num_seconds() as u64)
    }
}
