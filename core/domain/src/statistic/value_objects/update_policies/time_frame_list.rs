use crate::shared::time_frame_list::TimeFrameList;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum TimeFrameListUpdatePolicy {
    #[default]
    Append,
    UpdateEndValue,
}

impl UpdatePolicy<TimeFrameList> for TimeFrameListUpdatePolicy {
    fn apply(&self, existing: Option<TimeFrameList>, new: TimeFrameList) -> (TimeFrameList, bool) {
        match (self, existing) {
            (Self::Append, Some(_current)) => todo!(),
            (Self::Append, _) => (new, true),
            (Self::UpdateEndValue, Some(_current)) => todo!(),
            (Self::UpdateEndValue, _) => (new, true),
        }
    }
}
