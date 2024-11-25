use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StatisticId(pub u32);

impl fmt::Display for StatisticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for StatisticId {
    fn from(id: u32) -> Self {
        StatisticId(id)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatisticMetadata {
    pub id: StatisticId,
    pub category: String,
    pub name: String,
}