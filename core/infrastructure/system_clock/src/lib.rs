use chrono::{DateTime, Utc};
use lifestats_application::integrations::fitbit::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_recent_timestamp() {
        let clock = SystemClock;
        let now = Utc::now();
        let value = clock.now_utc();
        assert!((now - value).num_seconds().abs() < 2);
    }
}
