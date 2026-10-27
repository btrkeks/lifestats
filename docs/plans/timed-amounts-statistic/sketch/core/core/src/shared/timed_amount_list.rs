//! NEW FILE. Immutable canonical observations for one day.
//! Stored in `logging_data.value` as a bare array:
//!   [{"time":null,"amount":40},{"time":"08:12:00","amount":120},{"time":"13:40:00","amount":60}]
//! Construction, FromStr and Deserialize establish the SAME invariants (unlike
//! TimeFrameList, whose derived serde bypasses its constructors).
//!
//! `time` is the source's wall-clock time on the day the source attributed the entry
//! to. Logical-day boundary is deferred: a 00:30 entry counted toward the previous
//! day sorts first, not last. Totals are unaffected; ordering-based features
//! (last-dose cutoff, decay at bedtime) must solve this before relying on order.
use chrono::NaiveTime;
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};

/// Exact in current JS clients; multiplying by 100 remains inside i64 for AmountGoal.
pub const MAX_DAILY_AMOUNT: i64 = 9_007_199_254_740_991;
pub const MAX_DAILY_ENTRIES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimedAmount {
    /// `None`: the source knows the day but not the time (OpenNutriTracker date-label
    /// entries). Counts toward the total; carries no timing claim. Serialized as null.
    time: Option<NaiveTime>,
    amount: i64,
}
impl TimedAmount {
    /// Known times are whole-second non-leap local times; 1 <= amount <= MAX_DAILY_AMOUNT.
    pub fn new(time: Option<NaiveTime>, amount: i64) -> Result<Self, TimedAmountError> { todo!() }
    pub fn time(&self) -> Option<NaiveTime> { todo!() }
    pub fn amount(&self) -> i64 { todo!() }
}
impl<'de> Deserialize<'de> for TimedAmount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // TODO: Private deny_unknown_fields input {time:Option<String>, amount:i64}; the
        // `time` key is required (null allowed, missing rejected); reject duplicate
        // fields; parse exact HH:MM:SS by formatting round-trip, then new().
        // Never derive Deserialize onto private validated fields.
        todo!()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TimedAmountList(Vec<TimedAmount>);
impl TimedAmountList {
    /// <=1000 entries; checked sum <=MAX_DAILY_AMOUNT. Sort by (time, amount) with
    /// unknown times first (Option's Ord), so equality is order-insensitive.
    /// Preserve every duplicate. No public Vec mutation/DerefMut/push API.
    pub fn new(entries: Vec<TimedAmount>) -> Result<Self, TimedAmountError> { todo!() }
    pub fn entries(&self) -> &[TimedAmount] { todo!() }
    /// Infallible because all construction paths validate the sum and no mutation escapes.
    /// This is the only sum implementation used by projections, goals and plots.
    pub fn total(&self) -> i64 { todo!() }
}
impl<'de> Deserialize<'de> for TimedAmountList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // TODO: Deserialize Vec<TimedAmount>; call Self::new, mapping validation error.
        todo!()
    }
}
impl FromStr for TimedAmountList {
    type Err = serde_json::Error;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        // TODO: serde_json::from_str through validated Deserialize; require an array.
        todo!()
    }
}
impl fmt::Display for TimedAmountList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: JSON, matching FromStr rather than TimeFrameList's human-only Display.
        todo!()
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TimedAmountError {
    #[error("Expected whole-second non-leap local time")]
    InvalidTime,
    #[error("Expected a positive amount within the supported range")]
    InvalidAmount,
    #[error("Expected at most 1000 entries")]
    TooManyEntries,
    #[error("Daily amount exceeds the supported integer range")]
    TotalOverflow,
}

#[cfg(test)]
mod tests {
    #[test]
    fn canonical_multiset_preserves_doses_and_checked_total() { todo!() }
    #[test]
    fn unknown_time_sorts_first_counts_toward_total_and_round_trips_as_null() { todo!() }
    #[test]
    fn all_decoders_enforce_the_constructor_invariants() { todo!() }
}
