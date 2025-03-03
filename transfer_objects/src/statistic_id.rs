use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StatisticId(pub u32);

impl From<u32> for StatisticId {
    fn from(id: u32) -> Self {
        StatisticId(id)
    }
}

impl fmt::Display for StatisticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PlotId(pub u32);

impl From<u32> for PlotId {
    fn from(id: u32) -> Self {
        PlotId(id)
    }
}

impl fmt::Display for PlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
