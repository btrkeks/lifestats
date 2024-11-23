// fitbit/src/main.rs
use chrono::{Duration, Local};

mod access_token;
mod activity_summary;
mod request;
mod sleep;

use crate::sleep::SleepResponse;
use anyhow::{anyhow, Context, Result};
use backend::database::db_ops::open_db;
use backend::statistics::objects::StatisticId;
use backend::statistics::statistic_definition_storage::get_statistic_by_id;
use backend::statistics::type_definitions::{AnyStatistic, StatisticTypeSpecific};
use backend::statistics::update_policy::{AmountUpdatePolicy, FixedValueUpdatePolicy};

fn main() -> Result<()> {
    let today = Local::now().date_naive();
    let sleep_data = request::fetch_sleep_data(today).context("Fetching sleep data")?;

    let conn = open_db()?;

    let amount_slept_statistic = get_statistic_by_id(StatisticId(1))?.unwrap();
    if let AnyStatistic::TimeAmount(amount_slept_statistic) = amount_slept_statistic {
        let sleep_duration = sleep_data.get_total_duration_asleep();
        amount_slept_statistic.log_value(
            &conn,
            today,
            sleep_duration,
            AmountUpdatePolicy::Ignore,
        )?;
    }

    let wake_up_time_stat = get_statistic_by_id(StatisticId(22))?.unwrap();
    if let AnyStatistic::PointInTime(time_woken_up_stat) = wake_up_time_stat {
        let wake_up_time = sleep_data.get_wake_up_time().unwrap();
        time_woken_up_stat.log_value(&conn, today, wake_up_time, FixedValueUpdatePolicy::Ignore)?;
    }

    let fell_asleep_time_stat = get_statistic_by_id(StatisticId(21))?.unwrap();
    if let AnyStatistic::PointInTime(fell_asleep_time_stat) = fell_asleep_time_stat {
        let time_fell_asleep = sleep_data.get_time_fell_asleep().unwrap();
        fell_asleep_time_stat.log_value(
            &conn,
            today - Duration::days(1),
            time_fell_asleep,
            FixedValueUpdatePolicy::Ignore,
        )?;
    }

    let yesterday = (Local::now() - Duration::days(1)).date_naive();

    let time_went_to_bed_stat = get_statistic_by_id(StatisticId(20))?.unwrap();
    let time_went_to_bed =
        if let AnyStatistic::PointInTime(time_went_to_bed_stat) = time_went_to_bed_stat {
            if let Some(t) = time_went_to_bed_stat
                .get_row_for_date(&conn, yesterday)?
                .value
            {
                yesterday.and_time(t)
            } else {
                return Err(anyhow!("Time went to bed not logged yet"));
            }
        } else {
            return Err(anyhow!("ID 20 is not a PointOfTimeStatistic"));
        };

    let end_period_1 = time_went_to_bed + Duration::minutes(30);
    let end_period_2 = end_period_1 + Duration::hours(2);
    let end_period_3 = end_period_2 + Duration::hours(2);
    let end_period_4 = end_period_3 + Duration::hours(2);
    let end_period_5 = end_period_4 + Duration::minutes(60 + 50);

    println!(
        "Period 1: {:.1}m",
        sleep_data
            .get_time_awake_between(time_went_to_bed, end_period_1)
            .num_minutes()
    );
    println!(
        "Period 2: {:.1}m",
        sleep_data
            .get_time_awake_between(end_period_1, end_period_2)
            .num_minutes()
    );
    println!(
        "Period 3: {:.1}m",
        sleep_data
            .get_time_awake_between(end_period_2, end_period_3)
            .num_minutes()
    );
    println!(
        "Period 4: {:.1}m",
        sleep_data
            .get_time_awake_between(end_period_3, end_period_4)
            .num_minutes()
    );
    println!(
        "Period 5: {:.1}m",
        sleep_data
            .get_time_awake_between(end_period_4, end_period_5)
            .num_minutes()
    );

    Ok(())
}
