use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PlotId(pub u32);

impl fmt::Display for PlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for PlotId {
    fn from(id: u32) -> Self {
        PlotId(id)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlotMetadata {
    pub id: PlotId,
    pub title: String,
}