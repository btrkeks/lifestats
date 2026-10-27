use std::str::FromStr;
use super::update_policy_trait::UpdatePolicy;
/// Only operation: replace the complete value. Default cannot ignore or append.
#[derive(Debug, Clone, Copy, Default)]
pub struct Replace;
impl FromStr for Replace {
    type Err = crate::statistic::error::DomainError;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        // TODO: accept case-insensitive "overwrite" only, reject other policies.
        todo!()
    }
}
impl<T: PartialEq> UpdatePolicy<T> for Replace {
    fn apply(&self, current: Option<T>, incoming: T) -> (T, bool) {
        // TODO: changed=current.as_ref()!=Some(&incoming); return incoming,changed.
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn replay_is_unchanged_including_canonical_lists() { todo!() }
}
