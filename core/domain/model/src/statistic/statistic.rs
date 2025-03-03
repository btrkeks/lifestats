use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::update_policy::UpdatePolicy;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use log::info;
use serde::Serialize;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use std::sync::Arc;
use transfer_objects::raw_goal::RawGoal;
use transfer_objects::statistic_id::StatisticId;
use transfer_objects::statistic_metadata::StatisticMetadata;

pub trait StatisticBase {
    fn get_scores_for_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<f64>>>>;

    fn get_mean_score_for_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Option<f64>> {
        let (sum, count) = self
            .get_scores_for_closed_range(start_date, end_date)?
            .into_iter()
            .flat_map(|date_value| date_value.value)
            .fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));

        if count > 0 {
            Ok(Some(sum / count as f64))
        } else {
            Ok(None)
        }
    }

    fn get_streak(&self) -> Result<Option<u32>>;
}

pub trait Statistic: StatisticBase {
    /// The type of data that can be logged
    /// E.g.: A time frame [10:00 - 11:30]
    type LogValue: Display + FromStr + Debug; // FromStr for log_from_string

    /// The type of data that will be stored in the database
    /// E.g.: A list of time frames [[10:00 - 11:30], [12:00 - 13:30]]
    type RetrieveValue: Clone + Serialize + serde::de::DeserializeOwned + FromStr;

    type UpdatePolicyType: UpdatePolicy<Self::RetrieveValue, Self::LogValue>;

    type GoalType: Goal<Self::RetrieveValue>;

    // Metadata

    fn get_metadata(&self) -> &StatisticMetadata;

    fn id(&self) -> StatisticId {
        self.get_metadata().id()
    }

    fn get_category(&self) -> &str {
        self.get_metadata().category()
    }

    fn get_name(&self) -> &str {
        self.get_metadata().name()
    }

    // Goal related

    fn get_goal_repository(
        &self,
    ) -> &Arc<dyn GoalRepositoryTrait<GoalType = Self::GoalType, StoredValue = Self::RetrieveValue>>;

    // fn _get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
    //     let repository_goal = self.get_goal_repository().get(self.get_id(), date)?;
    //
    //     match repository_goal {
    //         Some(goal) => Ok(Some(goal)),
    //         None => self._get_default_goal(),
    //     }
    // }

    fn _get_default_goal(&self) -> Result<Option<Self::GoalType>> {
        self.get_goal_repository().get_default(self.id())
    }

    fn get_goal_for_date(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        let db_goal = self.get_goal_repository().get(self.id(), date)?;

        if let Some(db_goal) = db_goal {
            Ok(db_goal)
        } else {
            self._get_default_goal()
        }
    }

    fn _get_goals_for_closed_range(
        &self,
        date_start: NaiveDate,
        date_end: NaiveDate,
    ) -> Result<Vec<DateValue<Option<Self::GoalType>>>> {
        // Get the current default goal that will be used to fill gaps
        let default_goal = self._get_default_goal()?;

        // Get all explicitly set goals for the date range
        let mut explicit_goals = self
            .get_goal_repository()
            .get_for_closed_range(self.id(), date_start, date_end)?
            .into_iter();

        let mut next_explicit_goal = explicit_goals.next();
        let mut complete_goals =
            Vec::with_capacity((date_end - date_start).num_days() as usize + 1);
        let mut current_date = date_start;

        while current_date <= date_end {
            let goal = match &next_explicit_goal {
                Some(date_value) if date_value.date == current_date => {
                    // This goal was set explicitly, so should be taken as is. Can be None.
                    let goal = date_value.value.clone();
                    next_explicit_goal = explicit_goals.next();
                    goal
                }
                _ => default_goal.clone(), // Either no goals left or the next explicit goal is in the future
            };

            complete_goals.push(DateValue::new(current_date, goal));

            current_date = current_date
                .succ_opt()
                .ok_or_else(|| anyhow!("Date overflow when advancing to next date"))?;
        }

        Ok(complete_goals)
    }

    fn set_new_default_goal(&mut self, new_goal: Self::GoalType) -> Result<()> {
        self.get_goal_repository().set_default(self.id(), new_goal)
    }

    fn set_new_default_goal_from_string(
        &mut self,
        target: Option<&str>,
        extra_data_1: Option<&str>,
    ) -> Result<()> {
        if let Some(target_str) = target {
            let parsed_target = target_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Parse error"))?;

            let mut new_goal = Self::GoalType::new_with_target(parsed_target);

            if let Some(extra_data_1) = extra_data_1 {
                new_goal = new_goal.set_extra_data_1(extra_data_1)?;
            }

            self.set_new_default_goal(new_goal)
        } else {
            self.get_goal_repository().delete_default(self.id())
        }
    }

    fn get_raw_default_goal(&mut self) -> Result<Option<RawGoal>> {
        let goal = self._get_default_goal()?;
        Ok(goal.map(|g| g.into_raw_goal()))
    }

    fn set_goal_for_date(
        &mut self,
        date: NaiveDate,
        new_goal: Option<Self::GoalType>,
    ) -> Result<()> {
        self.get_goal_repository()
            .set_for_date(self.id(), date, new_goal)
    }

    fn set_goal_for_date_from_string(
        &mut self,
        date: NaiveDate,
        target: Option<&str>,
        extra_data_1: Option<&str>,
    ) -> Result<()> {
        if let Some(target_str) = target {
            let parsed_target = target_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Parse error"))?;
            let mut new_goal = Self::GoalType::new_with_target(parsed_target);

            if let Some(extra_data_1) = extra_data_1 {
                new_goal = new_goal.set_extra_data_1(extra_data_1)?;
            }

            self.set_goal_for_date(date, Some(new_goal))
        } else {
            // Set goal to None
            self.set_goal_for_date(date, None)
        }
    }

    fn get_raw_goal_for_date(&mut self, date: NaiveDate) -> Result<Option<RawGoal>> {
        let goal = self.get_goal_for_date(date)?;
        Ok(goal.map(|g| g.into_raw_goal()))
    }

    // fn get_percentage_achieved(&self, date: NaiveDate) -> Result<Option<f64>> {
    //     let logged_value = self.get_logging_repository().get(self.get_id(), date)?;
    //     Ok(self
    //         ._get_goal(date)?
    //         .as_ref()
    //         .map(|g| g.get_percentage_achieved_opt(&logged_value)))
    // }

    // fn get_scores_for_closed_range(
    //     &self,
    //     date_start: NaiveDate,
    //     date_end: NaiveDate,
    // ) -> Result<Vec<DateValue<Option<f64>>>> {
    //     let value_goals = self.get_values_with_goals(date_start, date_end)?;
    //     Ok(value_goals
    //         .into_iter()
    //         .map(|value_goals| {
    //             let (value, goal) = value_goals.value;
    //             DateValue {
    //                 date: value_goals.date,
    //                 value: goal.map(|goal| goal.calculate_score_opt(&value)),
    //             }
    //         })
    //         .collect())
    // }

    // Logging related

    fn get_logging_repository(
        &self,
    ) -> &Arc<dyn LoggingDataRepositoryTrait<StoredType = Self::RetrieveValue>>;

    fn log_from_string(
        &self,
        date: NaiveDate,
        value_str: &str,
        update_policy_str: Option<&str>,
    ) -> Result<()> {
        let value = value_str
            .parse()
            .map_err(|_| anyhow!("Could not parse {}", value_str))?;

        let update_policy =
            update_policy_str.map_or(Ok(Self::UpdatePolicyType::default()), |up| {
                up.parse()
                    .map_err(|_| anyhow!("Could not parse update policy {}", up))
            })?;

        self.log_value(date, value, update_policy)
    }

    fn log_value(
        &self,
        date: NaiveDate,
        new_value: Self::LogValue,
        update_policy: Self::UpdatePolicyType,
    ) -> Result<()> {
        info!(
            "Logging value '{}' with update_policy '{}' for statistic '{}'",
            new_value,
            update_policy,
            self.get_name()
        );

        // TODO: This should be performed atomically
        let old_value = self.get_logging_repository().get(self.id(), date)?;
        let updated_value = update_policy.apply(old_value, new_value);

        self.get_logging_repository()
            .insert_or_update(self.id(), date, updated_value)
    }

    /// Returns the logged data for the closed range [start, end] in ASCENDING ORDER.
    /// The returned vector contains every date in the range and fills gaps with None values
    /// in the value field.
    fn _get_values_for_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<DateValue<Option<Self::RetrieveValue>>>> {
        let mut logged_data = self
            .get_logging_repository()
            .get_in_closed_range(self.id(), start, end)?
            .into_iter();

        let mut next_logged_value = logged_data.next();
        let mut complete_data = Vec::with_capacity((end - start).num_days() as usize + 1);
        let mut current_date = start;

        while current_date <= end {
            let value = match &next_logged_value {
                Some(data) if data.date == current_date => {
                    // Use the logged value for this date and advance iterator
                    let value = data.value.clone();
                    next_logged_value = logged_data.next();
                    Some(value)
                }
                _ => None, // No logged value for this date
            };

            complete_data.push(DateValue::new(current_date, value));

            current_date = current_date
                .succ_opt()
                .ok_or_else(|| anyhow!("Date overflow when advancing to next date"))?;
        }

        Ok(complete_data)
    }

    /// Returns the zipped values and goals for the closed range [start_date, end_date]
    /// The list is sorted by date and contains every date in the range
    /// Not logged goals are filled with the current goal (which can also be None)
    fn get_values_with_goals(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<(Option<Self::RetrieveValue>, Option<Self::GoalType>)>>> {
        let values = self._get_values_for_closed_range(start_date, end_date)?;
        let goals = self._get_goals_for_closed_range(start_date, end_date)?;

        Ok(values
            .into_iter()
            .zip(goals)
            .map(|(value_data, goal_data)| {
                debug_assert_eq!(value_data.date, goal_data.date);
                DateValue {
                    date: value_data.date,
                    value: (value_data.value, goal_data.value),
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::goal_repository::GoalRepositoryTrait;
    use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
    use crate::statistic::objects::goal::Goal;
    use crate::statistic::statistic_types::statistic_template::StatisticTemplate;
    use chrono::{Duration, Local};
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::fmt;
    use std::str::FromStr;
    use std::sync::Arc;

    // Mock goal that's achieved if value >= target
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct TestGoal {
        target: i32,
    }

    impl Goal<i32> for TestGoal {
        fn new_with_target(target: i32) -> Self {
            Self { target }
        }

        fn set_extra_data_1(self, _: &str) -> anyhow::Result<Self> {
            Ok(self)
        }

        fn get_extra_data_1(&self) -> Option<String> {
            None
        }

        fn get_value(&self) -> &i32 {
            &self.target
        }

        fn get_percentage_achieved(&self, value: Option<&i32>) -> f64 {
            let value = value.unwrap_or(&0);

            if *value >= self.target {
                1.0
            } else {
                0.0
            }
        }

        fn into_raw_goal(self) -> RawGoal {
            unimplemented!()
        }
    }

    // Mock repositories
    struct MockGoalRepository {
        goals: HashMap<(StatisticId, NaiveDate), Option<TestGoal>>,
        default_goal: Option<TestGoal>,
    }

    impl GoalRepositoryTrait for MockGoalRepository {
        type StoredValue = i32;
        type GoalType = TestGoal;

        fn get_for_closed_range(
            &self,
            id: StatisticId,
            start_date: NaiveDate,
            end_date: NaiveDate,
        ) -> Result<Vec<DateValue<Option<TestGoal>>>> {
            let mut result = Vec::new();
            let mut current = start_date;

            while current <= end_date {
                let goal = self.goals.get(&(id, current)).cloned();
                if let Some(goal) = goal {
                    result.push(DateValue::new(current, goal));
                }
                current = current.succ_opt().unwrap();
            }
            Ok(result)
        }

        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Option<TestGoal>>> {
            Ok(self.goals.get(&(id, date)).cloned())
        }

        fn set_for_date(&self, _: StatisticId, _: NaiveDate, _: Option<TestGoal>) -> Result<()> {
            unimplemented!()
        }

        fn get_default(&self, _: StatisticId) -> Result<Option<TestGoal>> {
            Ok(self.default_goal.clone())
        }

        fn set_default(&self, _: StatisticId, _: TestGoal) -> Result<()> {
            unimplemented!()
        }

        fn delete_default(&self, _id: StatisticId) -> Result<()> {
            unimplemented!()
        }
    }

    struct MockLoggingRepository {
        values: HashMap<(StatisticId, NaiveDate), i32>,
    }

    impl LoggingDataRepositoryTrait for MockLoggingRepository {
        type StoredType = i32;

        fn get_in_closed_range(
            &self,
            id: StatisticId,
            start_date: NaiveDate,
            end_date: NaiveDate,
        ) -> Result<Vec<DateValue<i32>>> {
            let mut result = Vec::new();
            let mut current = start_date;

            while current <= end_date {
                let value = self.values.get(&(id, current)).cloned();
                if let Some(value) = value {
                    result.push(DateValue::new(current, value));
                }
                current = current.succ_opt().unwrap();
            }

            Ok(result)
        }

        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<i32>> {
            Ok(self.values.get(&(id, date)).cloned())
        }

        fn insert_or_update(&self, _: StatisticId, _: NaiveDate, _: i32) -> Result<()> {
            unimplemented!()
        }
    }

    // Simple update policy for testing
    #[derive(Debug, Clone, Copy)]
    struct TestUpdatePolicy;

    impl Display for TestUpdatePolicy {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test")
        }
    }

    impl FromStr for TestUpdatePolicy {
        type Err = anyhow::Error;

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(TestUpdatePolicy)
        }
    }

    impl Default for TestUpdatePolicy {
        fn default() -> Self {
            TestUpdatePolicy
        }
    }

    impl UpdatePolicy<i32, i32> for TestUpdatePolicy {
        fn apply(&self, _existing: Option<i32>, new: i32) -> i32 {
            new
        }
    }

    type TestStatistic = StatisticTemplate<i32, i32, TestUpdatePolicy, TestGoal>;

    #[test]
    fn test_today_not_logged_doesnt_break_streak() {
        let today = Local::now().date_naive();
        let yesterday = today - Duration::days(1);
        let day_before = today - Duration::days(2);

        let mut goals = HashMap::new();
        goals.insert((StatisticId(1), today), Some(TestGoal { target: 5 }));
        goals.insert((StatisticId(1), yesterday), Some(TestGoal { target: 5 }));
        goals.insert((StatisticId(1), day_before), Some(TestGoal { target: 5 }));

        let mut values = HashMap::new();
        values.insert((StatisticId(1), yesterday), 6); // Achieved
        values.insert((StatisticId(1), day_before), 6); // Achieved
                                                        // Today not logged yet

        let goal_repo = Arc::new(MockGoalRepository {
            goals,
            default_goal: None,
        });
        let logging_repo = Arc::new(MockLoggingRepository { values });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        assert_eq!(stat.get_streak().unwrap(), Some(2));
    }

    #[test]
    fn test_streak_breaks_on_missed_goal() {
        let today = Local::now().date_naive();
        let yesterday = today - Duration::days(1);
        let day_before = today - Duration::days(2);

        let mut goals = HashMap::new();
        goals.insert((StatisticId(1), today), Some(TestGoal { target: 5 }));
        goals.insert((StatisticId(1), yesterday), Some(TestGoal { target: 5 }));
        goals.insert((StatisticId(1), day_before), Some(TestGoal { target: 5 }));

        let mut values = HashMap::new();
        values.insert((StatisticId(1), today), 6); // Achieved
        values.insert((StatisticId(1), yesterday), 4); // Not achieved

        let goal_repo = Arc::new(MockGoalRepository {
            goals,
            default_goal: None,
        });
        let logging_repo = Arc::new(MockLoggingRepository { values });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        assert_eq!(stat.get_streak().unwrap(), Some(1));
    }

    // TODO: Streak with different goals

    #[test]
    fn test_streak_with_default_goal() {
        let today = Local::now().date_naive();
        let yesterday = today - Duration::days(1);
        let day_before = today - Duration::days(2);

        let mut values = HashMap::new();
        values.insert((StatisticId(1), today), 6); // Achieved
        values.insert((StatisticId(1), yesterday), 50); // Achieved
        values.insert((StatisticId(1), day_before), 100); // Achieved

        let goal_repo = Arc::new(MockGoalRepository {
            goals: HashMap::new(),
            default_goal: Some(TestGoal { target: 5 }),
        });
        let logging_repo = Arc::new(MockLoggingRepository { values });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        assert_eq!(stat.get_streak().unwrap(), Some(3));
    }

    #[test]
    fn test_streak_with_different_goals() {
        let today = Local::now().date_naive();

        let mut goals = HashMap::new();
        goals.insert((StatisticId(1), today), Some(TestGoal { target: 1 }));
        goals.insert(
            (StatisticId(1), today - Duration::days(1)),
            Some(TestGoal { target: 2 }),
        );
        goals.insert(
            (StatisticId(1), today - Duration::days(2)),
            Some(TestGoal { target: 3 }),
        );
        goals.insert(
            (StatisticId(1), today - Duration::days(3)),
            Some(TestGoal { target: 4 }),
        );
        goals.insert(
            (StatisticId(1), today - Duration::days(4)),
            Some(TestGoal { target: 5 }),
        );

        let mut values = HashMap::new();
        values.insert((StatisticId(1), today), 6); // Achieved
        values.insert((StatisticId(1), today - Duration::days(1)), 50); // Achieved
        values.insert((StatisticId(1), today - Duration::days(2)), 100); // Achieved
        values.insert((StatisticId(1), today - Duration::days(3)), 1); // Not Achieved
        values.insert((StatisticId(1), today - Duration::days(4)), 100); // Achieved

        let goal_repo = Arc::new(MockGoalRepository {
            goals: HashMap::new(),
            default_goal: Some(TestGoal { target: 5 }),
        });
        let logging_repo = Arc::new(MockLoggingRepository { values });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        let streak = stat.get_streak().unwrap();

        assert_eq!(streak, Some(3));
    }

    #[test]
    fn test_get_values_with_goals_empty_range() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        let goal_repo = Arc::new(MockGoalRepository {
            goals: HashMap::new(),
            default_goal: None,
        });
        let logging_repo = Arc::new(MockLoggingRepository {
            values: HashMap::new(),
        });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        let result = stat.get_values_with_goals(date, date).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].date, date);
        assert_eq!(result[0].value, (None, None));
    }

    #[test]
    fn test_get_values_with_goals_default_goal() {
        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();

        // Setup default goal
        let default_goal = TestGoal { target: 5 };

        let mut values = HashMap::new();
        values.insert((StatisticId(1), start_date), 6);
        values.insert((StatisticId(1), end_date), 4);

        let goal_repo = Arc::new(MockGoalRepository {
            goals: HashMap::new(),
            default_goal: Some(default_goal.clone()),
        });
        let logging_repo = Arc::new(MockLoggingRepository { values });

        let stat = TestStatistic::new(
            StatisticMetadata::new(StatisticId(1), "test".to_string().into_boxed_str(), "test".to_string().into_boxed_str()),
            logging_repo,
            goal_repo,
        );

        let result = stat.get_values_with_goals(start_date, end_date).unwrap();
        assert_eq!(result.len(), 3);

        // First day: value = 6, goal = default
        assert_eq!(result[0].date, start_date);
        assert_eq!(result[0].value.0, Some(6));
        assert_eq!(
            result[0].value.1.as_ref().unwrap().target,
            default_goal.target
        );

        // Second day: no value, goal = default
        assert_eq!(result[1].value.0, None);
        assert_eq!(
            result[1].value.1.as_ref().unwrap().target,
            default_goal.target
        );

        // Third day: value = 4, goal = default
        assert_eq!(result[2].date, end_date);
        assert_eq!(result[2].value.0, Some(4));
        assert_eq!(
            result[2].value.1.as_ref().unwrap().target,
            default_goal.target
        );
    }
}
