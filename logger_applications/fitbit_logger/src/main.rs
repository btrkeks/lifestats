// fitbit/src/main.rs
use chrono::{Duration, Local};

mod access_token;
mod activity_summary;
mod config;
mod error;
mod fitbit_client;
mod response_cache;
mod sleep;

use crate::config::{FitbitLoggingConfig, FitbitMetric};
use crate::fitbit_client::FitbitClient;
use crate::sleep::SleepResponse;
use anyhow::{Context, Result};
use clap::Parser;
use lifestats_core::app_context::AppContext;
use transfer_objects::statistic_id::StatisticId;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Number of days in the past to fetch data for (0 = today)
    #[arg(short = 'd', long = "days", default_value = "0")]
    days_ago: i64,
}

// TODO: Implement command line arguments

fn run(
    app_context: AppContext,
    fitbit_client: FitbitClient,
    config: FitbitLoggingConfig,
    args: Args,
) -> Result<()> {
    let mut response_cache = response_cache::FitbitResponseCache::new(fitbit_client);

    let target_date = (Local::now() - Duration::days(args.days_ago)).date_naive();
    let previous_date = target_date - Duration::days(1);

    for (metric, id) in &config.metric_map {
        let (day, log_string) = match metric {
            FitbitMetric::SleepDuration => {
                let sleep_data = response_cache.get_sleep_response(target_date)?;
                let sleep_duration = sleep_data.get_total_duration_asleep();
                (target_date, format!("{}s", sleep_duration.num_seconds()))
            }
            FitbitMetric::WakeUpTime => {
                let sleep_data = response_cache.get_sleep_response(target_date)?;
                let wake_up_time = sleep_data.get_wake_up_time().unwrap();
                (target_date, wake_up_time.format("%H:%M:%S").to_string())
            }
            FitbitMetric::FellAsleepTime => {
                let sleep_data = response_cache.get_sleep_response(target_date)?;
                let time_fell_asleep = sleep_data.get_time_fell_asleep().unwrap();
                (
                    previous_date,
                    time_fell_asleep.format("%H:%M:%S").to_string(),
                )
            }
            FitbitMetric::SleepEfficiency => {
                let sleep_data = response_cache.get_sleep_response(target_date)?;
                let sleep_efficiency = sleep_data.get_sleep_efficiency().unwrap();
                (target_date, format!("{}%", sleep_efficiency))
            }
            FitbitMetric::Steps => {
                let activity_summary = response_cache.get_activity_summary_response(target_date)?;
                let steps = activity_summary.get_steps();
                (target_date, steps.to_string())
            }
            FitbitMetric::CaloriesOut => unimplemented!(),
            FitbitMetric::ActiveMinutes => unimplemented!(),
            FitbitMetric::CardioMinutes => unimplemented!(),
            FitbitMetric::PeakMinutes => unimplemented!(),
            FitbitMetric::TotalDurationAwake => {
                let sleep_data = response_cache.get_sleep_response(target_date)?;
                let total_duration_awake =
                    sleep_data.get_total_duration_awake_during_sleep().unwrap();
                (
                    target_date,
                    format!("{}s", total_duration_awake.num_seconds()),
                )
            }
        };

        lifestats_core::log::log_statistic(&app_context, *id, day, &log_string, Some("ignore"))?;
    }

    Ok(())
}

fn get_fitbit_logging_config() -> FitbitLoggingConfig {
    // TODO: This should be configurable somehow
    let mut config = FitbitLoggingConfig::new();
    config.add_mapping(FitbitMetric::SleepDuration, StatisticId(1));
    config.add_mapping(FitbitMetric::WakeUpTime, StatisticId(22));
    config.add_mapping(FitbitMetric::FellAsleepTime, StatisticId(21));
    config
}

fn main() -> Result<()> {
    let args = Args::parse();
    let app_context = lifestats_core::app_context::build_application()?;
    let config = get_fitbit_logging_config();
    let access_token = access_token::get_access_token()?;
    let fitbit_client = FitbitClient::new(access_token);

    run(app_context, fitbit_client, config, args)
}
