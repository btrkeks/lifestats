use crate::progress::error::{validation_error, ProgressResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProgressUnit {
    Pages,
    Characters,
    Percent,
    Chapters,
    Custom { label: String },
}

impl ProgressUnit {
    pub fn from_storage_parts(kind: &str, label: Option<String>) -> ProgressResult<Self> {
        match kind {
            "Pages" => Ok(ProgressUnit::Pages),
            "Characters" => Ok(ProgressUnit::Characters),
            "Percent" => Ok(ProgressUnit::Percent),
            "Chapters" => Ok(ProgressUnit::Chapters),
            "Custom" => {
                let label = label.ok_or_else(|| {
                    validation_error("custom progress unit requires a unit label")
                })?;
                let unit = ProgressUnit::Custom { label };
                unit.validate()?;
                Ok(unit)
            }
            other => Err(validation_error(format!(
                "unknown progress unit kind: {other}"
            ))),
        }
    }

    pub fn validate(&self) -> ProgressResult<()> {
        match self {
            ProgressUnit::Custom { label } if label.trim().is_empty() => Err(validation_error(
                "custom progress unit label must not be empty",
            )),
            _ => Ok(()),
        }
    }

    pub fn into_storage_parts(self) -> (String, Option<String>) {
        match self {
            ProgressUnit::Pages => ("Pages".to_string(), None),
            ProgressUnit::Characters => ("Characters".to_string(), None),
            ProgressUnit::Percent => ("Percent".to_string(), None),
            ProgressUnit::Chapters => ("Chapters".to_string(), None),
            ProgressUnit::Custom { label } => ("Custom".to_string(), Some(label)),
        }
    }
}
