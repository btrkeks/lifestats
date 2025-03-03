use crate::app_context::AppContext;
use anyhow::Result;
use domain_models::statistic::statistic::Statistic;
use domain_models::statistic::statistic_types::any_statistic::AnyStatistic;
use transfer_objects::statistic_metadata::StatisticMetadata;

pub fn get_all_statistics(app_context: &AppContext) -> Result<Box<[AnyStatistic]>> {
    app_context.statistic_repository.get_all()
}

pub fn get_all_metadata(app_context: &AppContext) -> Result<Box<[StatisticMetadata]>> {
    Ok(app_context
        .statistic_repository
        .get_all()?
        .into_iter()
        .map(|stat| stat.get_metadata())
        .collect())
}

// TODO: Cleaner implementation
pub fn get_time_spans_metadata(app_context: &AppContext) -> Result<Box<[StatisticMetadata]>> {
    Ok(app_context
        .statistic_repository
        .get_all_time_spans_statistics()?
        .into_iter()
        .map(|stat| stat.get_metadata().clone()) // TODO: ideally don't clone
        .collect())
}
