use crate::testing::mock_repositories::{
    MockTimeSpansGoalsRepository, MockTimeSpansLogsRepository,
};
use chrono::NaiveDate;
use domain_models::objects::time_frame_list::TimeFrameList;
use domain_models::statistic::statistic_types::any_statistic::AnyStatistic;
use domain_models::statistic::statistic_types::time_spans::TimeSpansStatistic;
use std::sync::Arc;
use transfer_objects::statistic_id::StatisticId;
use transfer_objects::statistic_metadata::StatisticMetadata;

pub fn create_mock_time_spans_statistic(
    id: StatisticId,
    name: &str,
    category: &str,
) -> AnyStatistic {
    let metadata = StatisticMetadata::new(
        id,
        name.to_string().into_boxed_str(),
        category.to_string().into_boxed_str(),
    );

    // Create minimal mock repositories
    let logging_repo = Arc::new(MockTimeSpansLogsRepository::new());
    let goal_repo = Arc::new(MockTimeSpansGoalsRepository::new());

    AnyStatistic::TimeSpans(TimeSpansStatistic::new(
        metadata.clone(),
        logging_repo,
        goal_repo,
    ))
}

#[derive(Default)]
pub struct MockTimeSpansStatisticBuilder {
    id: Option<StatisticId>,
    name: Option<String>,
    category: Option<String>,
    logs: Vec<(NaiveDate, TimeFrameList)>,
}

impl MockTimeSpansStatisticBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: StatisticId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_log(mut self, date: NaiveDate, list: TimeFrameList) -> Self {
        self.logs.push((date, list));
        self
    }

    // pub fn with_insertion_expectation(mut self, date: NaiveDate, time_frames: TimeFrameList) -> Self {
    //     let mut logging_repo = MockTimeSpansLogsRepository::new();
    //
    //     // Set up get expectation for existing data check
    //     logging_repo
    //         .expect_get()
    //         .with(mockall::predicate::always(), mockall::predicate::eq(date))
    //         .returning(move |_, _| Ok(None));
    //
    //     // Set up insert_or_update expectation
    //     logging_repo
    //         .expect_insert_or_update()
    //         .with(
    //             mockall::predicate::always(),
    //             mockall::predicate::eq(date),
    //             mockall::predicate::eq(time_frames.clone())
    //         )
    //         .returning(|_, _, _| Ok(()));
    //
    //     self.logs.push((date, time_frames));
    //     self
    // }

    pub fn build(self) -> AnyStatistic {
        let id = self.id.unwrap_or(StatisticId(1));
        let name = self
            .name
            .unwrap_or_else(|| "Test Time Spans Statistic".to_string());
        let category = self
            .category
            .unwrap_or_else(|| "Testing Category".to_string());

        let metadata = StatisticMetadata::new(
            id,
            name.to_string().into_boxed_str(),
            category.to_string().into_boxed_str(),
        );

        let mut logging_repo = MockTimeSpansLogsRepository::new();
        let logs = self.logs.clone();
        logging_repo.expect_get().returning(move |_, date| {
            Ok(logs
                .iter()
                .find(|(log_date, _)| log_date == &date)
                .map(|(_, time_frames)| time_frames.clone()))
        });
        logging_repo
            .expect_insert_or_update()
            .returning(|_, _, _| Ok(())); // Ignore for now

        let goal_repo = MockTimeSpansGoalsRepository::new();

        AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            metadata.clone(),
            Arc::new(logging_repo),
            Arc::new(goal_repo),
        ))
    }
}
