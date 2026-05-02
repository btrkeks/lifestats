use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::{
    ProgressCheckpointRange, ProgressRepository,
};

use super::query::GetProgressCheckpointsQuery;

pub struct GetProgressCheckpointsQueryHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl GetProgressCheckpointsQueryHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl QueryHandler<GetProgressCheckpointsQuery> for GetProgressCheckpointsQueryHandler {
    fn handle(
        &self,
        query: GetProgressCheckpointsQuery,
    ) -> ApplicationResult<Vec<lifestats_core::progress::ProgressCheckpoint>> {
        let range = ProgressCheckpointRange {
            recorded_at_start: query.recorded_at_start,
            recorded_at_end: query.recorded_at_end,
        };
        Ok(self.repository.find_checkpoints(query.item_id, range)?)
    }
}
