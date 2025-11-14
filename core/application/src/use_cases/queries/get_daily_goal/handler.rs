use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

use super::query::GetDailyGoalQuery;

/// Handler for GetDailyGoalQuery.
pub struct GetDailyGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetDailyGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetDailyGoalQuery> for GetDailyGoalQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetDailyGoalQuery) -> ApplicationResult<GoalQueryResult> {
        Ok(self.repository.find_goal(query.statistic_id, query.date)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use serde_json::json;

    #[test]
    fn test_get_daily_goal_returns_value() {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_goal()
            .times(1)
            .return_once(|_, _| Ok(GoalQueryResult::Goal(json!({"target": 5}))));

        let handler = GetDailyGoalQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetDailyGoalQuery {
                statistic_id: StatisticId(1),
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            })
            .unwrap();
        assert!(result.is_set());
    }
}
