//! Generates the demo images embedded in the repository README.
//!
//! Run with: `cargo run -p lifestats-plotting --example readme_images`
//! Images are written to `docs/images/` (override with the first CLI argument).
//! All data is synthetic and deterministic.

use chrono::{Duration, NaiveDate, NaiveTime};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
use lifestats_plotting::{plot_timeline_events, PlottersRenderer, TimelineEventData};
use plot_domain::plot::axis::YAxis;
use plot_domain::plot::data::{DataPoint, DrawableData, DrawableGoal, PlotData, ScalarValue};
use plot_domain::plot::plot_type::PlotType;
use plot_domain::plot::renderer::{PlotRenderer, RenderConfig};

const GOLD: [u8; 3] = [255, 215, 0];
const GREEN: [u8; 3] = [169, 252, 177];
const ORANGE: [u8; 3] = [255, 174, 113];
const RED: [u8; 3] = [245, 169, 142];

/// Same thresholds as `percentage_to_color` in the application layer.
fn achievement_color(value: i64, goal: i64) -> [u8; 3] {
    let percentage = value * 100 / goal;
    if percentage >= 150 {
        GOLD
    } else if percentage >= 100 {
        GREEN
    } else if percentage >= 90 {
        ORANGE
    } else {
        RED
    }
}

/// Tiny deterministic PRNG so the images are reproducible.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn range(&mut self, min: i64, max: i64) -> i64 {
        min + (self.next() % (max - min + 1) as u64) as i64
    }
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn time(h: u32, min: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, min, 0).unwrap()
}

fn habit_bar_chart() -> anyhow::Result<PlotData> {
    let start = date(2026, 6, 1);
    let end = date(2026, 6, 30);
    let goal = 50;
    let mut rng = Xorshift(0x5eed);

    let mut points = Vec::new();
    let mut day = start;
    while day <= end {
        let value = match rng.next() % 10 {
            0 => rng.range(20, 40),     // a bad day
            1 | 2 => rng.range(45, 49), // just short of the goal
            3 => rng.range(75, 90),     // crushed it
            _ => rng.range(50, 70),     // goal achieved
        };
        points.push(DataPoint::new(
            day,
            Some(DrawableData::Scalar(ScalarValue::number(value))),
            achievement_color(value, goal),
            Some(DrawableGoal::Scalar(ScalarValue::number(goal))),
        ));
        day += Duration::days(1);
    }

    let y_axis = YAxis::from_data_points(&points, PlotType::BarChart)?;
    Ok(PlotData::new(
        "Pushups".to_string(),
        points,
        DateRange::new(start, end)?,
        PlotType::BarChart,
        y_axis,
    ))
}

fn reading_time_frames() -> anyhow::Result<PlotData> {
    let start = date(2026, 6, 16);
    let end = date(2026, 6, 30);
    let goal_minutes = 60;
    let mut rng = Xorshift(0xb00c);

    let mut goal_frames = TimeFrameList::default();
    goal_frames.append_frame(TimeFrame::new(time(20, 0), time(21, 30))?)?;

    let mut points = Vec::new();
    let mut day = start;
    while day <= end {
        // One or two evening reading sessions, occasionally skipped.
        let value = if rng.next() % 7 == 0 {
            None
        } else {
            let mut frames = TimeFrameList::default();
            let session_start = 19 * 60 + rng.range(30, 80);
            let first_len = rng.range(20, 55);
            frames.append_frame(TimeFrame::new(
                time((session_start / 60) as u32, (session_start % 60) as u32),
                time(
                    ((session_start + first_len) / 60) as u32,
                    ((session_start + first_len) % 60) as u32,
                ),
            )?)?;
            if rng.next() % 2 == 0 {
                let second_start = session_start + first_len + rng.range(10, 25);
                let second_len = rng.range(15, 45);
                frames.append_frame(TimeFrame::new(
                    time((second_start / 60) as u32, (second_start % 60) as u32),
                    time(
                        ((second_start + second_len) / 60) as u32,
                        ((second_start + second_len) % 60) as u32,
                    ),
                )?)?;
            }
            Some(frames)
        };

        let color = match &value {
            Some(frames) => {
                let minutes = frames.get_total_duration().num_minutes();
                achievement_color(minutes, goal_minutes)
            }
            None => RED,
        };

        points.push(DataPoint::new(
            day,
            value.map(DrawableData::TimeFrames),
            color,
            Some(DrawableGoal::TimeFrames(goal_frames.clone())),
        ));
        day += Duration::days(1);
    }

    let y_axis = YAxis::from_data_points(&points, PlotType::TimeFrames)?;
    Ok(PlotData::new(
        "Evening Reading".to_string(),
        points,
        DateRange::new(start, end)?,
        PlotType::TimeFrames,
        y_axis,
    ))
}

fn daily_timeline() -> Vec<TimelineEventData> {
    let event = |label: &str, from: (u32, u32), to: (u32, u32)| TimelineEventData {
        timeframe: TimeFrame::new(time(from.0, from.1), time(to.0, to.1)).unwrap(),
        label: label.to_string(),
    };

    vec![
        event("Sleep", (0, 0), (6, 45)),
        event("Anki Review", (7, 10), (7, 50)),
        event("Deep Work", (8, 30), (10, 15)),
        event("Deep Work", (10, 45), (12, 10)),
        event("Piano Practice", (13, 0), (13, 40)),
        event("Deep Work", (14, 15), (16, 30)),
        event("Exercise", (17, 30), (18, 20)),
        event("Reading", (20, 5), (21, 25)),
        event("Sleep", (22, 30), (23, 59)),
    ]
}

fn main() -> anyhow::Result<()> {
    let output_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/images".to_string());
    std::fs::create_dir_all(&output_dir)?;

    let renderer = PlottersRenderer::new();
    let config = RenderConfig::new(900, 450);

    let habit = habit_bar_chart()?;
    std::fs::write(
        format!("{output_dir}/habit-bar-chart.png"),
        renderer.render(&habit, &config)?,
    )?;

    let reading = reading_time_frames()?;
    std::fs::write(
        format!("{output_dir}/time-frames.png"),
        renderer.render(&reading, &config)?,
    )?;

    let timeline = plot_timeline_events(
        daily_timeline(),
        date(2026, 6, 30),
        RenderConfig::new(560, 700),
    )?;
    std::fs::write(format!("{output_dir}/daily-timeline.png"), timeline)?;

    println!("Wrote demo images to {output_dir}/");
    Ok(())
}
