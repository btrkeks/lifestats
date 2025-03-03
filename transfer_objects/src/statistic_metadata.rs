use crate::statistic_id::StatisticId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatisticMetadata {
    id: StatisticId,
    category: Box<str>,
    name: Box<str>,
}

impl StatisticMetadata {
    pub fn new(id: StatisticId, category: Box<str>, name: Box<str>) -> Self {
        Self { id, category, name }
    }

    pub fn id(&self) -> StatisticId {
        self.id
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
