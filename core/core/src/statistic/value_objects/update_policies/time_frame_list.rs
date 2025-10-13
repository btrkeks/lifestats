use crate::shared::time_frame_list::TimeFrameList;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
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
            (Self::UpdateEndValue, Some(mut current)) => {
                let original = current.clone();

                for frame in new.into_iter() {
                    current.update_end_value(frame)
                        .expect("UpdateEndValue policy resulted in an overlapping time frame. This is a critical logic error.");
                }

                let changed = current != original;
                (current, changed)
            }
            (Self::UpdateEndValue, _) => (new, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::time_frame_list::TimeFrame;
    use chrono::NaiveTime;

    fn create_time_frame(start_h: u32, start_m: u32, end_h: u32, end_m: u32) -> TimeFrame {
        TimeFrame::new(
            NaiveTime::from_hms_opt(start_h, start_m, 0).unwrap(),
            NaiveTime::from_hms_opt(end_h, end_m, 0).unwrap(),
        )
        .unwrap()
    }

    fn create_time_frame_list(frames: Vec<TimeFrame>) -> TimeFrameList {
        let mut list = TimeFrameList::default();
        for frame in frames {
            list.append_frame(frame).unwrap();
        }
        list
    }

    #[test]
    fn test_update_existing_frame_happy_path() {
        let existing = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);
        let new = create_time_frame_list(vec![create_time_frame(9, 0, 10, 30)]);

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        let (result, changed) = policy.apply(Some(existing), new);

        assert!(changed);
        assert_eq!(result.len(), 1);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(10, 30, 0).unwrap()
        );
    }

    #[test]
    fn test_upsert_behavior_add_new_frame() {
        let existing = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);
        let new = create_time_frame_list(vec![create_time_frame(11, 0, 12, 0)]);

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        let (result, changed) = policy.apply(Some(existing), new);

        assert!(changed);
        assert_eq!(result.len(), 2);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap()
        );
        assert_eq!(
            frames[1].get_start(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap()
        );
        assert_eq!(
            frames[1].get_end(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_idempotency_no_change_for_identical_value() {
        let existing = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);
        let new = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        let (result, changed) = policy.apply(Some(existing), new);

        assert!(!changed);
        assert_eq!(result.len(), 1);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "UpdateEndValue policy resulted in an overlapping time frame")]
    fn test_invariant_enforcement_panic_on_overlap() {
        let existing = create_time_frame_list(vec![
            create_time_frame(9, 0, 10, 0),
            create_time_frame(11, 0, 12, 0),
        ]);
        let new = create_time_frame_list(vec![create_time_frame(9, 0, 11, 30)]);

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        policy.apply(Some(existing), new);
    }

    #[test]
    fn test_multi_frame_update() {
        let existing = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);
        let new = create_time_frame_list(vec![
            create_time_frame(9, 0, 10, 30),
            create_time_frame(14, 0, 15, 0),
        ]);

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        let (result, changed) = policy.apply(Some(existing), new);

        assert!(changed);
        assert_eq!(result.len(), 2);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(10, 30, 0).unwrap()
        );
        assert_eq!(
            frames[1].get_start(),
            NaiveTime::from_hms_opt(14, 0, 0).unwrap()
        );
        assert_eq!(
            frames[1].get_end(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_empty_input() {
        let existing = create_time_frame_list(vec![create_time_frame(9, 0, 10, 0)]);
        let new = TimeFrameList::default();

        let policy = TimeFrameListUpdatePolicy::UpdateEndValue;
        let (result, changed) = policy.apply(Some(existing), new);

        assert!(!changed);
        assert_eq!(result.len(), 1);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap()
        );
    }
}
