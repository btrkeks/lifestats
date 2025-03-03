use crate::activity_summary::ActivitySummaryResponse;
use crate::error::FitbitError;
use crate::sleep::SleepResponseV1_2;
use chrono::NaiveDate;

const API_BASE_URL: &str = "https://api.fitbit.com";

#[cfg_attr(test, mockall::automock)]
pub trait FitbitClientTrait {
    fn fetch_sleep_data(&self, date: NaiveDate) -> Result<SleepResponseV1_2, FitbitError>;
    fn fetch_activity_summary(
        &self,
        date: NaiveDate,
    ) -> Result<ActivitySummaryResponse, FitbitError>;
}

pub struct FitbitClient {
    access_token: String,
}

impl FitbitClient {
    pub fn new(access_token: String) -> Self {
        Self { access_token }
    }

    fn make_api_request<T>(&self, url: &str) -> Result<T, FitbitError>
    where
        T: serde::de::DeserializeOwned,
    {
        ureq::get(url)
            .header("Authorization", &format!("Bearer {}", self.access_token))
            .call()
            .map_err(FitbitError::RequestError)?
            .body_mut()
            .read_json()
            .map_err(|e| FitbitError::JsonError(e.to_string()))
    }
}

impl FitbitClientTrait for FitbitClient {
    fn fetch_sleep_data(&self, date: NaiveDate) -> Result<SleepResponseV1_2, FitbitError> {
        let url = format!(
            "{}/1.2/user/-/sleep/date/{}.json",
            API_BASE_URL,
            date.format("%Y-%m-%d")
        );

        self.make_api_request(&url)
    }

    fn fetch_activity_summary(
        &self,
        date: NaiveDate,
    ) -> Result<ActivitySummaryResponse, FitbitError> {
        let url = format!(
            "{}/1/user/-/activities/date/{}.json",
            API_BASE_URL,
            date.format("%Y-%m-%d")
        );

        self.make_api_request(&url)
    }
}
