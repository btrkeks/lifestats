use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::ProgressRepository;

use super::query::ListProgressItemsQuery;

pub struct ListProgressItemsQueryHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl ListProgressItemsQueryHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl QueryHandler<ListProgressItemsQuery> for ListProgressItemsQueryHandler {
    fn handle(
        &self,
        query: ListProgressItemsQuery,
    ) -> ApplicationResult<Vec<lifestats_core::progress::ProgressItem>> {
        match query.tags {
            Some(tags) => Ok(self.repository.find_items_by_tags(&tags)?),
            None => Ok(self.repository.find_all_items()?),
        }
    }
}
