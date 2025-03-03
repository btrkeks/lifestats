use anyhow::Result;
use domain_models::repositories::goal_repositories::GoalRepositories;
use domain_models::repositories::logging_repositories::LoggingRepositories;
use domain_models::repositories::plots_repository::PlotRepositoryTrait;
use domain_models::repositories::statistic_repository::StatisticRepositoryTrait;
use events::event_port::EventPublisher;
use events::unix_socket::UnixSocketEvent;
use persistence::goal_repository::GoalRepositoryTemplate;
use persistence::logging_data_repository::LoggingDataRepository;
use persistence::plots_repository::PlotRepository;
use persistence::pool::get_database_pool;
use persistence::statistics_repository::StatisticRepository;
use std::sync::Arc;

pub struct AppContext {
    pub(crate) statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    pub(crate) plot_repository: Arc<dyn PlotRepositoryTrait>,
    pub(crate) event_publisher: Arc<dyn EventPublisher>,
}

#[cfg(not(test))] // Don't allow using the real implementations in tests
pub fn build_application() -> Result<AppContext> {
    let pool = get_database_pool();

    let event_publisher = Arc::new(UnixSocketEvent::default());

    // Loging Data repositories
    let logging_repositories = LoggingRepositories::new(
        Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
        Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
        Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
        Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
        Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
    );

    // Goal repositories
    let goal_repositories = GoalRepositories::new(
        Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
        Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
        Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
        Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
        Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
    );

    let statistic_repository = Arc::new(StatisticRepository::new(
        Arc::clone(&pool),
        logging_repositories,
        goal_repositories,
    ));

    let plot_repository = Arc::new(PlotRepository::new(
        Arc::clone(&pool),
        Arc::clone(&statistic_repository) as Arc<dyn StatisticRepositoryTrait>,
    ));

    Ok(AppContext {
        statistic_repository,
        plot_repository,
        event_publisher,
    })
}
