// use chrono::Duration;
// use backend::db_ops::open_db;
// use backend::statistic_getter::get_statistic_by_id;
// use backend::statistics::{AnyStatistic};
// use anyhow::{Result};
// use backend::util::format_duration;
//
// fn get_total_piano() -> Result<Duration> {
//     let statistic = get_statistic_by_id(5)?;
//
//     let statistic = match statistic {
//         AnyStatistic::TimeSpans(s) => {
//             s
//         }
//         _ => {
//             panic!("Not a TimeSpansStatistics");
//         }
//     };
//
//     let conn = open_db()?;
//     statistic.get_total_time_this_month(&conn)
// }
//
// fn main() -> Result<()> {
//     println!("Total piano: {}", format_duration(get_total_piano()?));
//     Ok(())
// }

fn main() {}
