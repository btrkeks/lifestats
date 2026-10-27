// EXCERPT — core/application/src/dto/statistic.rs
//
// Hardening in the same change: the string parser is derived from the variant
// list, so a new variant can no longer be printable but unparseable
// (today's `from_str` is a hand-written string match; gotcha #4).
// Needs `strum = "0.27"`, `strum_macros = "0.27"` in core/application/Cargo.toml
// (same versions as core/core).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum_macros::EnumIter)]
pub enum StatisticType {
    IntegerAmount,
    Habit,
    PointInTime,
    TimeSpans,
    TimedAmounts, // + NEW
    Summing,
    TimeDifference,
}

impl StatisticType {
    pub fn as_str(&self) -> &'static str {
        match self {
            // ...
            StatisticType::TimedAmounts => "TimedAmounts", // CE
            // ...
        }
    }

    /// Whether it appears in GET /statistics/loggable and accepts
    /// POST /statistics/{id}/log. Exhaustive: replaces the hard-coded
    /// LOGGABLE_TYPES allowlist in get_loggable_metadata.rs (today a silent miss).
    pub fn is_manually_loggable(self) -> bool {
        match self {
            Self::IntegerAmount | Self::Habit | Self::PointInTime | Self::TimeSpans => true,
            // Source-owned: written only by its integration's day replace.
            Self::TimedAmounts => false,
            Self::Summing | Self::TimeDifference => false,
        }
    }
}

impl FromStr for StatisticType {
    type Err = StatisticTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use strum::IntoEnumIterator;
        Self::iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| StatisticTypeParseError::new(s))
    }
}

impl From<&AnyStatistic> for StatisticSummary {
    fn from(statistic: &AnyStatistic) -> Self {
        let statistic_type = match statistic {
            // ...
            AnyStatistic::TimedAmounts(_) => StatisticType::TimedAmounts, // CE
            // ...
        };
        // ...
    }
}

// Round-trip test (:196): iterate StatisticType::iter() instead of a literal array.
