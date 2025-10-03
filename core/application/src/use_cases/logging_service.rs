use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_domain::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;

#[derive(Clone)]
pub struct LoggingService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> LoggingService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(statistic_repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            statistic_repository,
            event_publisher,
        }
    }

    pub fn log_timeframe_list(&self, command: LogTimeFrameListCommand) -> Result<()> {
        let LogTimeFrameListCommand {
            statistic_id,
            date,
            value,
            update_policy,
        } = command;

        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;

        let time_spans = statistic.as_time_spans().ok_or_else(|| {
            anyhow!(
                "Statistic {} does not support time span logging",
                statistic_id
            )
        })?;

        let current_value = self
            .statistic_repository
            .find_log_value::<TimeFrameList>(statistic_id, date)?;

        let policy = update_policy.unwrap_or_default();
        let (new_value, changed) = time_spans.calculate_new_log_value(current_value, value, policy);

        if !changed {
            return Ok(());
        }

        self.statistic_repository
            .save_log_value(statistic_id, date, new_value)?;

        self.event_publisher
            .publish_event_async(Event::ValueUpdated(statistic_id));

        Ok(())
    }
}

pub struct LogTimeFrameListCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value: TimeFrameList,
    pub update_policy: Option<TimeFrameListUpdatePolicy>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use events::event_port::{Event, EventPublisher};
    use lifestats_domain::shared::date_range::DateRange;
    use lifestats_domain::shared::dated_value::DatedValue;
    use lifestats_domain::shared::time_frame_list::TimeFrame;
    use lifestats_domain::statistic::any_statistic::AnyStatistic;
    use lifestats_domain::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn time_spans_statistic() -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(1),
            "Wellness".to_string().into_boxed_str(),
            "Deep Work".to_string().into_boxed_str(),
        );
        AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        ))
    }

    fn sample_time_frame_list() -> TimeFrameList {
        let start = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let frame = TimeFrame::new(start, end).unwrap();
        let mut list = TimeFrameList::default();
        list.append_frame(frame).unwrap();
        list
    }

    struct InMemoryStatisticRepository {
        statistic: AnyStatistic,
        logs: Mutex<HashMap<(StatisticId, NaiveDate), serde_json::Value>>,
    }

    impl InMemoryStatisticRepository {
        fn new(statistic: AnyStatistic) -> Self {
            Self {
                statistic,
                logs: Mutex::new(HashMap::new()),
            }
        }
    }

    impl StatisticRepository for InMemoryStatisticRepository {
        fn find_metadata_by_id(&self, _id: StatisticId) -> Result<AnyStatistic> {
            Ok(self.statistic.clone())
        }

        fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>> {
            Ok(vec![self.statistic.clone()])
        }

        fn save_metadata(&self, _statistic: &AnyStatistic) -> Result<()> {
            Ok(())
        }

        fn find_log_value<V>(&self, id: StatisticId, date: NaiveDate) -> Result<Option<V>>
        where
            V: DeserializeOwned + 'static,
        {
            let guard = self.logs.lock().unwrap();
            guard
                .get(&(id, date))
                .map(|value| serde_json::from_value(value.clone()).map_err(Into::into))
                .transpose()
        }

        fn find_log_values_in_range<V>(
            &self,
            id: StatisticId,
            range: DateRange,
        ) -> Result<Vec<DatedValue<V>>>
        where
            V: DeserializeOwned + 'static,
        {
            let guard = self.logs.lock().unwrap();
            let mut result = Vec::new();
            for ((stat_id, date), value) in guard.iter() {
                if *stat_id == id && date >= &range.start() && date <= &range.end() {
                    let parsed = serde_json::from_value(value.clone())?;
                    result.push(DatedValue::new(*date, parsed));
                }
            }
            Ok(result)
        }

        fn save_log_value<V>(&self, id: StatisticId, date: NaiveDate, value: V) -> Result<()>
        where
            V: Serialize + 'static,
        {
            let mut guard = self.logs.lock().unwrap();
            guard.insert((id, date), serde_json::to_value(value)?);
            Ok(())
        }

        fn find_goal<G>(&self, _id: StatisticId, _date: NaiveDate) -> Result<Option<G>>
        where
            G: DeserializeOwned + 'static,
        {
            Ok(None)
        }

        fn find_goals_in_range<G>(
            &self,
            _id: StatisticId,
            _range: DateRange,
        ) -> Result<Vec<DatedValue<G>>>
        where
            G: DeserializeOwned + 'static,
        {
            Ok(Vec::new())
        }

        fn save_goal<G>(&self, _id: StatisticId, _date: NaiveDate, _goal: Option<G>) -> Result<()>
        where
            G: Serialize + 'static,
        {
            Ok(())
        }

        fn find_default_goal<G>(&self, _id: StatisticId) -> Result<Option<G>>
        where
            G: DeserializeOwned + 'static,
        {
            Ok(None)
        }

        fn save_default_goal<G>(&self, _id: StatisticId, _goal: G) -> Result<()>
        where
            G: Serialize + 'static,
        {
            Ok(())
        }

        fn delete_default_goal(&self, _id: StatisticId) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingEventPublisher {
        events: Mutex<Vec<Event>>,
    }

    impl RecordingEventPublisher {
        fn events(&self) -> Vec<Event> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventPublisher for RecordingEventPublisher {
        fn publish_event_async(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }

        fn publish_event_sync(&self, event: Event) -> Result<()> {
            self.publish_event_async(event);
            Ok(())
        }
    }

    #[test]
    fn saves_new_value_and_publishes_event() {
        let repo = Arc::new(InMemoryStatisticRepository::new(time_spans_statistic()));
        let publisher = Arc::new(RecordingEventPublisher::default());

        let service = LoggingService::new(repo.clone(), publisher.clone());

        let command = LogTimeFrameListCommand {
            statistic_id: StatisticId(1),
            date: NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(),
            value: sample_time_frame_list(),
            update_policy: None,
        };

        service.log_timeframe_list(command).unwrap();

        let stored: Option<TimeFrameList> = repo
            .find_log_value(
                StatisticId(1),
                NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(),
            )
            .unwrap();
        assert!(stored.is_some());

        let events = publisher.events();
        assert_eq!(events, vec![Event::ValueUpdated(StatisticId(1))]);
    }
}
