pub mod paths;
pub mod string_parsers;

use chrono::{Datelike, Local, NaiveDate, NaiveTime, Timelike};

pub fn get_date_today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn get_time_now() -> NaiveTime {
    Local::now().time()
}

pub fn get_minutes_since_midnight() -> i32 {
    let now = get_time_now();
    (now.hour() * 60 + now.minute()) as i32
}

pub fn get_num_of_days_for_closed_date_interval(start_date: NaiveDate, end_date: NaiveDate) -> i64 {
    (end_date - start_date).num_days() + 1
}

pub fn get_current_month_range() -> (NaiveDate, NaiveDate) {
    let now = Local::now().date_naive();
    let year = now.year();
    let month = now.month();

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid date");

    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
        .expect("Invalid date")
        .pred_opt()
        .expect("Invalid date");

    (first_day, last_day)
}