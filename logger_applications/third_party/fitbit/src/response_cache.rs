use crate::activity_summary::ActivitySummaryResponse;
use crate::fitbit_client::{FitbitClient, FitbitClientTrait};
use crate::sleep::SleepResponseV1_2;
use anyhow::Result;

pub struct FitbitResponseCache<C: FitbitClientTrait> {
    fitbit_client: C,
    sleep_response: Option<SleepResponseV1_2>,
    activity_summary_response: Option<ActivitySummaryResponse>,
}

impl<C: FitbitClientTrait> FitbitResponseCache<C> {
    pub fn new(fitbit_client: C) -> Self {
        Self {
            fitbit_client,
            sleep_response: None,
            activity_summary_response: None,
        }
    }

    pub fn get_sleep_response(&mut self, date: chrono::NaiveDate) -> Result<&SleepResponseV1_2> {
        if self.sleep_response.is_none() {
            self.sleep_response = Some(self.fitbit_client.fetch_sleep_data(date)?);
        }

        Ok(self.sleep_response.as_ref().unwrap())
    }

    pub fn get_activity_summary_response(
        &mut self,
        date: chrono::NaiveDate,
    ) -> Result<&ActivitySummaryResponse> {
        if self.activity_summary_response.is_none() {
            self.activity_summary_response = Some(self.fitbit_client.fetch_activity_summary(date)?);
        }

        Ok(self.activity_summary_response.as_ref().unwrap())
    }
}

#[cfg(test)]
mod response_cache_tests {
    use super::*;
    use crate::fitbit_client::MockFitbitClientTrait;
    use chrono::NaiveDate;
    use mockall::predicate::*;

    #[test]
    fn test_cache_behavior() -> Result<()> {
        let mut mock_client = MockFitbitClientTrait::new();
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

        // Setup mock expectations - sleep data should only be called once
        mock_client
            .expect_fetch_sleep_data()
            .with(eq(date))
            .times(1)
            .returning(|_| Ok(create_mock_sleep_response()));

        let mut cache = FitbitResponseCache::new(mock_client);

        // First call should fetch from API
        let _response1 = cache.get_sleep_response(date)?;
        // Second call should use cached data
        let _response2 = cache.get_sleep_response(date)?;

        Ok(())
    }

    fn create_mock_sleep_response() -> SleepResponseV1_2 {
        SleepResponseV1_2::default()
    }
}
