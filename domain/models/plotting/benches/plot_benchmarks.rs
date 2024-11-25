use std::path::PathBuf;
use criterion::{criterion_group, criterion_main, Criterion};
use chrono::{Duration, NaiveDate, NaiveTime};
use rand::Rng;
use backend::statistics::objects::{TimeFrame, TimeFrameList};
use plotting_backend::plotters_backend::{plot_barplot, plot_time_frames};
use anyhow::Result;
use std::fs;
use plotting_backend::plots::plot_data::{
    BarplotData, BarplotDataPoint, TimeFramesPlotData, TimeFramesPlotPoint,
    YValueFormatting
};
use plotting_backend::util::color_consts::{GREEN, ORANGE, RED};

fn generate_barplot_data(num_points: usize) -> BarplotData {
    let mut rng = rand::thread_rng();
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let data_points: Vec<BarplotDataPoint> = (0..num_points)
        .map(|i| {
            let date = start_date + Duration::days(i as i64);
            let value = rng.gen_range(0..3600); // 1 hour max
            let goal = if rng.gen_bool(0.8) {
                Some(rng.gen_range(value-300..value+300))
            } else {
                None
            };

            let color = if let Some(g) = goal {
                if value >= g {
                    GREEN
                } else if value >= g * 9 / 10 {
                    ORANGE
                } else {
                    RED
                }
            } else {
                GREEN
            };

            BarplotDataPoint {
                date,
                value,
                goal,
                color,
            }
        })
        .collect();

    BarplotData {
        y_value_formatting: YValueFormatting::SecondsToTime,
        border_color: GREEN,
        data_points,
    }
}

fn generate_timeframes_data(num_points: usize) -> TimeFramesPlotData {
    let mut rng = rand::thread_rng();
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    let data_points: Vec<TimeFramesPlotPoint> = (0..num_points)
        .map(|i| {
            let date = start_date + Duration::days(i as i64);
            let num_frames = rng.gen_range(1..=4);

            let time_frames = TimeFrameList::new_from_vec(
                (0..num_frames)
                    .map(|_| {
                        let start_hour = rng.gen_range(6..20);
                        let duration = Duration::hours(rng.gen_range(1..4));
                        let start = NaiveTime::from_hms_opt(start_hour, 0, 0).unwrap();
                        let end = start + duration;
                        TimeFrame::new(start, end).unwrap()
                    })
                    .collect()
            );

            TimeFramesPlotPoint {
                date,
                time_frames: time_frames.clone(),
                goal_time_frames: time_frames,
                color: GREEN,
            }
        })
        .collect();

    TimeFramesPlotData {
        border_color: GREEN,
        data_points,
    }
}

fn setup_benchmark_dir() -> Result<PathBuf> {
    let bench_dir = std::env::temp_dir().join("plot_benchmarks");
    if bench_dir.exists() {
        fs::remove_dir_all(&bench_dir)?;
    }
    fs::create_dir_all(&bench_dir)?;
    Ok(bench_dir)
}

fn bench_plotting(c: &mut Criterion) {
    // Create benchmark directory at the start
    let bench_dir = setup_benchmark_dir().expect("Failed to create benchmark directory");

    let mut group = c.benchmark_group("plotting");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(10));
    
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    for plot_period in [7, 14, 30].iter() {
        // Ensure we create fresh data and paths for each iteration
        let bar_path = bench_dir.join(format!("bench_bar_{}.png", plot_period));
        let barplot_data = generate_barplot_data(*plot_period);
        
        group.bench_function(
            format!("barplot_{}_days", plot_period),
            |b| {
                b.iter(|| {
                    plot_barplot(
                        &bar_path,
                        "Benchmark Bar",
                        start_date,
                        start_date + Duration::days(*plot_period as i64),
                        &barplot_data,
                    ).expect("Failed to create barplot")
                })
            }
        );

        let tf_path = bench_dir.join(format!("bench_timeframes_{}.png", plot_period));
        let timeframes_data = generate_timeframes_data(*plot_period);
        
        group.bench_function(
            format!("timeframes_{}_days", plot_period),
            |b| {
                b.iter(|| {
                    plot_time_frames(
                        &tf_path,
                        "Benchmark TimeFrames",
                        start_date,
                        start_date + Duration::days(*plot_period as i64),
                        &timeframes_data,
                    ).expect("Failed to create timeframes plot")
                })
            }
        );
    }

    group.finish();
}

criterion_group!(benches, bench_plotting);
criterion_main!(benches);