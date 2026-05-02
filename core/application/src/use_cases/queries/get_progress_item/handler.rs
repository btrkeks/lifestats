use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::ProgressRepository;
use lifestats_core::shared::ports::error::RepositoryError;

use super::query::GetProgressItemQuery;

pub struct GetProgressItemQueryHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl GetProgressItemQueryHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl QueryHandler<GetProgressItemQuery> for GetProgressItemQueryHandler {
    fn handle(
        &self,
        query: GetProgressItemQuery,
    ) -> ApplicationResult<lifestats_core::progress::ProgressItem> {
        Ok(self
            .repository
            .find_item_by_id(query.item_id)?
            .ok_or(RepositoryError::NotFound)?)
    }
}
