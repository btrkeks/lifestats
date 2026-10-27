use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::{DefaultGoalResult, GetDefaultGoalQuery};

/// Handler for GetDefaultGoalQuery.
pub struct GetDefaultGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetDefaultGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetDefaultGoalQuery> for GetDefaultGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetDefaultGoalQuery) -> ApplicationResult<Option<DefaultGoalResult>> {
        let timeline = self
            .repository
            .find_default_goal_timeline(query.statistic_id)?;
        Ok(timeline.latest().and_then(|segment| {
            segment.goal.clone().map(|goal_data| DefaultGoalResult {
                goal_data,
                valid_since: segment.valid_since,
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::value_objects::goals::default_goal_timeline::{
        DefaultGoalSegment, DefaultGoalTimeline,
    };
    use serde_json::json;

    fn date(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    fn handle(timeline: DefaultGoalTimeline<serde_json::Value>) -> Option<DefaultGoalResult> {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_default_goal_timeline()
            .times(1)
            .return_once(move |_| Ok(timeline));
        GetDefaultGoalQueryHandler::new(Arc::new(repo))
            .handle(GetDefaultGoalQuery {
                statistic_id: StatisticId(1),
            })
            .unwrap()
    }

    #[test]
    fn test_get_default_goal_returns_the_latest_goal_and_its_start() {
        let result = handle(DefaultGoalTimeline::new(vec![
            DefaultGoalSegment::new(date(9, 9), Some(json!({"target_amount": 400}))),
            DefaultGoalSegment::new(date(10, 22), Some(json!({"target_amount": 450}))),
        ]));

        assert_eq!(
            result,
            Some(DefaultGoalResult {
                goal_data: json!({"target_amount": 450}),
                valid_since: date(10, 22),
            })
        );
    }

    #[test]
    fn test_get_default_goal_is_none_once_the_goal_was_removed() {
        let result = handle(DefaultGoalTimeline::new(vec![
            DefaultGoalSegment::new(date(9, 9), Some(json!({"target_amount": 400}))),
            DefaultGoalSegment::new(date(10, 22), None),
        ]));

        assert_eq!(result, None);
    }
}
