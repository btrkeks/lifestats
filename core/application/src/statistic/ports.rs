use async_trait::async_trait;

use crate::{error::ApplicationResult, shared::tags::TagSet};
use lifestats_core::shared::ids::StatisticId;

#[async_trait]
pub trait TagQueryableStatisticRepository: Send + Sync {
    async fn find_statistic_id_by_exact_tags(
        &self,
        tags: &TagSet,
    ) -> ApplicationResult<Option<StatisticId>>;
}
