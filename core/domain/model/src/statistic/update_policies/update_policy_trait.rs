// TODO: It might make sense to split this into `UpdateStrategy` and `AllowedUpdateStrategies`.
//       `UpdateStrategy` would then provide the implementation and
//       `AllowedUpdateStrategies` would define the set of possible update operations.
//       We would then need some way to check at compile time that only allowed update strategies
//       are passed to a statistic.

use std::str::FromStr;

/// An update policy defines a set of possible update operations for updating existing data
/// with a new incoming data point and provides their implementations.
///
/// The set of allowed (or meaningful) operations can differ for each statistic.
/// E.g. while 12 o'clock and 1 o'clock can be added, their sum doesn't need to make sense.
pub trait UpdatePolicy<T>: Default + FromStr {

    /// Apply the update policy to the current value with a new value
    ///
    /// Returns:
    /// - The resulting value after applying the policy
    /// - A boolean indicating whether the value changed
    fn apply(&self, current: Option<T>, new: T) -> (T, bool);
}
