//! One day of timed intake entries, e.g. caffeine in mg.
//!
//! Stored in `logging_data.value` as a bare array:
//! `[{"time":null,"amount":40},{"time":"08:12:00","amount":120}]`
//!
//! Construction, `Deserialize` and `FromStr` establish the same invariants, so a
//! value read from the database is as trustworthy as one built from a request
//! (unlike `TimeFrameList`, whose derived serde bypasses its constructors).
//!
//! `time` is the source's wall-clock time on the day the source counted the entry
//! toward. The logical-day boundary is not modelled: a 00:30 entry counted toward
//! the previous day sorts first, not last. Totals are unaffected; anything that
//! relies on order (a last-dose cutoff, decay at bedtime) must solve this first.

use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Largest amount and daily total. It is exact in JavaScript clients, and
/// multiplying it by 100 (as `AmountGoal` does) still fits in an `i64`.
pub const MAX_DAILY_AMOUNT: i64 = 9_007_199_254_740_991;
pub const MAX_DAILY_ENTRIES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimedAmountError {
    #[error("Expected HH:MM:SS local time or null")]
    InvalidTime,
    #[error("Expected a positive integer amount within the supported range")]
    InvalidAmount,
    #[error("Expected at most {MAX_DAILY_ENTRIES} entries")]
    TooManyEntries,
    #[error("Daily amount exceeds the supported integer range")]
    TotalOverflow,
}

/// One intake. Field order is the canonical sort key: time (unknown first), then amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimedAmount {
    /// `None`: the source knows the day but not the time (OpenNutriTracker's
    /// date-label entries). It counts toward the total and makes no timing claim.
    time: Option<NaiveTime>,
    amount: i64,
}

impl TimedAmount {
    /// Known times are whole-second, non-leap local times; `1 <= amount <= MAX_DAILY_AMOUNT`.
    pub fn new(time: Option<NaiveTime>, amount: i64) -> Result<Self, TimedAmountError> {
        if time.is_some_and(|time| time.nanosecond() != 0) {
            return Err(TimedAmountError::InvalidTime);
        }
        if !(1..=MAX_DAILY_AMOUNT).contains(&amount) {
            return Err(TimedAmountError::InvalidAmount);
        }
        Ok(Self { time, amount })
    }

    pub fn time(&self) -> Option<NaiveTime> {
        self.time
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }
}

/// Parses exactly `HH:MM:SS` (00:00:00..=23:59:59).
pub fn parse_entry_time(raw: &str) -> Result<NaiveTime, TimedAmountError> {
    let time =
        NaiveTime::parse_from_str(raw, "%H:%M:%S").map_err(|_| TimedAmountError::InvalidTime)?;
    // The round trip rejects single-digit fields; chrono parses "23:59:60" as a leap second.
    if time.nanosecond() != 0 || time.format("%H:%M:%S").to_string() != raw {
        return Err(TimedAmountError::InvalidTime);
    }
    Ok(time)
}

impl Serialize for TimedAmount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Stored {
            time: Option<String>,
            amount: i64,
        }
        Stored {
            time: self.time.map(|time| time.format("%H:%M:%S").to_string()),
            amount: self.amount,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TimedAmount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Stored {
            // `deserialize_with` makes the key required while still accepting null.
            #[serde(deserialize_with = "Option::deserialize")]
            time: Option<String>,
            amount: i64,
        }
        let stored = Stored::deserialize(deserializer)?;
        let time = stored
            .time
            .as_deref()
            .map(parse_entry_time)
            .transpose()
            .map_err(serde::de::Error::custom)?;
        TimedAmount::new(time, stored.amount).map_err(serde::de::Error::custom)
    }
}

/// A day's complete, canonical entry list. Immutable once built.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TimedAmountList(Vec<TimedAmount>);

impl TimedAmountList {
    /// Sorts into canonical order, so input order does not matter. Duplicates are
    /// kept: two identical entries are two intakes.
    pub fn new(mut entries: Vec<TimedAmount>) -> Result<Self, TimedAmountError> {
        if entries.len() > MAX_DAILY_ENTRIES {
            return Err(TimedAmountError::TooManyEntries);
        }
        entries
            .iter()
            .try_fold(0_i64, |total, entry| total.checked_add(entry.amount))
            .filter(|total| *total <= MAX_DAILY_AMOUNT)
            .ok_or(TimedAmountError::TotalOverflow)?;
        entries.sort();
        Ok(Self(entries))
    }

    pub fn entries(&self) -> &[TimedAmount] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The one definition of the daily total; every reader uses it.
    /// Cannot overflow, because every construction path checks the sum.
    pub fn total(&self) -> i64 {
        self.0.iter().map(|entry| entry.amount).sum()
    }
}

impl<'de> Deserialize<'de> for TimedAmountList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = Vec::<TimedAmount>::deserialize(deserializer)?;
        TimedAmountList::new(entries).map_err(serde::de::Error::custom)
    }
}

/// JSON, like `TimeFrameList`. Required by the `LogableStatistic` bound.
impl FromStr for TimedAmountList {
    type Err = serde_json::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(raw)
    }
}

impl fmt::Display for TimedAmountList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn at(hour: u32, minute: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(hour, minute, 0)
    }

    fn entry(time: Option<NaiveTime>, amount: i64) -> TimedAmount {
        TimedAmount::new(time, amount).unwrap()
    }

    #[test]
    fn input_order_does_not_matter_and_duplicates_count_twice() {
        let a = TimedAmountList::new(vec![entry(at(13, 40), 60), entry(at(8, 12), 120)]).unwrap();
        let b = TimedAmountList::new(vec![entry(at(8, 12), 120), entry(at(13, 40), 60)]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), b.to_string());
        assert_eq!(a.total(), 180);

        let gum = TimedAmountList::new(vec![entry(at(10, 0), 2), entry(at(10, 0), 2)]).unwrap();
        assert_eq!(gum.entries().len(), 2);
        assert_eq!(gum.total(), 4);
    }

    #[test]
    fn unknown_time_sorts_first_counts_and_round_trips_as_null() {
        let list = TimedAmountList::new(vec![entry(at(8, 12), 120), entry(None, 40)]).unwrap();
        assert_eq!(list.total(), 160);
        assert_eq!(
            serde_json::to_value(&list).unwrap(),
            json!([{"time": null, "amount": 40}, {"time": "08:12:00", "amount": 120}])
        );
        let parsed: TimedAmountList =
            serde_json::from_value(serde_json::to_value(&list).unwrap()).unwrap();
        assert_eq!(parsed, list);
    }

    #[test]
    fn empty_list_is_valid_and_totals_zero() {
        let list: TimedAmountList = "[]".parse().unwrap();
        assert!(list.is_empty());
        assert_eq!(list.total(), 0);
    }

    #[test]
    fn deserialization_rejects_what_the_constructor_rejects() {
        for invalid in [
            json!([{"time": "08:12:00", "amount": 0}]),
            json!([{"time": "08:12:00", "amount": -5}]),
            json!([{"time": "08:12:00", "amount": 120.0}]),
            json!([{"time": "08:12:00", "amount": "120"}]),
            json!([{"time": "08:12:00", "amount": MAX_DAILY_AMOUNT + 1}]),
            json!([{"time": "08:12", "amount": 1}]),
            json!([{"time": "8:12:00", "amount": 1}]),
            json!([{"time": "24:00:00", "amount": 1}]),
            json!([{"time": "23:59:60", "amount": 1}]),
            json!([{"time": "08:12:00.5", "amount": 1}]),
            json!([{"amount": 1}]),
            json!([{"time": null}]),
            json!([{"time": null, "amount": 1, "unit": "mg"}]),
            json!({"time": null, "amount": 1}),
            json!([
                {"time": null, "amount": MAX_DAILY_AMOUNT},
                {"time": null, "amount": 1}
            ]),
        ] {
            assert!(
                serde_json::from_value::<TimedAmountList>(invalid.clone()).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn constructor_bounds() {
        assert_eq!(
            TimedAmount::new(at(8, 0), 0),
            Err(TimedAmountError::InvalidAmount)
        );
        let leap = NaiveTime::from_hms_milli_opt(23, 59, 59, 1_500);
        assert_eq!(
            TimedAmount::new(leap, 1),
            Err(TimedAmountError::InvalidTime)
        );
        let too_many = vec![entry(None, 1); MAX_DAILY_ENTRIES + 1];
        assert_eq!(
            TimedAmountList::new(too_many),
            Err(TimedAmountError::TooManyEntries)
        );
        assert!(TimedAmountList::new(vec![entry(None, 1); MAX_DAILY_ENTRIES]).is_ok());
        let at_cap = TimedAmountList::new(vec![entry(None, MAX_DAILY_AMOUNT)]).unwrap();
        assert!(at_cap.total().checked_mul(100).is_some());
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let raw = r#"[{"time":null,"amount":1,"amount":2}]"#;
        assert!(raw.parse::<TimedAmountList>().is_err());
    }
}
