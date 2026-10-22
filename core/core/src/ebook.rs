use chrono::{DateTime, NaiveDate, Utc};
use std::{collections::BTreeMap, num::NonZeroU64};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{
    shared::{ids::StatisticId, ports::error::RepositoryError},
    statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProducerId(Uuid);
impl ProducerId {
    pub fn parse(raw: &str) -> Result<Self, EbookError> {
        Uuid::parse_str(raw)
            .ok()
            .filter(|id| id.to_string() == raw)
            .map(Self)
            .ok_or(EbookError::invalid(
                "producer",
                "Expected a canonical lowercase UUID",
            ))
    }
}
impl std::fmt::Display for ProducerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BookKey(String);
impl BookKey {
    pub fn parse(raw: &str) -> Result<Self, EbookError> {
        let digest = raw.strip_prefix("ttu-text-v1:").ok_or(EbookError::invalid(
            "book_key",
            "Expected ttu-text-v1: followed by 64 lowercase hex characters",
        ))?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(EbookError::invalid(
                "book_key",
                "Expected ttu-text-v1: followed by 64 lowercase hex characters",
            ));
        }
        Ok(Self(raw.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookTitle(String);
impl BookTitle {
    pub fn parse(raw: &str) -> Result<Self, EbookError> {
        let title: String = raw.trim().nfc().collect();
        if title.is_empty() || title.chars().count() > 1000 {
            return Err(EbookError::invalid(
                "title",
                "Required nonblank title of at most 1000 characters",
            ));
        }
        Ok(Self(title))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpanSet(Vec<(u64, u64)>);
impl SpanSet {
    pub const MAX_SPANS: usize = 4096;
    pub const MAX_OFFSET: u64 = 1 << 53;

    pub fn from_normalized(pairs: Vec<(u64, u64)>) -> Result<Self, EbookError> {
        if pairs.len() > Self::MAX_SPANS
            || pairs.iter().any(|&(a, b)| a >= b || b > Self::MAX_OFFSET)
            || pairs.windows(2).any(|w| w[0].1 >= w[1].0)
        {
            return Err(EbookError::invalid(
                "spans",
                "Expected at most 4096 sorted, disjoint, non-adjacent half-open spans",
            ));
        }
        Ok(Self(pairs))
    }
    pub fn from_stored(mut pairs: Vec<(u64, u64)>) -> Result<Self, EbookError> {
        if pairs.iter().any(|&(a, b)| a > b || b > Self::MAX_OFFSET) {
            return Err(EbookError::invalid("spans", "Invalid span bounds"));
        }
        pairs.sort_unstable();
        let mut out: Vec<(u64, u64)> = Vec::new();
        for (a, b) in pairs {
            if a == b {
                continue;
            }
            match out.last_mut() {
                Some(last) if a <= last.1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        Ok(Self(out))
    }
    pub fn union(&self, other: &Self) -> Self {
        let mut out: Vec<(u64, u64)> = Vec::with_capacity(self.0.len() + other.0.len());
        let (mut a, mut b) = (self.0.iter().peekable(), other.0.iter().peekable());
        while a.peek().is_some() || b.peek().is_some() {
            let &(start, end) =
                if b.peek().is_none() || (a.peek().is_some() && a.peek() <= b.peek()) {
                    a.next().unwrap()
                } else {
                    b.next().unwrap()
                };
            match out.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => out.push((start, end)),
            }
        }
        Self(out)
    }
    pub fn len(&self) -> u64 {
        self.0.iter().map(|&(a, b)| b - a).sum()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn end(&self) -> u64 {
        self.0.last().map_or(0, |p| p.1)
    }
    pub fn pairs(&self) -> &[(u64, u64)] {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct EbookDayReport {
    pub title: BookTitle,
    pub length: NonZeroU64,
    pub spans: SpanSet,
    pub last_position: u64,
    pub observed_at: DateTime<Utc>,
}
impl EbookDayReport {
    pub fn new(
        title: BookTitle,
        length: u64,
        spans: SpanSet,
        last_position: u64,
        observed_at: DateTime<Utc>,
    ) -> Result<Self, EbookError> {
        let length = parse_length(length)?;
        if spans.end() > length.get() {
            return Err(EbookError::invalid(
                "spans",
                "Spans must be within [0,length)",
            ));
        }
        if last_position > length.get() {
            return Err(EbookError::invalid(
                "last_position",
                "Position must be within [0,length]",
            ));
        }
        Ok(Self {
            title,
            length,
            spans,
            last_position,
            observed_at,
        })
    }
}
pub fn parse_length(value: u64) -> Result<NonZeroU64, EbookError> {
    NonZeroU64::new(value)
        .filter(|n| n.get() <= SpanSet::MAX_OFFSET)
        .ok_or(EbookError::invalid(
            "length",
            "Required positive integer at most 9007199254740992",
        ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDay {
    pub spans: SpanSet,
    pub last_position: u64,
    pub observed_at: DateTime<Utc>,
}
pub fn merge_day(stored: Option<&StoredDay>, report: &EbookDayReport) -> Option<StoredDay> {
    let mut merged = StoredDay {
        spans: report.spans.clone(),
        last_position: report.last_position,
        observed_at: report.observed_at,
    };
    if let Some(old) = stored {
        merged.spans = old.spans.union(&report.spans);
        if report.observed_at <= old.observed_at {
            merged.last_position = old.last_position;
            merged.observed_at = old.observed_at;
        }
        if &merged == old {
            return None;
        }
    }
    Some(merged)
}

pub fn credit_days<'a>(
    legacy: &SpanSet,
    rows: impl IntoIterator<Item = (NaiveDate, &'a SpanSet)>,
) -> BTreeMap<NaiveDate, u64> {
    let mut grouped: BTreeMap<NaiveDate, SpanSet> = BTreeMap::new();
    for (day, spans) in rows {
        let entry = grouped.entry(day).or_default();
        *entry = entry.union(spans);
    }
    let mut union = legacy.clone();
    grouped
        .into_iter()
        .map(|(day, spans)| {
            let previous = union.len();
            union = union.union(&spans);
            (day, union.len() - previous)
        })
        .collect()
}

pub struct EbookBundleDefinitions {
    pub characters_read: IntegerAmountStatistic,
}
#[derive(Debug, Error)]
pub enum EbookError {
    #[error("{field}: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("The resulting aggregate exceeds the supported integer range")]
    ProjectionOverflow,
    #[error("Ebook statistic bindings conflict")]
    BundleConflict,
    #[error("Book length differs from the stored length")]
    BookLengthMismatch,
    #[error("A matching legacy book must be adopted first")]
    LegacyAdoptionPending,
    #[error("Legacy book binding conflicts with an existing binding")]
    LegacyBindingConflict,
    #[error("Ebook reading could not be stored")]
    Storage(#[from] RepositoryError),
}
impl EbookError {
    pub fn invalid(field: &'static str, reason: &'static str) -> Self {
        Self::InvalidField { field, reason }
    }
}
impl From<crate::shared::projection::ProjectionOverflow> for EbookError {
    fn from(_: crate::shared::projection::ProjectionOverflow) -> Self {
        Self::ProjectionOverflow
    }
}
pub struct EbookWriteOutcome {
    pub changed_statistics: Vec<StatisticId>,
}
pub trait EbookReadingStore: Send + Sync {
    fn server_id(&self) -> Result<String, EbookError>;
    fn record_day(
        &self,
        producer: ProducerId,
        book: BookKey,
        day: NaiveDate,
        report: EbookDayReport,
        definitions: EbookBundleDefinitions,
    ) -> Result<EbookWriteOutcome, EbookError>;
    fn bind_legacy_book(
        &self,
        reader_data_id: u32,
        book: BookKey,
        title: BookTitle,
        length: NonZeroU64,
        definitions: EbookBundleDefinitions,
    ) -> Result<EbookWriteOutcome, EbookError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Cases {
        normalization: Vec<Normalization>,
        union: Vec<Union>,
        credit_days: Vec<Credits>,
    }
    #[derive(Deserialize)]
    struct Normalization {
        input: Vec<(u64, u64)>,
        normalized: Vec<(u64, u64)>,
        size: u64,
    }
    #[derive(Deserialize)]
    struct Union {
        a: Vec<(u64, u64)>,
        b: Vec<(u64, u64)>,
        union: Vec<(u64, u64)>,
        size: u64,
    }
    #[derive(Deserialize)]
    struct Credits {
        legacy: Vec<(u64, u64)>,
        rows: Vec<Row>,
        credits: BTreeMap<NaiveDate, u64>,
    }
    #[derive(Deserialize)]
    struct Row {
        day: NaiveDate,
        spans: Vec<(u64, u64)>,
    }
    fn spans(p: Vec<(u64, u64)>) -> SpanSet {
        SpanSet::from_stored(p).unwrap()
    }
    #[test]
    fn ebook_shared_cases_and_union_laws() {
        let cases: Cases =
            serde_json::from_str(include_str!("../../../docs/ebook-reading/span-cases.json"))
                .unwrap();
        for c in cases.normalization {
            let s = spans(c.input);
            assert_eq!(s.pairs(), c.normalized);
            assert_eq!(s.len(), c.size);
            assert_eq!(s.union(&s), s);
        }
        for c in cases.union {
            let (a, b) = (spans(c.a), spans(c.b));
            let u = a.union(&b);
            assert_eq!(u.pairs(), c.union);
            assert_eq!(u.len(), c.size);
            assert_eq!(u, b.union(&a));
            assert_eq!(u.union(&a), u);
        }
        for c in cases.credit_days {
            let legacy = spans(c.legacy);
            let rows: Vec<_> = c
                .rows
                .into_iter()
                .map(|r| (r.day, spans(r.spans)))
                .collect();
            assert_eq!(
                credit_days(&legacy, rows.iter().map(|(d, s)| (*d, s))),
                c.credits
            );
            assert_eq!(
                credit_days(
                    &legacy,
                    rows.iter().rev().chain(rows.iter()).map(|(d, s)| (*d, s))
                ),
                c.credits
            );
        }
    }
    #[test]
    fn ebook_credit_order_duplication_and_associativity() {
        for seed in 0..100u64 {
            let a = spans(vec![(seed, seed + 80)]);
            let b = spans(vec![(seed * 3, seed * 3 + 50)]);
            let c = spans(vec![(seed * 2, seed * 2 + 90)]);
            assert_eq!(a.union(&b).union(&c), a.union(&b.union(&c)));
            let mon = "2026-10-19".parse().unwrap();
            let tue = "2026-10-20".parse().unwrap();
            let expected = credit_days(&SpanSet::default(), [(mon, &a), (tue, &b), (mon, &c)]);
            assert_eq!(
                expected,
                credit_days(
                    &SpanSet::default(),
                    [(tue, &b), (mon, &c), (mon, &a), (tue, &b)]
                )
            );
            assert_eq!(expected.values().sum::<u64>(), a.union(&b).union(&c).len());
        }
    }
    #[test]
    fn ebook_merge_replay_and_subset_are_noops() {
        let report = EbookDayReport::new(
            BookTitle::parse("Book").unwrap(),
            1000,
            spans(vec![(0, 100)]),
            100,
            "2026-10-19T12:00:00Z".parse().unwrap(),
        )
        .unwrap();
        let stored = merge_day(None, &report).unwrap();
        assert_eq!(merge_day(Some(&stored), &report), None);
        let stale = EbookDayReport {
            spans: spans(vec![(20, 80)]),
            observed_at: report.observed_at - chrono::Duration::seconds(1),
            ..report.clone()
        };
        assert_eq!(merge_day(Some(&stored), &stale), None);
        let newer = EbookDayReport {
            last_position: 50,
            observed_at: report.observed_at + chrono::Duration::seconds(1),
            ..report
        };
        assert_eq!(merge_day(Some(&stored), &newer).unwrap().last_position, 50);
    }
    #[test]
    fn ebook_boundary_types_reject_invalid_input() {
        for p in [
            vec![(0, 1), (1, 2)],
            vec![(10, 20), (0, 5)],
            vec![(0, 2), (1, 3)],
            vec![(2, 2)],
            vec![(0, SpanSet::MAX_OFFSET + 1)],
            vec![(0, 1); 4097],
        ] {
            assert!(SpanSet::from_normalized(p).is_err());
        }
        assert!(ProducerId::parse("0B6C3F0E-2A71-4D38-9F43-5A1E8C2D9B77").is_err());
        assert!(BookKey::parse(&"a".repeat(64)).is_err());
        assert_eq!(BookTitle::parse(" e\u{301} ").unwrap().as_str(), "é");
        assert!(BookTitle::parse(" ").is_err());
        assert!(parse_length(0).is_err());
    }
}
