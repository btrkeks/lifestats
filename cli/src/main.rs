// cli/src/main.

use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use backend::database::db_ops::open_db;
use backend::statistics::objects::StatisticId;
use backend::statistics::statistic_definition_storage::get_all_statistics;
use backend::statistics::type_definitions::{AnyStatistic, StatisticTypeSpecific};
use backend::statistics::type_definitions::Statistic;

fn get_date_for_days_in_past(days_in_past: i64) -> NaiveDate {
    Local::now().date_naive() - Duration::days(days_in_past)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display scores for statistics
    Score {
        #[arg(
            short,
            default_value = "0",
            help = "Number of days in the past to log for"
        )]
        days_in_past: i64,
    },
    /// Dump database data in JSON format
    Dump {
        #[arg(
            short,
            default_value = "0",
            help = "Number of days in the past to dump data for"
        )]
        days_in_past: i64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Score { days_in_past } => score_command(*days_in_past),
        Commands::Dump { days_in_past } => dump_command(*days_in_past),
    }
}

fn score_command(days_in_past: i64) -> Result<()> {
    let date = get_date_for_days_in_past(days_in_past);
    let statistics = get_all_statistics()?;

    let conn = open_db()?;

    let mut percs_with_names = statistics
        .iter()
        .filter_map(|stat| {
            stat.get_score(&conn, date)
                .unwrap()
                .map(|x| (x, stat.get_name()))
        })
        .collect::<Vec<(f64, &str)>>();

    percs_with_names
        .sort_unstable_by(|(perc1, _name1), (perc2, _name2)| perc2.partial_cmp(perc1).unwrap());

    percs_with_names
        .iter()
        .for_each(|(perc, name)| println!("{:03.02}%\t{}", *perc * 100.0, name));

    let sum_of_percentages = percs_with_names
        .iter()
        .map(|(perc, _name)| *perc)
        .sum::<f64>();
    let total_score = if sum_of_percentages > 0.0 {
        sum_of_percentages / percs_with_names.len() as f64
    } else {
        0.0
    };
    println!("--------------------------------------");
    println!("Total score for {}: {:.02}%", date, total_score * 100.0);

    Ok(())
}

#[derive(Serialize)]
struct StatisticDump {
    id: StatisticId,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    goal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
}

fn dump_command(days_in_past: i64) -> Result<()> {
    let date = get_date_for_days_in_past(days_in_past);
    let statistics = get_all_statistics()?;
    let conn = open_db()?;

    let mut dump_data = Vec::new();

    for stat in statistics {
        let stat_dump = match &stat {
            AnyStatistic::IntegerAmount(s) => dump_statistic(s, &conn, date)?,
            AnyStatistic::TimeAmount(s) => dump_statistic(s, &conn, date)?,
            AnyStatistic::PointInTime(s) => dump_statistic(s, &conn, date)?,
            AnyStatistic::TimeSpans(s) => dump_statistic(s, &conn, date)?,
            // AnyStatistic::TimeSpan(s) => dump_statistic(s, &conn, date)?,
            AnyStatistic::Habit(s) => dump_statistic(s, &conn, date)?,
        };

        dump_data.push(stat_dump);
    }

    let json_output = json!({
        "date": date.to_string(),
        "statistics": dump_data,
    });

    println!("{}", serde_json::to_string_pretty(&json_output)?);

    Ok(())
}

fn dump_statistic<T, U>(
    stat: &impl StatisticTypeSpecific<RetrieveValue = T, LogValue = U>,
    conn: &rusqlite::Connection,
    date: NaiveDate,
) -> Result<StatisticDump>
where
    T: backend::database::db_ops::DbStorable,
{
    let row = stat.get_row_for_date(conn, date)?;
    let score = stat.calculate_score(&row);

    Ok(StatisticDump {
        id: stat.get_id(),
        name: stat.get_name().to_string(),
        value: row.value.map(|v| v.to_db_string()),
        goal: row.goal.map(|g| g.to_db_string()),
        score,
    })
}
