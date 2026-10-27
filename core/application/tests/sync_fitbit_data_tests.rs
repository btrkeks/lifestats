use chrono::{
    DateTime, Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
};
use mockall::{mock, predicate::*};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};

use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_core::statistic::ports::error::RepositoryError;
mod helpers;

use helpers::mock_fitbit_statistic_metadata;
use lifestats_application::integrations::fitbit::error::FitbitUpstreamError;
use lifestats_application::integrations::fitbit::ports::{
    FitbitActivityData, FitbitDataApi, FitbitMetric, FitbitOAuthApi, FitbitSleepData,
    TokenStorePort,
};
use lifestats_application::integrations::fitbit::Clock;
use lifestats_application::integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken};
use lifestats_application::shared::tags::TagSet;
use lifestats_application::statistic::ports::TagQueryableStatisticRepository;
use lifestats_application::use_cases::commands::sync_fitbit_data::{
    SyncFitbitDataCommand, SyncFitbitDataCommandHandler,
};
use lifestats_application::StatisticRepository;
use lifestats_core::shared::ids::StatisticId;

// ============================================================================
// Mock Implementations
// ============================================================================

mock! {
    pub FitbitOAuth {}

    #[async_trait::async_trait]
    impl FitbitOAuthApi for FitbitOAuth {
        async fn exchange_code_for_token(
            &self,
            code: &str,
            pkce_verifier: Option<String>,
        ) -> ApplicationResult<FitbitTokenData>;

        async fn refresh(
            &self,
            refresh_token: &RefreshToken,
        ) -> ApplicationResult<FitbitTokenData>;
    }
}

mock! {
    pub FitbitData {}

    #[async_trait::async_trait]
    impl FitbitDataApi for FitbitData {
        async fn get_activity_data(
            &self,
            access_token: &AccessToken,
            date: NaiveDate,
        ) -> ApplicationResult<FitbitActivityData>;

        async fn get_sleep_data(
            &self,
            access_token: &AccessToken,
            date: NaiveDate,
        ) -> ApplicationResult<FitbitSleepData>;
    }
}

mock! {
    pub TokenStore {}

    #[async_trait::async_trait]
    impl TokenStorePort for TokenStore {
        async fn load(&self, source: &str) -> ApplicationResult<Option<FitbitTokenData>>;
        async fn save(&self, source: &str, token: &FitbitTokenData) -> ApplicationResult<()>;
        async fn set_last_sync_at(&self, source: &str, t: DateTime<Utc>) -> ApplicationResult<()>;
        async fn last_sync_at(&self, source: &str) -> ApplicationResult<Option<DateTime<Utc>>>;
    }
}

mock! {
    pub TagRepository {}

    #[async_trait::async_trait]
    impl TagQueryableStatisticRepository for TagRepository {
        async fn find_statistic_id_by_exact_tags(
            &self,
            tags: &TagSet,
        ) -> ApplicationResult<Option<StatisticId>>;
    }
}

mock! {
    pub StatRepository {}

    impl StatisticRepository for StatRepository {
        fn find_metadata_by_id(&self, id: StatisticId) -> Result<lifestats_core::statistic::any_statistic::AnyStatistic, RepositoryError>;
        fn find_all_metadata(&self) -> Result<Vec<lifestats_core::statistic::any_statistic::AnyStatistic>, RepositoryError>;
        fn create_metadata(&self, statistic: lifestats_core::statistic::any_statistic::AnyStatistic) -> Result<lifestats_core::statistic::any_statistic::AnyStatistic, RepositoryError>;
        fn update_metadata(&self, statistic: lifestats_core::statistic::any_statistic::AnyStatistic) -> Result<(), RepositoryError>;
        fn create_many_metadata(&self, statistics: Vec<lifestats_core::statistic::any_statistic::AnyStatistic>) -> Result<Vec<lifestats_core::statistic::any_statistic::AnyStatistic>, RepositoryError>;
        fn find_by_tags(&self, tags: &HashMap<String, String>) -> Result<Vec<lifestats_core::statistic::any_statistic::AnyStatistic>, RepositoryError>;
        fn find_log_value(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>, RepositoryError>;
        fn save_log_value(&self, id: StatisticId, date: NaiveDate, value: serde_json::Value) -> Result<(), RepositoryError>;
        fn log_integer_value(&self, id: StatisticId, date: NaiveDate, value: i64, policy: lifestats_core::statistic::value_objects::update_policies::amount::AmountUpdatePolicy) -> Result<bool, RepositoryError>;
        fn delete_log_value(&self, id: StatisticId, date: NaiveDate) -> Result<bool, RepositoryError>;
        fn find_log_values_in_range(&self, id: StatisticId, range: lifestats_core::shared::date_range::DateRange) -> Result<Vec<lifestats_core::shared::dated_value::DatedValue<serde_json::Value>>, RepositoryError>;
        fn find_goal(&self, id: StatisticId, date: NaiveDate) -> Result<lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult, RepositoryError>;
        fn save_goal(&self, id: StatisticId, date: NaiveDate, goal: Option<serde_json::Value>) -> Result<(), RepositoryError>;
        fn save_goals_in_range(&self, id: StatisticId, range: lifestats_core::shared::date_range::DateRange, goal: Option<serde_json::Value>) -> Result<(), RepositoryError>;
        fn delete_goal(&self, id: StatisticId, date: NaiveDate) -> Result<(), RepositoryError>;
        fn find_goals_in_range(&self, id: StatisticId, range: lifestats_core::shared::date_range::DateRange) -> Result<Vec<lifestats_core::shared::dated_value::DatedValue<lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult>>, RepositoryError>;
        fn find_default_goal(&self, id: StatisticId) -> Result<Option<serde_json::Value>, RepositoryError>;
        fn save_default_goal(&self, id: StatisticId, goal: serde_json::Value) -> Result<(), RepositoryError>;
        fn delete_default_goal(&self, id: StatisticId) -> Result<(), RepositoryError>;
        fn find_default_goal_timeline(&self, id: StatisticId) -> Result<lifestats_core::statistic::value_objects::goals::default_goal_timeline::DefaultGoalTimeline<serde_json::Value>, RepositoryError>;
        fn save_default_goal_timeline(&self, id: StatisticId, timeline: &lifestats_core::statistic::value_objects::goals::default_goal_timeline::DefaultGoalTimeline<serde_json::Value>) -> Result<(), RepositoryError>;
        fn delete_metadata(&self, id: StatisticId) -> Result<(), RepositoryError>;
    }
}

#[derive(Clone)]
struct MockClock {
    now: DateTime<Utc>,
}

impl Clock for MockClock {
    fn now_utc(&self) -> DateTime<Utc> {
        self.now
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_token(expires_in_seconds: i64) -> FitbitTokenData {
    FitbitTokenData {
        access_token: AccessToken::new("test_access_token".to_string()),
        refresh_token: RefreshToken::new("test_refresh_token".to_string()),
        expires_at: Utc::now() + ChronoDuration::seconds(expires_in_seconds),
        scopes: vec!["activity".to_string(), "sleep".to_string()],
    }
}

fn create_test_activity_data() -> FitbitActivityData {
    FitbitActivityData {
        steps: Some(10000),
        calories_out: Some(2000),
        active_minutes: Some(60),
        cardio_minutes: Some(20),
        peak_minutes: Some(10),
    }
}

fn create_test_sleep_data() -> FitbitSleepData {
    FitbitSleepData {
        total_duration_asleep: Some(ChronoDuration::hours(8)),
        sleep_efficiency: Some(95),
        // Fitbit returns times in user's local timezone (as NaiveDateTime)
        fell_asleep_at_local: Some(
            NaiveDate::from_ymd_opt(2025, 1, 1)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap()),
        ),
        woke_up_at_local: Some(
            NaiveDate::from_ymd_opt(2025, 1, 2)
                .unwrap()
                .and_time(NaiveTime::from_hms_opt(6, 0, 0).unwrap()),
        ),
        total_duration_awake: Some(ChronoDuration::minutes(30)),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_unauthorized_triggers_refresh_and_retry() {
    // Arrange
    let mut oauth_mock = MockFitbitOAuth::new();
    let mut data_mock = MockFitbitData::new();
    let mut token_store = MockTokenStore::new();
    let mut tag_repo = MockTagRepository::new();
    let mut stat_repo = MockStatRepository::new();

    let clock = Arc::new(MockClock { now: Utc::now() });

    let token = create_test_token(3600);
    let refreshed_token = create_test_token(3600);

    // Token store returns initial token
    token_store
        .expect_load()
        .times(1)
        .returning(move |_| Ok(Some(token.clone())));

    // Tag repository returns statistic ID for steps with canonical Fitbit tags
    let steps_metadata =
        mock_fitbit_statistic_metadata(1, FitbitMetric::Steps.canonical_tag_value());
    let expected_steps_tags = TagSet::new(
        steps_metadata
            .tags()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    let steps_stat_id = steps_metadata.id();
    tag_repo
        .expect_find_statistic_id_by_exact_tags()
        .times(1)
        .withf(move |tags| *tags == expected_steps_tags)
        .returning(move |_| Ok(Some(steps_stat_id)));

    // First API call returns 401 Unauthorized
    let unauthorized_call = StdMutex::new(false);
    data_mock
        .expect_get_activity_data()
        .times(2)
        .returning(move |_, _| {
            let mut called = unauthorized_call.lock().unwrap();
            if !*called {
                *called = true;
                Err(ApplicationError::FitbitSyncFailed(
                    FitbitUpstreamError::Unauthorized,
                ))
            } else {
                Ok(create_test_activity_data())
            }
        });

    // OAuth API expects refresh call
    oauth_mock
        .expect_refresh()
        .times(1)
        .returning(move |_| Ok(refreshed_token.clone()));

    // Token store expects save after refresh
    token_store.expect_save().times(1).returning(|_, _| Ok(()));

    // Stat repository expects save_log_value
    stat_repo
        .expect_save_log_value()
        .times(1)
        .returning(|_, _, _| Ok(()));

    // Token store expects set_last_sync_at
    token_store
        .expect_set_last_sync_at()
        .times(1)
        .returning(|_, _| Ok(()));

    // Build handler with mocks
    let handler = SyncFitbitDataCommandHandler::new(
        Arc::new(oauth_mock),
        Arc::new(data_mock),
        Arc::new(token_store),
        Arc::new(tag_repo),
        Arc::new(stat_repo),
        clock,
        chrono_tz::UTC,
        HashSet::from([FitbitMetric::Steps]),
    );

    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };

    // Act
    let result = handler.handle(command).await;

    // Assert
    assert!(
        result.is_ok(),
        "Sync should succeed after refresh: {:?}",
        result
    );
}

#[tokio::test]
async fn test_missing_scopes_returns_actionable_error() {
    // Arrange
    let oauth_mock = MockFitbitOAuth::new();
    let data_mock = MockFitbitData::new();
    let mut token_store = MockTokenStore::new();
    let tag_repo = MockTagRepository::new();
    let stat_repo = MockStatRepository::new();

    let clock = Arc::new(MockClock { now: Utc::now() });

    // Token with incomplete scopes (missing "sleep")
    let incomplete_token = FitbitTokenData {
        access_token: AccessToken::new("test".to_string()),
        refresh_token: RefreshToken::new("test".to_string()),
        expires_at: Utc::now() + ChronoDuration::hours(1),
        scopes: vec!["activity".to_string()], // Missing "sleep"
    };

    token_store
        .expect_load()
        .times(1)
        .returning(move |_| Ok(Some(incomplete_token.clone())));

    let handler = SyncFitbitDataCommandHandler::new(
        Arc::new(oauth_mock),
        Arc::new(data_mock),
        Arc::new(token_store),
        Arc::new(tag_repo),
        Arc::new(stat_repo),
        clock,
        chrono_tz::UTC,
        HashSet::from([FitbitMetric::Steps, FitbitMetric::SleepDuration]),
    );

    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };

    // Act
    let result = handler.handle(command).await;

    // Assert
    assert!(result.is_err());
    let error_msg = format!("{:?}", result.err().unwrap());
    assert!(
        error_msg.contains("missing required scopes") || error_msg.contains("sleep"),
        "Error should mention missing scopes: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_token_expiring_triggers_proactive_refresh() {
    // Arrange
    let mut oauth_mock = MockFitbitOAuth::new();
    let mut data_mock = MockFitbitData::new();
    let mut token_store = MockTokenStore::new();
    let mut tag_repo = MockTagRepository::new();
    let mut stat_repo = MockStatRepository::new();

    let now = Utc::now();
    let clock = Arc::new(MockClock { now });

    // Token expiring in 2 minutes (less than 5 minute default skew)
    let expiring_token = FitbitTokenData {
        access_token: AccessToken::new("expiring".to_string()),
        refresh_token: RefreshToken::new("refresh".to_string()),
        expires_at: now + ChronoDuration::minutes(2),
        scopes: vec!["activity".to_string()],
    };

    let refreshed_token = create_test_token(3600);

    token_store
        .expect_load()
        .times(1)
        .returning(move |_| Ok(Some(expiring_token.clone())));

    // Expect proactive refresh BEFORE any API calls
    oauth_mock
        .expect_refresh()
        .times(1)
        .returning(move |_| Ok(refreshed_token.clone()));

    // Expect token save after proactive refresh
    token_store.expect_save().times(1).returning(|_, _| Ok(()));

    let steps_metadata =
        mock_fitbit_statistic_metadata(1, FitbitMetric::Steps.canonical_tag_value());
    let expected_steps_tags = TagSet::new(
        steps_metadata
            .tags()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    let steps_stat_id = steps_metadata.id();
    tag_repo
        .expect_find_statistic_id_by_exact_tags()
        .times(1)
        .withf(move |tags| *tags == expected_steps_tags)
        .returning(move |_| Ok(Some(steps_stat_id)));

    data_mock
        .expect_get_activity_data()
        .times(1)
        .returning(|_, _| Ok(create_test_activity_data()));

    stat_repo
        .expect_save_log_value()
        .times(1)
        .returning(|_, _, _| Ok(()));

    token_store
        .expect_set_last_sync_at()
        .times(1)
        .returning(|_, _| Ok(()));

    let handler = SyncFitbitDataCommandHandler::new(
        Arc::new(oauth_mock),
        Arc::new(data_mock),
        Arc::new(token_store),
        Arc::new(tag_repo),
        Arc::new(stat_repo),
        clock,
        chrono_tz::UTC,
        HashSet::from([FitbitMetric::Steps]),
    );

    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };

    // Act
    let result = handler.handle(command).await;

    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_same_date_twice_is_idempotent() {
    // Arrange
    let oauth_mock = MockFitbitOAuth::new();
    let mut data_mock = MockFitbitData::new();
    let mut token_store = MockTokenStore::new();
    let mut tag_repo = MockTagRepository::new();
    let mut stat_repo = MockStatRepository::new();

    let clock = Arc::new(MockClock { now: Utc::now() });

    let token = create_test_token(3600);

    token_store
        .expect_load()
        .times(2)
        .returning(move |_| Ok(Some(token.clone())));

    let steps_metadata =
        mock_fitbit_statistic_metadata(1, FitbitMetric::Steps.canonical_tag_value());
    let expected_steps_tags = TagSet::new(
        steps_metadata
            .tags()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    let steps_stat_id = steps_metadata.id();
    tag_repo
        .expect_find_statistic_id_by_exact_tags()
        .times(2)
        .withf(move |tags| *tags == expected_steps_tags)
        .returning(move |_| Ok(Some(steps_stat_id)));

    // API should return same data both times
    data_mock
        .expect_get_activity_data()
        .times(2)
        .returning(|_, _| Ok(create_test_activity_data()));

    // Repository should receive same data twice (UPSERT behavior)
    stat_repo
        .expect_save_log_value()
        .times(2)
        .withf(|id, date, value| {
            *id == StatisticId(1)
                && *date == NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
                && value == &serde_json::json!(10000)
        })
        .returning(|_, _, _| Ok(()));

    token_store
        .expect_set_last_sync_at()
        .times(2)
        .returning(|_, _| Ok(()));

    let handler = SyncFitbitDataCommandHandler::new(
        Arc::new(oauth_mock),
        Arc::new(data_mock),
        Arc::new(token_store),
        Arc::new(tag_repo),
        Arc::new(stat_repo),
        clock,
        chrono_tz::UTC,
        HashSet::from([FitbitMetric::Steps]),
    );

    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };

    // Act - Run sync twice
    let result1 = handler.handle(command).await;
    let result2 = handler.handle(command).await;

    // Assert
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_validates_date_range() {
    // Arrange
    let oauth_mock = MockFitbitOAuth::new();
    let data_mock = MockFitbitData::new();
    let token_store = MockTokenStore::new();
    let tag_repo = MockTagRepository::new();
    let stat_repo = MockStatRepository::new();

    let clock = Arc::new(MockClock { now: Utc::now() });

    let handler = SyncFitbitDataCommandHandler::new(
        Arc::new(oauth_mock),
        Arc::new(data_mock),
        Arc::new(token_store),
        Arc::new(tag_repo),
        Arc::new(stat_repo),
        clock,
        chrono_tz::UTC,
        HashSet::from([FitbitMetric::Steps]),
    );

    // Test 1: since > until
    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };

    let result = handler.handle(command).await;
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains("before"));

    // Test 2: Range too large (> 31 days)
    let command = SyncFitbitDataCommand {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    };

    let result = handler.handle(command).await;
    assert!(result.is_err());
    let error_msg = format!("{:?}", result.err().unwrap());
    assert!(
        error_msg.contains("range") || error_msg.contains("31"),
        "Error should mention range or 31: {}",
        error_msg
    );
}
