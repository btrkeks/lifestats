use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Days, NaiveDate};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
use lifestats_core::statistic::any_statistic::{AnyGoal, AnyStatistic};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use lifestats_core::statistic::value_objects::goals::goal_trait::Goal;
use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_core::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;

use crate::error::ApplicationResult;
use crate::services::goal_resolver::GoalResolver;

/// Represents the evaluation result for a single day
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayEval {
    pub date: NaiveDate,
    /// Whether this day had any applicable goal (default or daily)
    pub has_goal: bool,
    /// Whether the goal was achieved (typically percentage >= 100)
    pub achieved: bool,
    /// Computed goal achievement percentage for this day.
    /// - `Some(p)` when there is a goal (and we can evaluate it)
    /// - `None` when there is no goal or the statistic type does not support goals
    pub percentage_achieved: Option<Percentage>,
}

/// Trait for evaluating daily achievement across a date range.
/// Provides bulk evaluation with efficient data fetching (no N+1 queries).
#[cfg_attr(test, mockall::automock)]
pub trait DailyAchievementEvaluator: Send + Sync {
    /// Evaluates achievement for each day in the given range.
    ///
    /// - Bulk fetches all necessary data (metadata, logs, goals) in O(1) DB calls
    /// - For TimeSpans statistics, merges same-day frames before evaluation
    /// - Resolves goals per day: daily override > default goal (if date >= valid_since) > no goal
    /// - Returns one DayEval per day in the range
    fn eval_range(&self, stat: &AnyStatistic, range: DateRange) -> ApplicationResult<Vec<DayEval>>;
}

/// Default implementation of DailyAchievementEvaluator
pub struct DefaultDailyAchievementEvaluator<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repo: Arc<R>,
}

impl<R> DefaultDailyAchievementEvaluator<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Helper to evaluate a range for a specific statistic type with generic log/goal types
    fn eval_range_typed<LogValue, GoalType>(
        &self,
        stat: &AnyStatistic,
        range: DateRange,
        default_goal: Option<GoalType>,
        default_goal_valid_since: NaiveDate,
    ) -> ApplicationResult<Vec<DayEval>>
    where
        LogValue: serde::de::DeserializeOwned + 'static,
        GoalType: serde::de::DeserializeOwned + Clone + Goal<LogValue = LogValue> + 'static,
    {
        let stat_id = stat.get_metadata().id();

        // Bulk fetch log values
        let log_values = self.repo.find_log_values_in_range(stat_id, range)?;

        // Resolve goals using GoalResolver
        let goal_resolver = GoalResolver::new(self.repo.clone());
        let resolved_goals = goal_resolver.resolve_goals_for_range(stat, range)?;

        // Build HashMap for log values O(1) lookup
        let mut log_map: HashMap<NaiveDate, LogValue> = HashMap::with_capacity(log_values.len());
        for dated_value in log_values {
            let value: LogValue = serde_json::from_value(dated_value.value)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize log value: {}", e))?;
            log_map.insert(dated_value.date, value);
        }

        // Iterate through each day in the range
        let mut result = Vec::new();
        let start = range.start();
        let end = range.end();

        let num_days = (end.signed_duration_since(start).num_days() + 1) as u64;
        for i in 0..num_days {
            let date = start.checked_add_days(Days::new(i)).unwrap_or(start);

            // Get resolved goal for this day (already handles default/override/suppression)
            let goal_json = resolved_goals.get(&date).and_then(|opt| opt.as_ref());

            let goal: Option<GoalType> = match goal_json {
                Some(json) => Some(
                    serde_json::from_value(json.clone())
                        .map_err(|e| anyhow::anyhow!("Failed to deserialize goal: {}", e))?,
                ),
                None => None,
            };

            if goal.is_none() {
                // No goal for this day
                result.push(DayEval {
                    date,
                    has_goal: false,
                    achieved: false,
                    percentage_achieved: None,
                });
                continue;
            }

            // Evaluate using domain logic
            let log_value = log_map.get(&date);
            let goal_ref = goal.as_ref().unwrap();

            // Compute percentage using Goal trait method
            let percentage = goal_ref.get_percentage_achieved(log_value);
            let achieved = percentage.as_num() >= 100;

            result.push(DayEval {
                date,
                has_goal: true,
                achieved,
                percentage_achieved: Some(percentage),
            });
        }

        Ok(result)
    }

    /// Special handling for TimeSpans: merge same-day frames before evaluation
    fn eval_range_time_spans(
        &self,
        stat: &AnyStatistic,
        range: DateRange,
        _default_goal: Option<TimeSpansGoal>,
        _default_goal_valid_since: NaiveDate,
    ) -> ApplicationResult<Vec<DayEval>> {
        let stat_id = stat.get_metadata().id();

        // Bulk fetch log values
        let log_values = self.repo.find_log_values_in_range(stat_id, range)?;

        // Resolve goals using GoalResolver
        let goal_resolver = GoalResolver::new(self.repo.clone());
        let resolved_goals = goal_resolver.resolve_goals_for_range(stat, range)?;

        // Build log map with TimeFrameList merging
        // Group by date first, then merge frames within each day
        let mut log_map: HashMap<NaiveDate, Vec<TimeFrameList>> = HashMap::new();
        for dated_value in log_values {
            let frame_list: TimeFrameList = serde_json::from_value(dated_value.value)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize TimeFrameList: {}", e))?;
            log_map
                .entry(dated_value.date)
                .or_default()
                .push(frame_list);
        }

        // Merge frames per day into a single canonical TimeFrameList
        let merged_log_map: HashMap<NaiveDate, TimeFrameList> = log_map
            .into_iter()
            .map(|(date, frame_lists)| {
                let merged = Self::merge_time_frame_lists(frame_lists);
                (date, merged)
            })
            .collect();

        // Iterate through each day in the range
        let mut result = Vec::new();
        let start = range.start();
        let end = range.end();

        let num_days = (end.signed_duration_since(start).num_days() + 1) as u64;
        for i in 0..num_days {
            let date = start.checked_add_days(Days::new(i)).unwrap_or(start);

            // Get resolved goal for this day (already handles default/override/suppression)
            let goal_json = resolved_goals.get(&date).and_then(|opt| opt.as_ref());

            let goal: Option<TimeSpansGoal> =
                match goal_json {
                    Some(json) => Some(serde_json::from_value(json.clone()).map_err(|e| {
                        anyhow::anyhow!("Failed to deserialize TimeSpans goal: {}", e)
                    })?),
                    None => None,
                };

            if goal.is_none() {
                // No goal for this day
                result.push(DayEval {
                    date,
                    has_goal: false,
                    achieved: false,
                    percentage_achieved: None,
                });
                continue;
            }

            // Evaluate using domain logic
            let log_value = merged_log_map.get(&date);
            let goal_ref = goal.as_ref().unwrap();

            // Compute percentage using Goal trait method
            let percentage = goal_ref.get_percentage_achieved(log_value);
            let achieved = percentage.as_num() >= 100;

            result.push(DayEval {
                date,
                has_goal: true,
                achieved,
                percentage_achieved: Some(percentage),
            });
        }

        Ok(result)
    }

    /// Merges multiple TimeFrameLists into a single list with coalesced frames.
    ///
    /// Algorithm:
    /// 1. Collect all frames from all lists
    /// 2. Sort by start time (stable sort)
    /// 3. Merge overlapping/adjacent frames
    fn merge_time_frame_lists(lists: Vec<TimeFrameList>) -> TimeFrameList {
        if lists.is_empty() {
            return TimeFrameList::default();
        }

        // Collect all frames from all lists
        let mut all_frames: Vec<TimeFrame> = Vec::new();
        for list in lists {
            all_frames.extend(list.iter().copied());
        }

        if all_frames.is_empty() {
            return TimeFrameList::default();
        }

        // Sort by start time
        all_frames.sort_by_key(|frame| frame.get_start());

        // Merge overlapping/adjacent frames
        let mut merged = TimeFrameList::default();
        let mut current = all_frames[0];

        for &next_frame in &all_frames[1..] {
            // Check if frames overlap or are adjacent
            if next_frame.get_start() <= current.get_end() {
                // Merge: extend current to cover both frames
                let new_end = std::cmp::max(current.get_end(), next_frame.get_end());
                // Create merged frame
                if let Ok(merged_frame) = TimeFrame::new(current.get_start(), new_end) {
                    current = merged_frame;
                }
            } else {
                // No overlap - add current and start new
                let _ = merged.append_frame(current);
                current = next_frame;
            }
        }

        // Don't forget the last frame
        let _ = merged.append_frame(current);
        merged
    }
}

impl<R> DailyAchievementEvaluator for DefaultDailyAchievementEvaluator<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn eval_range(&self, stat: &AnyStatistic, range: DateRange) -> ApplicationResult<Vec<DayEval>> {
        let default_goal = stat.get_default_goal();
        let default_goal_valid_since = stat.get_default_goal_valid_since();

        match stat {
            AnyStatistic::Habit(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::Habit(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_typed::<bool, HabitGoal>(
                    stat,
                    range,
                    goal,
                    default_goal_valid_since,
                )
            }
            AnyStatistic::IntegerAmount(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::IntegerAmount(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_typed::<i64, AmountGoal<i64>>(
                    stat,
                    range,
                    goal,
                    default_goal_valid_since,
                )
            }
            AnyStatistic::PointInTime(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::FinishedInTime(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_typed::<chrono::NaiveTime, FinishedInTimeGoal>(
                    stat,
                    range,
                    goal,
                    default_goal_valid_since,
                )
            }
            AnyStatistic::TimeSpans(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::TimeSpans(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_time_spans(stat, range, goal, default_goal_valid_since)
            }
            AnyStatistic::Summing(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::SecondsAmount(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_typed::<Seconds, AmountGoal<Seconds>>(
                    stat,
                    range,
                    goal,
                    default_goal_valid_since,
                )
            }
            AnyStatistic::TimeDifference(_) => {
                let goal = default_goal.and_then(|g| match g {
                    AnyGoal::SecondsAmount(goal) => Some(goal),
                    _ => None,
                });
                self.eval_range_typed::<Seconds, AmountGoal<Seconds>>(
                    stat,
                    range,
                    goal,
                    default_goal_valid_since,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

    #[test]
    fn test_merge_time_frame_lists_empty() {
        let result =
            DefaultDailyAchievementEvaluator::<MockStatisticRepository>::merge_time_frame_lists(
                vec![],
            );
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_merge_time_frame_lists_single() {
        let mut list = TimeFrameList::default();
        let frame = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        )
        .unwrap();
        list.append_frame(frame).unwrap();

        let result =
            DefaultDailyAchievementEvaluator::<MockStatisticRepository>::merge_time_frame_lists(
                vec![list.clone()],
            );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_merge_time_frame_lists_overlapping() {
        let mut list1 = TimeFrameList::default();
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        )
        .unwrap();
        list1.append_frame(frame1).unwrap();

        let mut list2 = TimeFrameList::default();
        let frame2 = TimeFrame::new(
            NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )
        .unwrap();
        list2.append_frame(frame2).unwrap();

        let result =
            DefaultDailyAchievementEvaluator::<MockStatisticRepository>::merge_time_frame_lists(
                vec![list1, list2],
            );

        // Should merge into single frame from 9:00 to 12:00
        assert_eq!(result.len(), 1);
        let frames: Vec<_> = result.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_merge_time_frame_lists_non_overlapping() {
        let mut list1 = TimeFrameList::default();
        let frame1 = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        )
        .unwrap();
        list1.append_frame(frame1).unwrap();

        let mut list2 = TimeFrameList::default();
        let frame2 = TimeFrame::new(
            NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
        )
        .unwrap();
        list2.append_frame(frame2).unwrap();

        let result =
            DefaultDailyAchievementEvaluator::<MockStatisticRepository>::merge_time_frame_lists(
                vec![list1, list2],
            );

        // Should keep both frames separate
        assert_eq!(result.len(), 2);
    }
}
