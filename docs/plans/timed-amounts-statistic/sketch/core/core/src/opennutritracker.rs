//! NEW FILE. OpenNutriTracker's complete intake day and its atomic store port.
//! Mirrors `beurer.rs` / `yomikata.rs`: domain request, error and port live here;
//! SQL lives in `persistence/src/opennutritracker_day_store.rs`.
use chrono::NaiveDate;
use crate::{shared::{ids::StatisticId, ports::error::RepositoryError, timed_amount_list::TimedAmountList},
    statistic::types::logable_statistics::timed_amounts::TimedAmountsStatistic};

pub const SOURCE: &str = "opennutritracker";

/// The ONE identity definition for this source's statistics. Creation
/// (`new_statistic`) and the store's lookup both read `tag()`, so a statistic the
/// store inserts is always the one it finds on the next PUT.
/// Adding a metric: add a variant; `tag`/`display_name` force the rest, and
/// `OpenNutriTrackerDay` gains a named field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::EnumIter)]
pub enum OpenNutriTrackerMetric {
    Caffeine,
    Nicotine,
}
impl OpenNutriTrackerMetric {
    /// Every metric, derived from the enum (no hand-kept list; callers need no strum dep).
    pub fn all() -> impl Iterator<Item = Self> { <Self as strum::IntoEnumIterator>::iter() }
    /// Value of the `metric` tag; `source` tag is always `SOURCE`.
    pub const fn tag(self) -> &'static str {
        match self { Self::Caffeine => "caffeine", Self::Nicotine => "nicotine" }
    }
    /// Initial name only (repo convention `[Source] Metric (unit)`); never identity.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Caffeine => "[OpenNutriTracker] Caffeine (mg)",
            Self::Nicotine => "[OpenNutriTracker] Nicotine (mg)",
        }
    }
    /// Category "OpenNutriTracker"; tags source, metric=tag(), unit=mg, bundle_version=1;
    /// no default goal (the user chooses limits); goal history starts `today`.
    pub fn new_statistic(self, today: NaiveDate) -> TimedAmountsStatistic {
        // TODO: StatisticMetadata::new(StatisticId(0), "OpenNutriTracker", display_name())
        //   .with_tag("source", SOURCE).with_tag("metric", tag())
        //   .with_tag("unit", "mg").with_tag("bundle_version", "1")
        todo!()
    }
}

/// Both metrics are always supplied: this is the app's complete day.
/// An empty list is a durable "confirmed zero", not an omission.
#[derive(Debug, Clone)]
pub struct OpenNutriTrackerDay {
    caffeine: TimedAmountList,
    nicotine: TimedAmountList,
}
impl OpenNutriTrackerDay {
    pub fn new(caffeine: TimedAmountList, nicotine: TimedAmountList) -> Self { todo!() }
    /// Exhaustive over the enum, so a new metric cannot be silently skipped.
    pub fn list(&self, metric: OpenNutriTrackerMetric) -> &TimedAmountList { todo!() }
}
pub struct OpenNutriTrackerDayReplacement {
    pub changed_statistics: Vec<StatisticId>,
}
#[derive(Debug, thiserror::Error)]
pub enum OpenNutriTrackerDayError {
    #[error("OpenNutriTracker statistic bindings conflict")]
    BundleConflict,
    #[error("Unable to store OpenNutriTracker day")]
    Storage(#[from] RepositoryError),
}
pub trait OpenNutriTrackerDayStore: Send + Sync {
    /// One immediate transaction: for every `OpenNutriTrackerMetric`, resolve the
    /// statistic by (source, metric tag) — >1 match, wrong type or unit != mg →
    /// BundleConflict; none → insert `metric.new_statistic(today)` — then replace
    /// that metric's list on `day`. Stores `[]`; never touches other days, names
    /// or goals. Reordered equal content is unchanged; duplicate entries are real
    /// separate intakes. Returns changed IDs; caller publishes after commit.
    /// Last write wins: no revision or freshness check.
    fn replace_day(&self, day: NaiveDate, today: NaiveDate, values: OpenNutriTrackerDay)
        -> Result<OpenNutriTrackerDayReplacement, OpenNutriTrackerDayError>;
}
