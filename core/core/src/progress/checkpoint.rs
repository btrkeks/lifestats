use chrono::{DateTime, NaiveDate, Utc};

use crate::progress::error::{validation_error, ProgressResult};
use crate::shared::ids::{ProgressCheckpointId, ProgressItemId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressCheckpoint {
    id: Option<ProgressCheckpointId>,
    item_id: ProgressItemId,
    recorded_at: DateTime<Utc>,
    effective_date: NaiveDate,
    position: i64,
    source: Option<String>,
    source_event_id: Option<String>,
    note: Option<String>,
    created_at: DateTime<Utc>,
}

impl ProgressCheckpoint {
    pub fn new(
        item_id: ProgressItemId,
        recorded_at: DateTime<Utc>,
        effective_date: NaiveDate,
        position: i64,
        source: Option<String>,
        source_event_id: Option<String>,
        note: Option<String>,
        created_at: DateTime<Utc>,
    ) -> ProgressResult<Self> {
        if position < 0 {
            return Err(validation_error(
                "progress checkpoint position must be non-negative",
            ));
        }

        let source = normalize_optional(source);
        let source_event_id = normalize_optional(source_event_id);
        if source_event_id.is_some() && source.is_none() {
            return Err(validation_error(
                "progress checkpoint source_event_id requires source",
            ));
        }

        Ok(Self {
            id: None,
            item_id,
            recorded_at,
            effective_date,
            position,
            source,
            source_event_id,
            note: normalize_optional(note),
            created_at,
        })
    }

    pub fn with_id(mut self, id: ProgressCheckpointId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn id(&self) -> Option<ProgressCheckpointId> {
        self.id
    }

    pub fn item_id(&self) -> ProgressItemId {
        self.item_id
    }

    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }

    pub fn position(&self) -> i64 {
        self.position
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn source_event_id(&self) -> Option<&str> {
        self.source_event_id.as_deref()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProgressCheckpoint {
    checkpoint: ProgressCheckpoint,
}

impl NewProgressCheckpoint {
    pub fn new(
        item_id: ProgressItemId,
        recorded_at: DateTime<Utc>,
        effective_date: NaiveDate,
        position: i64,
        source: Option<String>,
        source_event_id: Option<String>,
        note: Option<String>,
        created_at: DateTime<Utc>,
    ) -> ProgressResult<Self> {
        Ok(Self {
            checkpoint: ProgressCheckpoint::new(
                item_id,
                recorded_at,
                effective_date,
                position,
                source,
                source_event_id,
                note,
                created_at,
            )?,
        })
    }

    pub fn checkpoint(&self) -> &ProgressCheckpoint {
        &self.checkpoint
    }

    pub fn into_checkpoint(self) -> ProgressCheckpoint {
        self.checkpoint
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
