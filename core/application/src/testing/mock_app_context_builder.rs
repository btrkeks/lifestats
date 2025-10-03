use crate::app_context::AppContext;
use lifestats_domain::statistic::repositories::plots_repository::MockPlotRepositoryTrait;
use lifestats_domain::statistic::repositories::statistic_repository::MockStatisticRepositoryTrait;
use lifestats_domain::statistic::statistic_types::any_statistic::AnyStatistic;
use events::event_port::MockEventPublisher;
use std::sync::Arc;

/// A builder for creating mock AppContext instances for testing
#[derive(Default)]
pub struct MockAppContextBuilder {
    statistic_repository: Option<Arc<MockStatisticRepositoryTrait>>,
    plot_repository: Option<Arc<MockPlotRepositoryTrait>>,
    event_publisher: Option<Arc<MockEventPublisher>>,
}

impl MockAppContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_statistic_repository(mut self, repository: MockStatisticRepositoryTrait) -> Self {
        self.statistic_repository = Some(Arc::new(repository));
        self
    }

    pub fn with_statistics(self, statistics: Vec<AnyStatistic>) -> Self {
        let mut repository = MockStatisticRepositoryTrait::new();

        // Set up get_by_id expectations
        for statistic in &statistics {
            let statistic = (*statistic).clone();
            repository
                .expect_get_by_id()
                .with(mockall::predicate::eq(statistic.id()))
                .returning(move |_| Ok(statistic.clone()));
        }

        // Set up get_all expectations if needed
        if !statistics.is_empty() {
            let all_stats: Box<_> = statistics.iter().cloned().collect();
            repository
                .expect_get_all()
                .returning(move || Ok(all_stats.clone()));
        }

        self.with_statistic_repository(repository)
    }

    pub fn with_plot_repository(mut self, repository: MockPlotRepositoryTrait) -> Self {
        self.plot_repository = Some(Arc::new(repository));
        self
    }

    pub fn with_event_publisher(mut self, publisher: MockEventPublisher) -> Self {
        self.event_publisher = Some(Arc::new(publisher));
        self
    }

    fn create_default_statistic_repository() -> MockStatisticRepositoryTrait {
        let mut repository = MockStatisticRepositoryTrait::new();

        // Set up default behaviors that return empty or error results
        repository
            .expect_get_by_id()
            .returning(|id| Err(anyhow::anyhow!("No statistic found with id: {}", id)));

        repository.expect_get_all().returning(|| Ok(Box::new([])));

        repository
            .expect_get_all_time_spans_statistics()
            .returning(|| Ok(Box::new([])));

        repository.expect_save().returning(|_| Ok(()));

        repository
    }

    fn create_default_plot_repository() -> MockPlotRepositoryTrait {
        let mut repository = MockPlotRepositoryTrait::new();

        repository.expect_get_by_id().returning(|_| Ok(None));

        repository.expect_get_all().returning(|| Ok(Box::new([])));

        repository
            .expect_get_for_statistic_id()
            .returning(|_| Ok(Box::new([])));

        repository.expect_save().returning(|_| Ok(()));

        repository
    }

    fn create_default_event_publisher() -> MockEventPublisher {
        let mut publisher = MockEventPublisher::new();
        publisher.expect_publish_event_async().returning(|_| ()); // Just ignore this call by default
        publisher
    }

    pub fn build(self) -> AppContext {
        AppContext {
            statistic_repository: self
                .statistic_repository
                .unwrap_or_else(|| Arc::new(Self::create_default_statistic_repository())),
            plot_repository: self
                .plot_repository
                .unwrap_or_else(|| Arc::new(Self::create_default_plot_repository())),
            event_publisher: self
                .event_publisher
                .unwrap_or_else(|| Arc::new(Self::create_default_event_publisher())),
        }
    }
}
