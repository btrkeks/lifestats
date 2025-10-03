use anyhow::Result;
use chrono::NaiveDate;
use lifestats_domain::statistic::repositories::goal_repository::GoalRepositoryTrait;
use lifestats_domain::statistic::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use lifestats_domain::shared::logging_data::DateValue;
use lifestats_domain::shared::my_time::MyTime;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use lifestats_domain::statistic::statistic_types::point_in_time::FinishedInTimeGoal;
use lifestats_domain::statistic::statistic_types::time_spans::TimeSpansGoal;
use mockall::mock;
use lifestats_domain::shared::ids::StatisticId;

mock! {
    pub PointInTimeLogsRepository {}

    impl LoggingDataRepositoryTrait for PointInTimeLogsRepository {
        type StoredType = MyTime;

        fn get_in_closed_range(
            &self,
            id: StatisticId,
            start_date: NaiveDate,
            end_date: NaiveDate,
        ) -> Result<Vec<DateValue<MyTime>>>;

        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<MyTime>>;

        fn insert_or_update(&self, id: StatisticId, date: NaiveDate, new_value: MyTime) -> Result<()>;
    }

    unsafe impl Sync for PointInTimeLogsRepository {}
    unsafe impl Send for PointInTimeLogsRepository {}
}

mock! {
    pub PointInTimeGoalsRepository {}

    impl GoalRepositoryTrait for PointInTimeGoalsRepository {
           type StoredValue = MyTime;
           type GoalType = FinishedInTimeGoal;

           fn get_for_closed_range(
               &self,
               id: StatisticId,
               start_date: NaiveDate,
               end_date: NaiveDate,
           ) -> Result<Vec<DateValue<Option<FinishedInTimeGoal>>>>;

           fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Option<FinishedInTimeGoal>>>;

           fn set_for_date(
               &self,
               id: StatisticId,
               date: NaiveDate,
               new_goal: Option<FinishedInTimeGoal>,
           ) -> Result<()>;

           fn get_default(&self, id: StatisticId) -> Result<Option<FinishedInTimeGoal>>;

           fn set_default(&self, id: StatisticId, new_goal: FinishedInTimeGoal) -> Result<()>;

           fn delete_default(&self, id: StatisticId) -> Result<()>;
    }

    unsafe impl Sync for PointInTimeGoalsRepository {}
    unsafe impl Send for PointInTimeGoalsRepository {}
}

mock! {
    pub TimeSpansLogsRepository {}

    impl LoggingDataRepositoryTrait for TimeSpansLogsRepository {
        type StoredType = TimeFrameList;

        fn get_in_closed_range(
            &self,
            id: StatisticId,
            start_date: NaiveDate,
            end_date: NaiveDate,
        ) -> Result<Vec<DateValue<TimeFrameList>>>;

        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<TimeFrameList>>;

        fn insert_or_update(&self, id: StatisticId, date: NaiveDate, new_value: TimeFrameList) -> Result<()>;
    }

    unsafe impl Sync for TimeSpansLogsRepository {}
    unsafe impl Send for TimeSpansLogsRepository {}
}

mock! {
    pub TimeSpansGoalsRepository {}

    impl GoalRepositoryTrait for TimeSpansGoalsRepository {
        type StoredValue = TimeFrameList;
        type GoalType = TimeSpansGoal;

        fn get_for_closed_range(
            &self,
            id: StatisticId,
            start_date: NaiveDate,
            end_date: NaiveDate,
        ) -> Result<Vec<DateValue<Option<TimeSpansGoal>>>>;

        fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Option<TimeSpansGoal>>>;

        fn set_for_date(
            &self,
            id: StatisticId,
            date: NaiveDate,
            new_goal: Option<TimeSpansGoal>,
        ) -> Result<()>;

        fn get_default(&self, id: StatisticId) -> Result<Option<TimeSpansGoal>>;

        fn set_default(&self, id: StatisticId, new_goal: TimeSpansGoal) -> Result<()>;

        fn delete_default(&self, id: StatisticId) -> Result<()>;
    }

    unsafe impl Sync for TimeSpansGoalsRepository {}
    unsafe impl Send for TimeSpansGoalsRepository {}
}
