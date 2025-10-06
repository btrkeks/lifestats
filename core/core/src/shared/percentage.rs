use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PercentageError {
    #[error("Value {0} is out of range (0-100)")]
    OutOfRange(u8),
}

// TODO: u8 precision doesn't need to make sense everywhere!
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Percentage(u8);

impl Percentage {
    /// Creates a percentage as a number
    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn as_num(&self) -> u8 {
        self.0
    }

    pub fn as_fraction(&self) -> f64 {
        self.0 as f64 / 100.0
    }
}

impl TryFrom<f64> for Percentage {
    type Error = PercentageError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        // Convert fraction (0.0-1.0) to percentage (0-100)
        let percentage = (value * 100.0).round() as u8;
        Ok(Self::new(percentage))
    }
}

impl From<bool> for Percentage {
    fn from(value: bool) -> Self {
        if value {
            Self(100)
        } else {
            Self(0)
        }
    }
}
