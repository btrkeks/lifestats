use chrono::NaiveDate;

/// One stretch of a statistic's default goal: `goal` applies from `valid_since`
/// (inclusive) until the next segment starts. `None` means no default goal.
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultGoalSegment<G> {
    pub valid_since: NaiveDate,
    pub goal: Option<G>,
}

impl<G> DefaultGoalSegment<G> {
    pub fn new(valid_since: NaiveDate, goal: Option<G>) -> Self {
        Self { valid_since, goal }
    }
}

/// The default goal of a statistic over time.
///
/// Changing the default goal from a date only affects that date and later days:
/// earlier days keep being judged against the goal that applied to them.
///
/// Invariants (kept by every constructor and mutation):
/// - segments are sorted by `valid_since` and no two share a date
/// - the first segment has a goal (days before it have none anyway)
/// - neighbouring segments have different goals
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultGoalTimeline<G> {
    segments: Vec<DefaultGoalSegment<G>>,
}

impl<G> Default for DefaultGoalTimeline<G> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
        }
    }
}

impl<G: Clone + PartialEq> DefaultGoalTimeline<G> {
    /// Builds a timeline from segments in any order. When two segments share a
    /// date, the later one in `segments` wins.
    pub fn new(segments: Vec<DefaultGoalSegment<G>>) -> Self {
        let mut timeline = Self::default();
        let mut sorted = segments;
        sorted.sort_by_key(|segment| segment.valid_since);
        for segment in sorted {
            if let Some(last) = timeline.segments.last_mut() {
                if last.valid_since == segment.valid_since {
                    *last = segment;
                    continue;
                }
            }
            timeline.segments.push(segment);
        }
        timeline.normalize();
        timeline
    }

    pub fn segments(&self) -> &[DefaultGoalSegment<G>] {
        &self.segments
    }

    /// The segment that applies to the latest days (it may start in the future).
    pub fn latest(&self) -> Option<&DefaultGoalSegment<G>> {
        self.segments.last()
    }

    /// The default goal that applies on `date`, if any.
    pub fn goal_on(&self, date: NaiveDate) -> Option<&G> {
        self.segments
            .iter()
            .rev()
            .find(|segment| segment.valid_since <= date)
            .and_then(|segment| segment.goal.as_ref())
    }

    /// Applies `goal` from `valid_since` onwards (`None` removes the default goal).
    /// Segments that started on or after `valid_since` are replaced; days before
    /// it keep their goal.
    pub fn set_from(&mut self, valid_since: NaiveDate, goal: Option<G>) {
        self.segments
            .retain(|segment| segment.valid_since < valid_since);
        self.segments
            .push(DefaultGoalSegment::new(valid_since, goal));
        self.normalize();
    }

    fn normalize(&mut self) {
        let leading_empty = self
            .segments
            .iter()
            .take_while(|segment| segment.goal.is_none())
            .count();
        self.segments.drain(..leading_empty);
        self.segments
            .dedup_by(|later, earlier| later.goal == earlier.goal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn segment(month: u32, day: u32, goal: Option<u32>) -> DefaultGoalSegment<u32> {
        DefaultGoalSegment::new(date(month, day), goal)
    }

    #[test]
    fn test_empty_timeline_has_no_goal() {
        let timeline = DefaultGoalTimeline::<u32>::default();
        assert_eq!(timeline.goal_on(date(10, 1)), None);
        assert!(timeline.latest().is_none());
    }

    #[test]
    fn test_goal_on_picks_the_segment_covering_the_date() {
        let timeline =
            DefaultGoalTimeline::new(vec![segment(10, 20, Some(450)), segment(9, 9, Some(400))]);

        assert_eq!(timeline.goal_on(date(9, 8)), None);
        assert_eq!(timeline.goal_on(date(9, 9)), Some(&400));
        assert_eq!(timeline.goal_on(date(10, 19)), Some(&400));
        assert_eq!(timeline.goal_on(date(10, 20)), Some(&450));
        assert_eq!(timeline.goal_on(date(12, 31)), Some(&450));
    }

    #[test]
    fn test_set_from_later_date_keeps_earlier_days() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(9, 9, Some(400))]);

        timeline.set_from(date(10, 27), Some(450));

        assert_eq!(timeline.goal_on(date(10, 26)), Some(&400));
        assert_eq!(timeline.goal_on(date(10, 27)), Some(&450));
        assert_eq!(timeline.segments().len(), 2);
    }

    #[test]
    fn test_set_from_earlier_date_replaces_later_segments() {
        let mut timeline =
            DefaultGoalTimeline::new(vec![segment(8, 1, Some(300)), segment(10, 22, Some(400))]);

        timeline.set_from(date(10, 12), Some(500));

        assert_eq!(timeline.goal_on(date(10, 11)), Some(&300));
        assert_eq!(timeline.goal_on(date(10, 12)), Some(&500));
        assert_eq!(timeline.goal_on(date(10, 30)), Some(&500));
        assert_eq!(timeline.latest(), Some(&segment(10, 12, Some(500))));
    }

    #[test]
    fn test_set_from_same_date_replaces_the_goal() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(10, 22, Some(400))]);

        timeline.set_from(date(10, 22), Some(450));

        assert_eq!(timeline.segments(), &[segment(10, 22, Some(450))]);
    }

    #[test]
    fn test_removing_the_goal_keeps_the_past() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(9, 9, Some(400))]);

        timeline.set_from(date(10, 27), None);

        assert_eq!(timeline.goal_on(date(10, 26)), Some(&400));
        assert_eq!(timeline.goal_on(date(10, 27)), None);
        assert_eq!(timeline.latest(), Some(&segment(10, 27, None)));
    }

    #[test]
    fn test_goal_after_a_gap_does_not_reach_back_into_the_gap() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(9, 9, Some(400))]);
        timeline.set_from(date(10, 1), None);

        timeline.set_from(date(10, 10), Some(450));

        assert_eq!(timeline.goal_on(date(9, 30)), Some(&400));
        assert_eq!(timeline.goal_on(date(10, 5)), None);
        assert_eq!(timeline.goal_on(date(10, 10)), Some(&450));
    }

    #[test]
    fn test_leading_empty_segments_are_dropped() {
        let timeline =
            DefaultGoalTimeline::new(vec![segment(1, 1, None), segment(10, 1, Some(400))]);

        assert_eq!(timeline.segments(), &[segment(10, 1, Some(400))]);
    }

    #[test]
    fn test_removing_the_only_goal_leaves_an_empty_timeline() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(10, 1, Some(400))]);

        timeline.set_from(date(10, 1), None);

        assert!(timeline.segments().is_empty());
    }

    #[test]
    fn test_setting_the_same_goal_again_merges_segments() {
        let mut timeline = DefaultGoalTimeline::new(vec![segment(9, 9, Some(400))]);

        timeline.set_from(date(10, 1), Some(400));

        assert_eq!(timeline.segments(), &[segment(9, 9, Some(400))]);
    }

    #[test]
    fn test_new_keeps_the_last_segment_for_a_duplicate_date() {
        let timeline =
            DefaultGoalTimeline::new(vec![segment(10, 1, Some(400)), segment(10, 1, Some(450))]);

        assert_eq!(timeline.segments(), &[segment(10, 1, Some(450))]);
    }
}
