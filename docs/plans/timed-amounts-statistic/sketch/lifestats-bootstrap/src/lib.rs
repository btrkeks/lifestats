// CHANGED build_services chain, alongside Beurer/Yomikata injections:
// IntegrationService::new(...)
//    .with_opennutritracker_day_store(statistic_repo.clone())
//    ...
// No generic OpenNutriTracker BundleFactory registration: the PUT store provisions.
