// EXCERPT — core/application/src/use_cases/queries/get_loggable_metadata.rs
// Before: const LOGGABLE_TYPES: [StatisticType; 4] = [...]  (silent if missed)
// After:  derived from the exhaustive StatisticType::is_manually_loggable.
let summaries = statistics
    .iter()
    .map(StatisticSummary::from)
    .filter(|summary| summary.statistic_type.is_manually_loggable())
    .collect();
