use chrono::NaiveDate;
use thiserror::Error;

use crate::shared::ids::StatisticId;
use crate::shared::ports::error::RepositoryError;
use crate::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YomikataDayTotals {
    cards_reviewed: i64,
    study_time_seconds: i64,
}

impl YomikataDayTotals {
    pub fn new(cards_reviewed: i64, study_time_seconds: i64) -> Result<Self, YomikataDayError> {
        for (field, value) in [
            ("cards_reviewed", cards_reviewed),
            ("study_time_seconds", study_time_seconds),
        ] {
            if value < 0 {
                return Err(YomikataDayError::InvalidTotal { field });
            }
        }
        Ok(Self {
            cards_reviewed,
            study_time_seconds,
        })
    }
    pub fn cards_reviewed(self) -> i64 {
        self.cards_reviewed
    }
    pub fn study_time_seconds(self) -> i64 {
        self.study_time_seconds
    }
    pub fn study_minutes(self) -> i64 {
        self.study_time_seconds / 60
    }
}

/// Preserve all contributions outside this source, including signed corrections.
pub fn replace_source_projection(
    current: i64,
    old: i64,
    new: i64,
) -> Result<i64, YomikataDayError> {
    // The residual can exceed i64 even when the resulting aggregate fits.
    let result = i128::from(current) - i128::from(old) + i128::from(new);
    i64::try_from(result).map_err(|_| YomikataDayError::ProjectionOverflow)
}

pub struct YomikataBundleDefinitions {
    pub cards_reviewed: IntegerAmountStatistic,
    pub study_minutes: IntegerAmountStatistic,
}

#[derive(Debug, Error)]
pub enum YomikataDayError {
    #[error("{field} must be a nonnegative i64 integer")]
    InvalidTotal { field: &'static str },
    #[error("The resulting aggregate exceeds the supported integer range")]
    ProjectionOverflow,
    #[error("Yomikata statistic bindings or legacy values conflict")]
    BundleConflict,
    #[error("Daily totals could not be stored")]
    Storage(#[from] RepositoryError),
}

pub struct YomikataDayReplacement {
    pub changed_statistics: Vec<StatisticId>,
}

pub trait YomikataDayStore: Send + Sync {
    /// Resolve/adopt/create the bundle and replace the ledger and both aggregate
    /// projections in one transaction. Return only after the commit succeeds.
    fn replace_day(
        &self,
        day: NaiveDate,
        totals: YomikataDayTotals,
        definitions: YomikataBundleDefinitions,
    ) -> Result<YomikataDayReplacement, YomikataDayError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacements_preserve_external_residuals_and_check_final_range() {
        assert_eq!(replace_source_projection(15, 10, 8).unwrap(), 13);
        assert_eq!(replace_source_projection(4, 8, 0).unwrap(), -4);
        assert_eq!(replace_source_projection(i64::MIN, 1, 1).unwrap(), i64::MIN);
        assert!(replace_source_projection(i64::MAX, 0, 1).is_err());
        assert!(replace_source_projection(i64::MIN, 1, 0).is_err());
    }
    #[test]
    fn totals_keep_seconds_and_project_only_complete_minutes() {
        for (seconds, minutes) in [
            (59, 0),
            (60, 1),
            (61, 1),
            (119, 1),
            (i64::MAX, i64::MAX / 60),
        ] {
            let totals = YomikataDayTotals::new(42, seconds).unwrap();
            assert_eq!(totals.study_time_seconds(), seconds);
            assert_eq!(totals.study_minutes(), minutes);
        }
        assert!(YomikataDayTotals::new(-1, 0).is_err());
        assert!(YomikataDayTotals::new(0, -1).is_err());
    }
}
