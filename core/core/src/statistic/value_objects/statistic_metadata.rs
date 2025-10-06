use crate::shared::ids::StatisticId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatisticMetadata {
    id: StatisticId,
    category: Box<str>,
    name: Box<str>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

impl StatisticMetadata {
    pub fn new(id: StatisticId, category: Box<str>, name: Box<str>) -> Self {
        Self {
            id,
            category,
            name,
            tags: HashMap::new(),
        }
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

    /// Builder-style method to add a tag to the metadata.
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    /// Gets the value of a tag by key.
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }

    /// Gets a reference to all tags.
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}
