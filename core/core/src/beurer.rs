use chrono::NaiveDate;
use thiserror::Error;

use crate::shared::{ids::StatisticId, ports::error::RepositoryError};
use crate::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;

/// Daily scale levels in lossless integer storage units, never additive totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeurerDayReading {
    values: [Option<i64>; 4],
}

pub const METRICS: [&str; 4] = ["weight", "body_fat", "muscle", "body_water"];

impl BeurerDayReading {
    pub fn new(
        weight_grams: i64,
        body_fat_permille: Option<i64>,
        muscle_permille: Option<i64>,
        body_water_permille: Option<i64>,
    ) -> Result<Self, BeurerDayError> {
        if !(1..=500_000).contains(&weight_grams) {
            return Err(BeurerDayError::InvalidField {
                field: "weight_grams",
            });
        }
        for (field, value) in [
            ("body_fat_permille", body_fat_permille),
            ("muscle_permille", muscle_permille),
            ("body_water_permille", body_water_permille),
        ] {
            if value.is_some_and(|value| !(0..=1000).contains(&value)) {
                return Err(BeurerDayError::InvalidField { field });
            }
        }
        Ok(Self {
            values: [
                Some(weight_grams),
                body_fat_permille,
                muscle_permille,
                body_water_permille,
            ],
        })
    }

    pub fn values(self) -> [Option<i64>; 4] {
        self.values
    }
}

pub struct BeurerBundleDefinitions(pub [IntegerAmountStatistic; 4]);

#[derive(Debug, Error)]
pub enum BeurerDayError {
    #[error("{field} is outside the supported range")]
    InvalidField { field: &'static str },
    #[error("Beurer statistic bindings conflict")]
    BundleConflict,
    #[error("Daily readings could not be stored")]
    Storage(#[from] RepositoryError),
}

pub struct BeurerDayReplacement {
    pub changed_statistics: Vec<StatisticId>,
}

pub trait BeurerDayStore: Send + Sync {
    /// Resolve/create all four statistics and overwrite supplied levels atomically.
    fn replace_day(
        &self,
        day: NaiveDate,
        reading: BeurerDayReading,
        definitions: BeurerBundleDefinitions,
    ) -> Result<BeurerDayReplacement, BeurerDayError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_boundaries_and_preserves_omitted_metrics() {
        assert_eq!(
            BeurerDayReading::new(1, None, Some(0), Some(1000))
                .unwrap()
                .values(),
            [Some(1), None, Some(0), Some(1000)]
        );
        assert!(BeurerDayReading::new(500_000, Some(1000), None, None).is_ok());
    }

    #[test]
    fn rejects_each_out_of_range_metric() {
        for weight in [i64::MIN, 0, 500_001, i64::MAX] {
            assert!(matches!(
                BeurerDayReading::new(weight, None, None, None),
                Err(BeurerDayError::InvalidField {
                    field: "weight_grams"
                })
            ));
        }
        for value in [-1, 1001, i64::MIN, i64::MAX] {
            for (fat, muscle, water, field) in [
                (Some(value), None, None, "body_fat_permille"),
                (None, Some(value), None, "muscle_permille"),
                (None, None, Some(value), "body_water_permille"),
            ] {
                assert!(
                    matches!(BeurerDayReading::new(67_500, fat, muscle, water), Err(BeurerDayError::InvalidField { field: actual }) if actual == field)
                );
            }
        }
    }
}
