use anyhow::Result;
use chrono::{Duration, NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::slice::Iter;

#[derive(Default, Clone)]
pub struct StatisticRow<T> {
    pub date: NaiveDate,
    pub value: Option<T>,
    pub goal: Option<T>,
}

impl<T> StatisticRow<T> {
    pub fn new_with_value(date: NaiveDate, value: T, goal: Option<T>) -> Self {
        StatisticRow {
            date,
            value: Some(value),
            goal,
        }
    }

    pub fn new(date: NaiveDate, value: Option<T>, goal: Option<T>) -> Self {
        StatisticRow { date, value, goal }
    }

    pub fn new_without_value(date: NaiveDate, goal: Option<T>) -> Self {
        StatisticRow {
            date,
            value: None,
            goal,
        }
    }
}

fn serialize_time_without_milliseconds<S>(
    time: &NaiveTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&time.format("%H:%M:%S").to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum TimeFrameError {
    #[error("Start time cannot be later than end time")]
    InvalidTimeOrder,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct TimeFrame {
    #[serde(serialize_with = "serialize_time_without_milliseconds")]
    pub start: NaiveTime,
    #[serde(serialize_with = "serialize_time_without_milliseconds")]
    pub end: NaiveTime,
}

impl TimeFrame {
    pub fn duration(&self) -> Duration {
        self.end.signed_duration_since(self.start)
    }

    pub fn new(start: NaiveTime, end: NaiveTime) -> Result<Self, TimeFrameError> {
        if start > end {
            return Err(TimeFrameError::InvalidTimeOrder);
        }
        Ok(Self { start, end })
    }
}

impl fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}",
            self.start.format("%H:%M:%S"),
            self.end.format("%H:%M:%S")
        )
    }
}

impl TryFrom<(NaiveTime, NaiveTime)> for TimeFrame {
    type Error = TimeFrameError;

    fn try_from(value: (NaiveTime, NaiveTime)) -> Result<Self, Self::Error> {
        Self::new(value.0, value.1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeFrameList {
    // data points should always be sorted by start time
    pub(crate) data: Vec<TimeFrame>,
}

impl TimeFrameList {
    pub fn new_from_vec(vec: Vec<TimeFrame>) -> TimeFrameList {
        TimeFrameList { data: vec }
    }

    pub fn push_frame(&mut self, frame: TimeFrame) {
        self.data.push(frame);
    }

    pub fn update_end_value(&mut self, frame: TimeFrame) {
        let matching_index = self.rfind_matching_start_time_index(frame.start);
        if let Some(matching_index) = matching_index {
            self.data[matching_index].end = frame.end;
        } else {
            self.push_frame(frame);
        }
    }

    fn rfind_matching_start_time_index(&self, start_time: NaiveTime) -> Option<usize> {
        self.data
            .iter()
            // .rev()
            // .take_while(|frame| frame.start >= start_time)
            .position(|frame|
                // Don't count millisecs
                frame.start.num_seconds_from_midnight() == start_time.num_seconds_from_midnight())
    }

    pub fn get_total_duration(&self) -> Duration {
        self.data.iter().map(|frame| frame.duration()).sum()
    }

    pub fn into_vec(self) -> Vec<TimeFrame> {
        self.data
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

#[derive(Debug)]
pub struct IntValueRow {
    pub date: NaiveDate,
    pub value: i64,
    pub suspend: bool,
}

impl IntValueRow {
    pub fn default(date: NaiveDate) -> IntValueRow {
        IntValueRow {
            date,
            value: 0,
            suspend: false,
        }
    }

    pub fn new(date: NaiveDate, value: i64) -> IntValueRow {
        IntValueRow {
            date,
            value,
            suspend: false,
        }
    }

    pub fn get_value(&self) -> i64 {
        self.value
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
        list.push_frame(frame1);

        let update_frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(update_frame1);

        assert_eq!(list.data.len(), 1);
        assert_eq!(list.data[0].end, NaiveTime::from_hms_opt(13, 0, 0).unwrap());

        // Test 2: Add a new frame when no matching start time exists
        let new_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(new_frame);

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
        list.update_end_value(update_last_frame);

        assert_eq!(list.data.len(), 2);
        assert_eq!(list.data[1].end, NaiveTime::from_hms_opt(18, 0, 0).unwrap());

        // Test 4: Attempt to update a non-existent frame (should add a new frame)
        let non_existent_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
        )
        .unwrap();
        list.update_end_value(non_existent_frame);

        assert_eq!(list.data.len(), 3);
        assert_eq!(
            list.data[2].start,
            NaiveTime::from_hms_opt(20, 0, 0).unwrap()
        );
        assert_eq!(list.data[2].end, NaiveTime::from_hms_opt(22, 0, 0).unwrap());
    }
}
