use std::sync::Arc;

use anyhow::Result;
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::any_statistic::AnyStatistic;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;

#[derive(Clone)]
pub struct StatisticQueryService<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<R>,
}

impl<R> StatisticQueryService<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<R>) -> Self {
        Self {
            statistic_repository,
        }
    }

    pub fn get_all_statistics(&self) -> Result<Vec<AnyStatistic>> {
        self.statistic_repository.find_all_metadata()
    }

    pub fn get_all_metadata(&self) -> Result<Vec<StatisticMetadata>> {
        self.statistic_repository.find_all_metadata().map(|stats| {
            stats
                .into_iter()
                .map(|stat| stat.get_metadata().clone())
                .collect()
        })
    }

    pub fn get_time_spans_metadata(&self) -> Result<Vec<StatisticMetadata>> {
        self.statistic_repository.find_all_metadata().map(|stats| {
            stats
                .into_iter()
                .filter_map(|stat| {
                    if matches!(&stat, AnyStatistic::TimeSpans(_)) {
                        Some(stat.get_metadata().clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    pub fn get_metadata(&self, statistic_id: StatisticId) -> Result<StatisticMetadata> {
        self.statistic_repository
            .find_metadata_by_id(statistic_id)
            .map(|stat| stat.get_metadata().clone())
    }
}
