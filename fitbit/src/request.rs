use crate::access_token::get_access_token;
use crate::activity_summary::ActivitySummaryResponse;
use crate::sleep::SleepResponseV1_2;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use reqwest::blocking::{Client, Response};

pub fn fetch_fitbit_data(url: String) -> Result<Response> {
    let access_token = get_access_token()?;
    Client::new()
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .context("Sending request for fetching fitbit data")
}

pub(crate) fn fetch_sleep_data(date: NaiveDate) -> Result<SleepResponseV1_2> {
    let url = format!(
        "https://api.fitbit.com/1.2/user/-/sleep/date/{}.json",
        date.format("%Y-%m-%d")
    );

    fetch_fitbit_data(url)
        .context(format!("Fetching fitbit sleep data for date {}", date))?
        .json()
        .context("Parsing fitbit sleep response")
}

fn fetch_activity_summary(date: NaiveDate) -> Result<ActivitySummaryResponse> {
    let url = format!(
        "https://api.fitbit.com/1/user/-/activities/date/{}.json",
        date.format("%Y-%m-%d")
    );

    fetch_fitbit_data(url)
        .context(format!(
            "Fetching fitbit activity summary for date {}",
            date
        ))?
        .json()
        .context("Parsing fitbit activity summary response")
}
