use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::progress::error::{validation_error, ProgressResult};
use crate::progress::unit::ProgressUnit;
use crate::shared::ids::ProgressItemId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressMetadata {
    name: String,
    category: String,
    tags: HashMap<String, String>,
}

impl ProgressMetadata {
    pub fn new(
        name: String,
        category: String,
        tags: HashMap<String, String>,
    ) -> ProgressResult<Self> {
        if name.trim().is_empty() {
            return Err(validation_error("progress item name must not be empty"));
        }
        if category.trim().is_empty() {
            return Err(validation_error("progress item category must not be empty"));
        }

        Ok(Self {
            name: name.trim().to_string(),
            category: category.trim().to_string(),
            tags,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressTarget {
    value: i64,
    unit: ProgressUnit,
}

impl ProgressTarget {
    pub fn new(value: i64, unit: ProgressUnit) -> ProgressResult<Self> {
        if value <= 0 {
            return Err(validation_error("progress target value must be positive"));
        }
        unit.validate()?;
        Ok(Self { value, unit })
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn unit(&self) -> &ProgressUnit {
        &self.unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressItem {
    id: ProgressItemId,
    metadata: ProgressMetadata,
    target: ProgressTarget,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProgressItem {
    pub fn new(
        id: ProgressItemId,
        metadata: ProgressMetadata,
        target: ProgressTarget,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> ProgressResult<Self> {
        Ok(Self {
            id,
            metadata,
            target,
            created_at,
            updated_at,
        })
    }

    pub fn id(&self) -> ProgressItemId {
        self.id
    }

    pub fn metadata(&self) -> &ProgressMetadata {
        &self.metadata
    }

    pub fn target(&self) -> &ProgressTarget {
        &self.target
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn with_target_value(
        self,
        target_value: i64,
        updated_at: DateTime<Utc>,
    ) -> ProgressResult<Self> {
        let target = ProgressTarget::new(target_value, self.target.unit().clone())?;
        Ok(Self {
            target,
            updated_at,
            ..self
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProgressItem {
    metadata: ProgressMetadata,
    target: ProgressTarget,
    created_at: DateTime<Utc>,
}

impl NewProgressItem {
    pub fn new(
        metadata: ProgressMetadata,
        target: ProgressTarget,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            metadata,
            target,
            created_at,
        }
    }

    pub fn metadata(&self) -> &ProgressMetadata {
        &self.metadata
    }

    pub fn target(&self) -> &ProgressTarget {
        &self.target
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
