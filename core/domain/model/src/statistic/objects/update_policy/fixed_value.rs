use crate::statistic::objects::update_policy::UpdatePolicy;
use log::info;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum FixedValueUpdatePolicy {
    #[default]
    Ignore,
    Overwrite,
}

impl<T: std::fmt::Display> UpdatePolicy<T, T> for FixedValueUpdatePolicy {
    fn apply(&self, existing: Option<T>, new: T) -> T {
        match self {
            FixedValueUpdatePolicy::Overwrite => {
                if existing.is_some() {
                    info!("Overwriting {} with {}", existing.unwrap(), new);
                }

                new
            }
            FixedValueUpdatePolicy::Ignore => {
                if existing.is_some() {
                    info!("Ignoring new value {}", new);
                }

                existing.unwrap_or(new)
            }
        }
    }
}
