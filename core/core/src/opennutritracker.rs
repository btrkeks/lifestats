use chrono::NaiveDate;
use thiserror::Error;

use crate::shared::{
    ids::StatisticId, ports::error::RepositoryError, timed_amount_list::TimedAmountList,
};
use crate::statistic::types::logable_statistics::timed_amounts::TimedAmountsStatistic;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;

/// Value of the `source` tag on every OpenNutriTracker statistic.
pub const SOURCE: &str = "opennutritracker";
pub const CATEGORY: &str = "OpenNutriTracker";
/// Every metric is stored in milligrams; recorded as the `unit` tag.
pub const UNIT: &str = "mg";

/// The one identity definition for this source's statistics. Creation
/// (`new_statistic`) and the store's lookup both use `tag()`, so a statistic the
/// store creates is always the one it finds on the next sync.
///
/// Adding a metric: add a variant (the matches below force its tag and name) and a
/// named field on `OpenNutriTrackerDay`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::EnumIter)]
pub enum OpenNutriTrackerMetric {
    Caffeine,
    Nicotine,
}

impl OpenNutriTrackerMetric {
    pub fn all() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }

    /// Value of the `metric` tag.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Caffeine => "caffeine",
            Self::Nicotine => "nicotine",
        }
    }

    /// Initial name only; renaming the statistic later is fine because identity is the tags.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Caffeine => "[OpenNutriTracker] Caffeine (mg)",
            Self::Nicotine => "[OpenNutriTracker] Nicotine (mg)",
        }
    }

    /// The statistic created on first sync: no default goal, so the user picks limits.
    pub fn new_statistic(self, today: NaiveDate) -> TimedAmountsStatistic {
        let metadata =
            StatisticMetadata::new(StatisticId(0), CATEGORY.into(), self.display_name().into())
                .with_tag("source".into(), SOURCE.into())
                .with_tag("metric".into(), self.tag().into())
                .with_tag("unit".into(), UNIT.into())
                .with_tag("bundle_version".into(), "1".into());
        TimedAmountsStatistic::new(metadata, None, today)
    }
}

/// The app's complete day for every metric. An empty list is a confirmed zero,
/// not an omission, so both metrics are always present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenNutriTrackerDay {
    caffeine: TimedAmountList,
    nicotine: TimedAmountList,
}

impl OpenNutriTrackerDay {
    pub fn new(caffeine: TimedAmountList, nicotine: TimedAmountList) -> Self {
        Self { caffeine, nicotine }
    }

    pub fn list(&self, metric: OpenNutriTrackerMetric) -> &TimedAmountList {
        match metric {
            OpenNutriTrackerMetric::Caffeine => &self.caffeine,
            OpenNutriTrackerMetric::Nicotine => &self.nicotine,
        }
    }
}

#[derive(Debug, Error)]
pub enum OpenNutriTrackerDayError {
    #[error("OpenNutriTracker statistic bindings conflict")]
    BundleConflict,
    #[error("Daily intake could not be stored")]
    Storage(#[from] RepositoryError),
}

pub struct OpenNutriTrackerDayReplacement {
    pub changed_statistics: Vec<StatisticId>,
}

pub trait OpenNutriTrackerDayStore: Send + Sync {
    /// In one transaction, for every metric: resolve its statistic by the `source`
    /// and `metric` tags (several matches, another type, or a unit other than mg is
    /// a `BundleConflict`; none creates `metric.new_statistic(today)`), then replace
    /// that metric's list on `day`. Stores `[]`; never touches other days, names or
    /// goals. An equal list is unchanged. Last write wins: there is no freshness check.
    fn replace_day(
        &self,
        day: NaiveDate,
        today: NaiveDate,
        values: OpenNutriTrackerDay,
    ) -> Result<OpenNutriTrackerDayReplacement, OpenNutriTrackerDayError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_statistics_carry_the_identity_the_store_looks_up() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        for metric in OpenNutriTrackerMetric::all() {
            let statistic = metric.new_statistic(today);
            let metadata =
                crate::statistic::traits::base_statistic::Statistic::get_metadata(&statistic);
            assert_eq!(metadata.get_tag("source").map(String::as_str), Some(SOURCE));
            assert_eq!(
                metadata.get_tag("metric").map(String::as_str),
                Some(metric.tag())
            );
            assert_eq!(metadata.get_tag("unit").map(String::as_str), Some(UNIT));
            assert_eq!(metadata.category(), CATEGORY);
        }
        assert_eq!(OpenNutriTrackerMetric::all().count(), 2);
    }
}
