// use lifestats_domain::statistic::repositories::statistic_repository::MockStatisticRepositoryTrait;
// use lifestats_domain::statistic::statistic_types::any_statistic::AnyStatistic;
// use lifestats_domain::statistic::statistic_types::time_spans::TimeSpansStatistic;
// use lifestats_domain::shared::ids::StatisticId;
//
// #[derive(Default)]
// pub struct MockStatisticRepositoryBuilder {
//     get_by_id_results: Vec<(StatisticId, anyhow::Result<AnyStatistic>)>,
//     save_results: Vec<anyhow::Result<()>>,
//     get_all_results: Option<anyhow::Result<Vec<AnyStatistic>>>,
//     get_all_time_spans_results: Option<anyhow::Result<Vec<TimeSpansStatistic>>>,
// }
//
// impl MockStatisticRepositoryBuilder {
//     pub fn new() -> Self {
//         Self::default()
//     }
//
//     pub fn with_statistic(mut self, id: StatisticId, result: anyhow::Result<AnyStatistic>) -> Self {
//         self.get_by_id_results.push((id, result));
//         self
//     }
//
//     pub fn with_save_result(mut self, result: anyhow::Result<()>) -> Self {
//         self.save_results.push(result);
//         self
//     }
//
//     pub fn with_get_all(mut self, result: anyhow::Result<Vec<AnyStatistic>>) -> Self {
//         self.get_all_results = Some(result);
//         self
//     }
//
//     pub fn with_get_all_time_spans(
//         mut self,
//         result: anyhow::Result<Vec<TimeSpansStatistic>>,
//     ) -> Self {
//         self.get_all_time_spans_results = Some(result);
//         self
//     }
//
//     pub fn build(self) -> MockStatisticRepositoryTrait {
//         let mut mock = MockStatisticRepositoryTrait::new();
//
//         // Set up get_by_id expectations
//         if !self.get_by_id_results.is_empty() {
//             mock.expect_get_by_id()
//                 .times(self.get_by_id_results.len())
//                 .with(mockall::predicate::function(move |id: &StatisticId| {
//                     self.get_by_id_results
//                         .iter()
//                         .any(|(expected_id, _)| expected_id == id)
//                 }))
//                 .returning(move |id| {
//                     self.get_by_id_results
//                         .iter()
//                         .find(|(expected_id, _)| expected_id == &id)
//                         .map(|(_, result)| result.clone())
//                         .unwrap_or_else(|| {
//                             Err(anyhow::anyhow!("Unexpected get_by_id call for id: {}", id))
//                         })
//                 });
//         }
//
//         // Set up save expectations
//         if !self.save_results.is_empty() {
//             mock.expect_save()
//                 .times(self.save_results.len())
//                 .returning(move |_| {
//                     self.save_results
//                         .get(0)
//                         .unwrap_or(&Ok(()))
//                         .clone()
//                 });
//         }
//
//         // Set up get_all expectations
//         if let Some(result) = self.get_all_results {
//             mock.expect_get_all()
//                 .returning(move || result.clone());
//         }
//
//         // Set up get_all_time_spans expectations
//         if let Some(result) = self.get_all_time_spans_results {
//             mock.expect_get_all_time_spans_statistics()
//                 .returning(move || result.clone());
//         }
//
//         mock
//     }
// }
