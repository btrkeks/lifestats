use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetDefaultGoalQuery;

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
    fn handle(&self, query: GetDefaultGoalQuery) -> ApplicationResult<Option<serde_json::Value>> {
        Ok(self.repository.find_default_goal(query.statistic_id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use serde_json::json;

    #[test]
    fn test_get_default_goal_returns_value() {
        let mut repo = MockStatisticRepository::new();
        repo.expect_find_default_goal()
            .times(1)
            .return_once(|_| Ok(Some(json!({"target": 10}))));

        let handler = GetDefaultGoalQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetDefaultGoalQuery {
                statistic_id: StatisticId(1),
            })
            .unwrap();
        assert!(result.is_some());
    }
}
