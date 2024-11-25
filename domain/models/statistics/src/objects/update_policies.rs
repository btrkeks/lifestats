use std::fmt::Display;
use std::str::FromStr;

pub mod amount;
pub mod fixed_value;
pub mod time_frame_list;

pub trait UpdatePolicy<T,U>: Display + Default + FromStr {
    fn apply(&self, existing: Option<T>, new: U) -> T;
}