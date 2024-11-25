use strum_macros::{Display, EnumString};
use crate::objects::update_policies::UpdatePolicy;

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum FixedValueUpdatePolicy {
    #[default]
    Ignore,
    Overwrite,
}

impl<T> UpdatePolicy<T, T> for FixedValueUpdatePolicy {
    fn apply(&self, existing: Option<T>, new: T) -> T {
        match self {
            FixedValueUpdatePolicy::Overwrite => new,
            FixedValueUpdatePolicy::Ignore => existing.unwrap_or(new),
        }
    }
}