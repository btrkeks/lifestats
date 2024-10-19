use crate::objects::{TimeFrame, TimeFrameList};
use std::ops::Add;
use strum_macros::{Display, EnumString};

pub trait UpdatePolicy<T, U> {
    type Output;
    fn apply(&self, existing: Option<T>, new: U) -> Self::Output;
}

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum AmountUpdatePolicy {
    #[default]
    Sum,
    Overwrite,
    Ignore,
}

impl<T> UpdatePolicy<T, T> for AmountUpdatePolicy
where
    T: Add<Output = T> + Clone + Default,
{
    type Output = T;

    fn apply(&self, existing: Option<T>, new: T) -> Self::Output {
        match self {
            AmountUpdatePolicy::Sum => existing.unwrap_or_default() + new,
            AmountUpdatePolicy::Overwrite => new,
            AmountUpdatePolicy::Ignore => existing.unwrap_or(new),
        }
    }
}

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum FixedValueUpdatePolicy {
    #[default]
    Ignore,
    Overwrite,
}

impl<T> UpdatePolicy<T, T> for FixedValueUpdatePolicy {
    type Output = T;

    fn apply(&self, existing: Option<T>, new: T) -> Self::Output {
        match self {
            FixedValueUpdatePolicy::Overwrite => new,
            FixedValueUpdatePolicy::Ignore => existing.unwrap_or(new),
        }
    }
}

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum TimeFrameListUpdatePolicy {
    #[default]
    Append,
    OverwriteLast,
    UpdateLatestEndTime,
    Ignore,
}

impl UpdatePolicy<TimeFrameList, TimeFrame> for TimeFrameListUpdatePolicy {
    type Output = TimeFrameList;

    fn apply(&self, existing: Option<TimeFrameList>, new: TimeFrame) -> Self::Output {
        let mut existing = existing.unwrap_or_default();

        match self {
            TimeFrameListUpdatePolicy::Append => {
                existing.push_frame(new);
                existing
            }
            TimeFrameListUpdatePolicy::OverwriteLast => {
                existing.update_end_value(new);
                existing
            }
            TimeFrameListUpdatePolicy::Ignore => existing,
            TimeFrameListUpdatePolicy::UpdateLatestEndTime => {
                unimplemented!()
            }
        }
    }
}
