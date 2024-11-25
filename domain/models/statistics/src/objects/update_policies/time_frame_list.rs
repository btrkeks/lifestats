use strum_macros::{Display, EnumString};
use crate::objects::time_frame_list::{TimeFrame, TimeFrameList};
use crate::objects::update_policies::UpdatePolicy;

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum TimeFrameListUpdatePolicy {
    #[default]
    Append,
    UpdateEndValue,
}

impl UpdatePolicy<TimeFrameList, TimeFrame> for TimeFrameListUpdatePolicy {
    fn apply(&self, existing: Option<TimeFrameList>, new: TimeFrame) -> TimeFrameList {
        let mut existing = existing.unwrap_or_default();

        match self {
            TimeFrameListUpdatePolicy::Append => {
                existing.append_frame(new).unwrap(); // TODO
                existing
            }
            TimeFrameListUpdatePolicy::UpdateEndValue => {
                existing.update_end_value(new).unwrap(); // TODO
                existing
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use chrono::NaiveTime;

    // Helper function to create a TimeFrame for testing
    fn create_time_frame(start_str: &str, end_str: &str) -> TimeFrame {
        let start = NaiveTime::parse_from_str(start_str, "%H:%M:%S")
            .expect("Failed to parse start time");
        let end = NaiveTime::parse_from_str(end_str, "%H:%M:%S")
            .expect("Failed to parse end time");
        TimeFrame::new(start, end).expect("Failed to create TimeFrame")
    }

    #[test]
    fn test_enum_string_conversion() {
        // Test string to enum conversion
        assert_eq!(
            TimeFrameListUpdatePolicy::from_str("append").unwrap(),
            TimeFrameListUpdatePolicy::Append
        );
        assert_eq!(
            TimeFrameListUpdatePolicy::from_str("updateendvalue").unwrap(),
            TimeFrameListUpdatePolicy::UpdateEndValue
        );

        // Test invalid string conversion
        assert!(TimeFrameListUpdatePolicy::from_str("invalid").is_err());
    }

    #[test]
    fn test_default() {
        assert_eq!(TimeFrameListUpdatePolicy::default(), TimeFrameListUpdatePolicy::Append);
    }

    #[test]
    fn test_append_policy() {
        let policy = TimeFrameListUpdatePolicy::Append;

        // Test appending to empty list
        let new_frame = create_time_frame("09:00:00", "10:00:00");
        let result = policy.apply(None, new_frame);
        assert_eq!(result.len(), 1);
        assert_eq!(result.to_vec().get(0).unwrap(), &new_frame);

        // Test appending to existing list
        let mut initial_list = TimeFrameList::default();
        initial_list.append_frame(create_time_frame("09:00:00", "10:00:00")).unwrap();
        let new_frame = create_time_frame("10:00:00", "11:00:00");
        let result = policy.apply(Some(initial_list), new_frame);
        assert_eq!(result.len(), 2);
        assert_eq!(result.to_vec().get(1).unwrap(), &new_frame);
    }

    #[test]
    fn test_update_end_value_policy() {
        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;

        // Test overwriting in empty list
        let new_frame = create_time_frame("09:00:00", "10:00:00");
        let result = policy.apply(None, new_frame);
        assert_eq!(result.len(), 1);
        assert_eq!(result.to_vec().get(0).unwrap(), &new_frame);

        // Test overwriting last frame in existing list
        let mut initial_list = TimeFrameList::default();
        initial_list.append_frame(create_time_frame("09:00:00", "10:00:00")).unwrap();
        initial_list.append_frame(create_time_frame("10:00:00", "11:00:00")).unwrap();
        let new_frame = create_time_frame("10:00:00", "12:00:00");
        let result = policy.apply(Some(initial_list), new_frame).to_vec();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(0).unwrap(), &create_time_frame("09:00:00", "10:00:00"));
        assert_eq!(result.get(1).unwrap(), &create_time_frame("10:00:00", "12:00:00"));
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(TimeFrameListUpdatePolicy::Append.to_string(), "append");
        assert_eq!(TimeFrameListUpdatePolicy::UpdateEndValue.to_string(), "updateendvalue");
    }

    #[test]
    fn test_debug_trait() {
        assert_eq!(
            format!("{:?}", TimeFrameListUpdatePolicy::Append),
            "Append"
        );
        assert_eq!(
            format!("{:?}", TimeFrameListUpdatePolicy::UpdateEndValue),
            "UpdateEndValue"
        );
    }

    #[test]
    fn test_clone_trait() {
        let original = TimeFrameListUpdatePolicy::Append;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_copy_trait() {
        let original = TimeFrameListUpdatePolicy::Append;
        let copied = original;
        assert_eq!(original, copied);
        // Verify original is still valid (Copy trait)
        assert_eq!(original, TimeFrameListUpdatePolicy::Append);
    }
}
