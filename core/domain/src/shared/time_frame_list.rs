use crate::shared::my_time::MyTime;
use anyhow::anyhow;
use chrono::{Duration, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::slice::Iter;
// TODO: Replace Vec with BTreeSet?

#[derive(Debug, thiserror::Error)]
pub enum TimeFrameError {
    #[error("Start time cannot be later than end time")]
    InvalidTimeOrder,
    #[error("Cannot interpret string as a time frame")]
    InvalidFormat,
    #[error("Time frame overlaps with existing frame")]
    OverlappingFrame,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct TimeFrame {
    #[serde(serialize_with = "serialize_time_without_milliseconds")]
    start: NaiveTime,
    #[serde(serialize_with = "serialize_time_without_milliseconds")]
    end: NaiveTime,
}

fn serialize_time_without_milliseconds<S>(
    time: &NaiveTime,
    serializer: S,
) -> anyhow::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&time.format("%H:%M:%S").to_string())
}

impl TimeFrame {
    pub fn duration(&self) -> Duration {
        let duration = self.end.signed_duration_since(self.start);
        debug_assert!(duration >= Duration::zero());
        duration
    }

    pub fn new(start: NaiveTime, end: NaiveTime) -> anyhow::Result<Self, TimeFrameError> {
        if start > end {
            return Err(TimeFrameError::InvalidTimeOrder);
        }
        Ok(Self { start, end })
    }

    pub fn get_start(&self) -> NaiveTime {
        self.start
    }

    pub fn get_end(&self) -> NaiveTime {
        self.end
    }

    pub fn overlaps_with(&self, other: &TimeFrame) -> bool {
        !(self.end <= other.start || self.start >= other.end)
    }
}

impl std::str::FromStr for TimeFrame {
    type Err = TimeFrameError;

    fn from_str(s: &str) -> anyhow::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(TimeFrameError::InvalidFormat);
        }

        let start_str = parts[0].trim();
        let end_str = parts[1].trim();

        if start_str.is_empty() || end_str.is_empty() {
            return Err(TimeFrameError::InvalidFormat);
        }

        let start: MyTime = start_str
            .parse()
            .map_err(|_| TimeFrameError::InvalidFormat)?;
        let end: MyTime = end_str.parse().map_err(|_| TimeFrameError::InvalidFormat)?;

        TimeFrame::new(start.0, end.0)
    }
}

impl fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format = if self.start.second() == 0 && self.end.second() == 0 {
            "%H:%M"
        } else {
            "%H:%M:%S"
        };

        write!(
            f,
            "{}-{}",
            self.start.format(format),
            self.end.format(format)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TimeFrameList {
    data: Vec<TimeFrame>,
}

impl TimeFrameList {
    /// Adds a new TimeFrame to the collection while maintaining chronological order
    /// and preventing overlaps.
    ///
    /// # Arguments
    /// * `new_frame` - The TimeFrame to be added
    ///
    /// # Returns
    /// * `Ok(())` if the frame was successfully added
    /// * `Err(TimeFrameError::OverlappingFrame)` if the new frame overlaps with existing ones
    ///
    /// # Performance
    /// Optimized for append operations (O(1) for chronologically ordered insertions).
    /// Worst case O(n) for random insertions.
    pub fn append_frame(&mut self, new_frame: TimeFrame) -> Result<(), TimeFrameError> {
        // Fast path: check if we can append to the end (most common case)
        if self.can_append_to_end(&new_frame) {
            self.data.push(new_frame);
            return Ok(());
        }

        let insertion_index = self.find_insertion_index(&new_frame)?;
        self.validate_no_overlap(insertion_index, &new_frame)?;
        self.data.insert(insertion_index, new_frame);
        Ok(())
    }

    fn validate_no_overlap(
        &self,
        insertion_index: usize,
        new_frame: &TimeFrame,
    ) -> Result<(), TimeFrameError> {
        if let Some(next_frame) = self.data.get(insertion_index) {
            if new_frame.end > next_frame.start {
                return Err(TimeFrameError::OverlappingFrame);
            }
        }

        if insertion_index > 0 {
            let prev_frame = &self.data[insertion_index - 1];
            if prev_frame.end > new_frame.start {
                return Err(TimeFrameError::OverlappingFrame);
            }
        }

        Ok(())
    }

    fn find_insertion_index(&self, new_frame: &TimeFrame) -> Result<usize, TimeFrameError> {
        match self
            .data
            .binary_search_by_key(&new_frame.start, |frame| frame.start)
        {
            Ok(_) => Err(TimeFrameError::OverlappingFrame), // Exact start time match -> there is an overlap
            Err(index) => Ok(index),
        }
    }

    fn can_append_to_end(&self, new_frame: &TimeFrame) -> bool {
        self.data
            .last()
            .map_or(true, |last_frame| last_frame.end <= new_frame.start)
    }

    /// Updates the end time of an existing TimeFrame or creates a new one.
    ///
    /// # Arguments
    /// * `frame` - The TimeFrame containing the start time to match and the new end time
    ///
    /// # Returns
    /// * `Ok(())` if the update was successful or a new frame was added
    /// * `Err(TimeFrameError::OverlappingFrame)` if the new end time would create an overlap
    ///
    /// # Performance
    /// O(log n) for finding the frame to update using binary search
    /// O(1) for the actual update operation
    pub fn update_end_value(&mut self, frame: TimeFrame) -> Result<(), TimeFrameError> {
        // Fast path: check if the last frame matches
        if let Some(last_frame) = self.data.last_mut() {
            if last_frame.start == frame.start {
                last_frame.end = frame.end;
                return Ok(());
            }
        } else {
            // Empty collection case
            self.data.push(frame);
            return Ok(());
        }

        match self.find_index_with_matching_start_time(frame.start) {
            Some(index) => {
                // Check for potential overlaps with the next frame
                if let Some(next_frame) = self.data.get(index + 1) {
                    if frame.end > next_frame.start {
                        return Err(TimeFrameError::OverlappingFrame);
                    }
                }

                // Update the end time
                self.data[index].end = frame.end;
                Ok(())
            }
            None => self.append_frame(frame),
        }
    }

    /// Finds the index of a TimeFrame with a matching start time.
    /// Ignores milliseconds in the comparison.
    ///
    /// # Arguments
    /// * `start_time` - The start time to search for
    ///
    /// # Returns
    /// * `Some(index)` if a matching frame was found
    /// * `None` if no matching frame exists
    ///
    /// # Performance
    /// O(log n) using binary search
    fn find_index_with_matching_start_time(&self, start_time: NaiveTime) -> Option<usize> {
        let target_seconds = start_time.num_seconds_from_midnight();

        match self.data.binary_search_by(|frame| {
            let frame_seconds = frame.start.num_seconds_from_midnight();
            frame_seconds.cmp(&target_seconds)
        }) {
            Ok(index) => Some(index),
            Err(_) => None,
        }
    }

    pub fn get_total_duration(&self) -> Duration {
        let total_duration = self.data.iter().map(|frame| frame.duration()).sum();
        debug_assert!(total_duration >= Duration::zero());
        total_duration
    }

    pub fn start_time(&self) -> Option<NaiveTime> {
        self.data.first().map(|first| first.start)
    }

    pub fn end_time(&self) -> Option<NaiveTime> {
        self.data.last().map(|first| first.end)
    }

    pub fn iter(&self) -> Iter<'_, TimeFrame> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn to_vec(self) -> Vec<TimeFrame> {
        self.data
    }
}

impl fmt::Display for TimeFrameList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.data.is_empty() {
            return write!(f, "");
        }

        // Format as comma-separated list of time frames
        let frames: Vec<String> = self.data.iter().map(|frame| frame.to_string()).collect();

        write!(f, "{}", frames.join(", "))
    }
}

impl std::str::FromStr for TimeFrameList {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(TimeFrameList::default());
        }

        // Split the string by commas and trim whitespace
        let frames = s.split(',').map(str::trim);

        let mut list = TimeFrameList::default();

        for frame_str in frames {
            // Parse each time frame string into a TimeFrame
            let time_frame: TimeFrame = frame_str
                .parse()
                .map_err(|e| anyhow!("Failed to parse time frame '{}': {}", frame_str, e))?;

            // Append to our list, maintaining chronological order
            list.append_frame(time_frame)
                .map_err(|e| anyhow!("Failed to append time frame '{}': {}", frame_str, e))?;
        }

        Ok(list)
    }
}

impl<'a> IntoIterator for &'a TimeFrameList {
    type Item = &'a TimeFrame;
    type IntoIter = Iter<'a, TimeFrame>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl IntoIterator for TimeFrameList {
    type Item = TimeFrame;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_frame_list_update_end_value() {
        let mut list = TimeFrameList::default();

        // Test 1: Update end value of an existing frame
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )
        .unwrap();
        list.append_frame(frame1).unwrap();

        let update_frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(update_frame1).unwrap();

        assert_eq!(list.data.len(), 1);
        assert_eq!(list.data[0].end, NaiveTime::from_hms_opt(13, 0, 0).unwrap());

        // Test 2: Add a new frame when no matching start time exists
        let new_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(new_frame).unwrap();

        assert_eq!(list.data.len(), 2);
        assert_eq!(
            list.data[1].start,
            NaiveTime::from_hms_opt(14, 0, 0).unwrap()
        );
        assert_eq!(list.data[1].end, NaiveTime::from_hms_opt(17, 0, 0).unwrap());

        // Test 3: Update the last frame in the list
        let update_last_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(update_last_frame).unwrap();

        assert_eq!(list.data.len(), 2);
        assert_eq!(list.data[1].end, NaiveTime::from_hms_opt(18, 0, 0).unwrap());

        // Test 4: Attempt to update a non-existent frame (should add a new frame)
        let non_existent_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(non_existent_frame).unwrap();

        assert_eq!(list.data.len(), 3);
        assert_eq!(
            list.data[2].start,
            NaiveTime::from_hms_opt(20, 0, 0).unwrap()
        );
        assert_eq!(list.data[2].end, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
    }

    #[test]
    fn time_frame_with_seconds_properly_parsed() {
        let time_frame: TimeFrame = "10:01:48-11:15:17".parse().unwrap();

        let start_time = NaiveTime::from_hms_opt(10, 1, 48).unwrap();
        let end_time = NaiveTime::from_hms_opt(11, 15, 17).unwrap();
        assert_eq!(time_frame.get_start(), start_time);
        assert_eq!(time_frame.get_end(), end_time);
    }

    #[test]
    fn time_frame_without_seconds_properly_parsed() {
        let start_time = NaiveTime::from_hms_opt(10, 1, 0).unwrap();
        let end_time = NaiveTime::from_hms_opt(11, 15, 0).unwrap();

        let time_frame: TimeFrame = "10:01-11:15".parse().unwrap();

        assert_eq!(time_frame.get_start(), start_time);
        assert_eq!(time_frame.get_end(), end_time);
    }

    #[test]
    fn test_overlapping_frames_rejected() {
        let mut list = TimeFrameList::default();

        // Add initial frame
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )
        .unwrap();
        list.append_frame(frame1).unwrap();

        // Test overlapping frame (complete overlap)
        let frame2 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            list.append_frame(frame2),
            Err(TimeFrameError::OverlappingFrame)
        ));

        // Test overlapping frame (partial overlap at start)
        let frame3 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            list.append_frame(frame3),
            Err(TimeFrameError::OverlappingFrame)
        ));

        // Test overlapping frame (partial overlap at end)
        let frame4 = TimeFrame::new(
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            list.append_frame(frame4),
            Err(TimeFrameError::OverlappingFrame)
        ));

        // Test non-overlapping frame (should succeed)
        let frame5 = TimeFrame::new(
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(list.append_frame(frame5).is_ok());
    }

    #[test]
    fn test_time_frame_list_maintains_sorted_order() {
        let mut list = TimeFrameList::default();

        // Create frames in non-chronological order
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        )
        .unwrap();

        let frame2 = TimeFrame::new(
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        )
        .unwrap();

        let frame3 = TimeFrame::new(
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();

        // Add frames in random order
        list.append_frame(frame1).unwrap();
        list.append_frame(frame2).unwrap();
        list.append_frame(frame3).unwrap();

        // Verify the list is sorted by start time
        let mut prev_end = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        for frame in list.iter() {
            assert!(
                frame.start >= prev_end,
                "Frames should be sorted by start time"
            );
            prev_end = frame.end;
        }

        // Verify specific order
        let frames: Vec<_> = list.iter().collect();
        assert_eq!(frames[0].start, NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(frames[1].start, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert_eq!(frames[2].start, NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    }

    #[test]
    fn test_time_frame_list_maintains_sorted_order_after_updates() {
        let mut list = TimeFrameList::default();

        // Add initial frames
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        )
        .unwrap();

        let frame2 = TimeFrame::new(
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();

        list.append_frame(frame1).unwrap();
        list.append_frame(frame2).unwrap();

        // Try to update with a frame that would break the order
        let update_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 30, 0).unwrap(), // Would overlap with frame2
        )
        .unwrap();

        assert!(matches!(
            list.update_end_value(update_frame),
            Err(TimeFrameError::OverlappingFrame)
        ));

        // Verify order is maintained after failed update
        let frames: Vec<_> = list.iter().collect();
        assert_eq!(frames[0].start, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert_eq!(frames[1].start, NaiveTime::from_hms_opt(12, 0, 0).unwrap());

        // Update with valid frame
        let valid_update = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
        )
        .unwrap();

        assert!(list.update_end_value(valid_update).is_ok());

        // Verify order is maintained after successful update
        let frames: Vec<_> = list.iter().collect();
        assert_eq!(frames[0].start, NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert_eq!(frames[0].end, NaiveTime::from_hms_opt(11, 30, 0).unwrap());
        assert_eq!(frames[1].start, NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    }

    #[test]
    fn test_time_frame_list_edge_case_insertions() {
        let mut list = TimeFrameList::default();

        // Test inserting at exactly midnight
        let midnight_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
        )
        .unwrap();
        list.append_frame(midnight_frame).unwrap();

        // Test inserting at 23:59:59
        let late_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        )
        .unwrap();
        list.append_frame(late_frame).unwrap();

        // Verify order
        let frames: Vec<_> = list.iter().collect();
        assert_eq!(frames[0].start, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(
            frames[1].start,
            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
        );

        // Test inserting between existing frames
        let middle_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();
        list.append_frame(middle_frame).unwrap();

        // Verify final order
        let frames: Vec<_> = list.iter().collect();
        assert_eq!(frames[0].start, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(frames[1].start, NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert_eq!(
            frames[2].start,
            NaiveTime::from_hms_opt(23, 59, 59).unwrap()
        );
    }

    #[test]
    fn test_parse_time_frame_list() {
        let input = "10:30-12:00, 12:30-14:00, 14:30-16:00, 16:30-18:00, 18:20-19:00";
        let list: TimeFrameList = input.parse().unwrap();

        let frames: Vec<TimeFrame> = list.into_iter().collect();
        assert_eq!(frames.len(), 5);

        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(10, 30, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap()
        );

        assert_eq!(
            frames[1].get_start(),
            NaiveTime::from_hms_opt(12, 30, 0).unwrap()
        );
        assert_eq!(
            frames[1].get_end(),
            NaiveTime::from_hms_opt(14, 0, 0).unwrap()
        );

        assert_eq!(
            frames[2].get_start(),
            NaiveTime::from_hms_opt(14, 30, 0).unwrap()
        );
        assert_eq!(
            frames[2].get_end(),
            NaiveTime::from_hms_opt(16, 0, 0).unwrap()
        );

        assert_eq!(
            frames[3].get_start(),
            NaiveTime::from_hms_opt(16, 30, 0).unwrap()
        );
        assert_eq!(
            frames[3].get_end(),
            NaiveTime::from_hms_opt(18, 0, 0).unwrap()
        );

        assert_eq!(
            frames[4].get_start(),
            NaiveTime::from_hms_opt(18, 20, 0).unwrap()
        );
        assert_eq!(
            frames[4].get_end(),
            NaiveTime::from_hms_opt(19, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_parse_empty_string() {
        let input = "";
        let list: TimeFrameList = input.parse().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_parse_time_frame_list_with_single_frame() {
        let input = "09:00-10:30";
        let list: TimeFrameList = input.parse().unwrap();

        let frames: Vec<TimeFrame> = list.into_iter().collect();
        assert_eq!(frames.len(), 1);

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
    fn test_parse_invalid_format() {
        let input = "10:30-12:00, invalid";
        let result: Result<TimeFrameList, _> = input.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_overlapping_frames() {
        let input = "10:00-12:00, 11:00-13:00";
        let result: Result<TimeFrameList, _> = input.parse();
        assert!(result.is_err());
    }
}
