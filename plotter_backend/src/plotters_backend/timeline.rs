use crate::common::seconds_to_time_formatter;
use crate::plot_data::{LabeledTime, LabeledTimeFrame, TimelineData};
use crate::plotters_backend::util::{draw_gray_dashed_line, get_bar};
use anyhow::Result;
use backend::paths::get_plot_path;
use chrono::{Duration, Local};
use chrono::{NaiveTime, Timelike};
use plotters::prelude::*;
use std::collections::HashMap;

fn create_color_map(
    labeled_time_frames: &Vec<LabeledTimeFrame>,
    labeled_time_points: &Vec<LabeledTime>,
) -> HashMap<String, RGBAColor> {
    let mut hmap = HashMap::new();
    let mut index = 0_usize;

    for frame in labeled_time_frames {
        if !hmap.contains_key(&frame.label) {
            let color = Palette99::pick(index).mix(0.9);
            hmap.insert(frame.label.clone(), color);
            index += 1;
        }
    }

    for point in labeled_time_points {
        if !hmap.contains_key(&point.label) {
            let color = Palette99::pick(index).mix(0.9);
            hmap.insert(point.label.clone(), color);
            index += 1;
        }
    }

    hmap
}

pub fn plot_timeline(filename: &str, title: &str, data: TimelineData) -> Result<()> {
    const PLOT_HEIGHT: u32 = 1050;
    let plot_path = get_plot_path()?.join(filename);
    let root = BitMapBackend::new(&plot_path, (700, PLOT_HEIGHT)).into_drawing_area();
    root.fill(&WHITE)?;

    let min_time: i64 = NaiveTime::from_hms_opt(6, 0, 0)
        .unwrap()
        .num_seconds_from_midnight() as i64;
    let max_time: i64 = NaiveTime::from_hms_opt(22, 0, 0)
        .unwrap()
        .num_seconds_from_midnight() as i64;

    let mut cc = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .margin(10)
        .caption(title, ("IPAMincho", 40))
        .build_cartesian_2d(
            0f32..1f32,
            (min_time..max_time).with_key_points((0..=32).map(|i| min_time + i * 1800).collect()),
        )?;

    cc.configure_mesh()
        .disable_mesh()
        .y_label_style(("IPAMincho", 20).into_font())
        .y_label_formatter(&seconds_to_time_formatter)
        .draw()?;

    let color_map = create_color_map(&data.time_frames, &data.time_points);

    let mut grouped_frames: HashMap<String, Vec<&LabeledTimeFrame>> = HashMap::new();
    for frame in &data.time_frames {
        grouped_frames
            .entry(frame.label.clone())
            .or_default()
            .push(frame);
    }

    for (label, frames) in grouped_frames {
        let color = *color_map.get(&label).unwrap();

        cc.draw_series(frames.iter().map(|lframe| {
            let start_sec = lframe.time_frame.start.num_seconds_from_midnight() as i64;
            let end_sec = lframe.time_frame.end.num_seconds_from_midnight() as i64;

            Rectangle::new([(0f32, start_sec), (1f32, end_sec)], color.filled())
        }))?
        .label(label)
        .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 10, y + 5)], color.filled()));
    }

    for time_point in data.time_points.iter() {
        let color = *color_map.get(&time_point.label).unwrap();
        let seconds = time_point.time.num_seconds_from_midnight() as i64;

        cc.draw_series(std::iter::once(PathElement::new(
            [(0f32, seconds), (1f32, seconds)],
            color.stroke_width(2),
        )))?
        .label(&time_point.label)
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 15, y)], color.clone()));
    }

    // anki
    draw_gray_dashed_line(&mut cc, 24300)?;
    draw_gray_dashed_line(&mut cc, 28800)?;

    // piano
    draw_gray_dashed_line(&mut cc, 30600)?;
    draw_gray_dashed_line(&mut cc, 36000)?;

    cc.configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .label_font(("IPAMincho", 20).into_font())
        .draw()?;

    Ok(())
}
