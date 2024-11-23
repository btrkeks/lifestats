// logging/cli_logger/src/main.rs
// CLI utility for manual logging of statistics data, supporting various data types
// and update policies for flexible data entry.

use backend::database::db_ops;
use backend::util::paths;

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local, NaiveDate};
use clap::Parser;
use env_logger::Builder;
use log::LevelFilter;
use std::fs::OpenOptions;
use std::io::Write;
use backend::statistics::objects::StatisticId;
use backend::statistics::statistic_definition_storage::get_statistic_by_id;
use backend::statistics::type_definitions::Statistic;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Id of the statistic")]
    id: u32,

    #[arg(long, help = "What to do in case of existing values")]
    update_policy: Option<String>,

    #[arg(
        short,
        default_value = "0",
        help = "Number of days in the past to log for"
    )]
    days_in_past: i64,

    #[arg(short, long, help = "Value to add")]
    value: Option<String>,

    #[arg(long, help = "Use current time as the value")]
    now: bool,
}

fn get_date_for_days_in_past(days_in_past: i64) -> NaiveDate {
    Local::now().date_naive() - Duration::days(days_in_past)
}

fn setup_logging() -> Result<()> {
    let log_path = paths::get_log_file()?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    Builder::new()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();

    Ok(())
}

fn main() -> Result<()> {
    setup_logging().context("Failed to set up logging")?;

    let args = Args::parse();

    let statistic = get_statistic_by_id(StatisticId(args.id))?
        .ok_or_else(|| anyhow!("Failed to get statistic with id {}", args.id))?;
    let date = get_date_for_days_in_past(args.days_in_past);
    let conn = db_ops::open_db().context("Failed to open database")?;

    let value = if args.now {
        Local::now().time().to_string()
    } else if let Some(val) = args.value {
        val
    } else {
        return Err(anyhow!(
            "You need to either provide a value to log or specify --now."
        ));
    };

    statistic.log_from_string(&conn, date, &value, args.update_policy.as_deref())
}
