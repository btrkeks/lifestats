use crate::error::ApplicationResult;
use chrono::{NaiveDate, NaiveDateTime};

#[cfg(test)]
use mockall::automock;

/// A completed LiftLog workout session extracted from a backup snapshot.
///
/// Sessions without any completed set are filtered out by the parser, so
/// `started_at`/`finished_at` are always available.
#[derive(Debug, Clone, PartialEq)]
pub struct LiftLogWorkout {
    /// LiftLog's session UUID.
    pub session_id: String,
    /// The calendar date LiftLog attributes the session to.
    pub date: NaiveDate,
    /// Local wall-clock time of the first completed set.
    pub started_at: NaiveDateTime,
    /// Local wall-clock time of the last completed set.
    pub finished_at: NaiveDateTime,
    /// Total weight lifted in kilograms (effective weight x reps, including
    /// bodyweight for bodyweight-based exercises).
    pub total_weight_kg: f64,
}

/// Port for parsing a gzipped LiftLog remote-backup payload (a serialized
/// SQLite snapshot) into workout facts. Implemented in infrastructure.
#[cfg_attr(test, automock)]
pub trait LiftLogBackupParser: Send + Sync {
    fn parse(&self, gzipped_backup: &[u8]) -> ApplicationResult<Vec<LiftLogWorkout>>;
}
