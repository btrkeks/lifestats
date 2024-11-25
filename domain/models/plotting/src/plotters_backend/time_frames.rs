// use crate::util::color_consts::{BACKGROUND_COLOR, GOAL_LINE_COLOR};
// use crate::util::axis_label_formatter::{seconds_to_time_formatter};
// use crate::plot::plot_data::{TimeFramesPlotData, TimeFramesPlotPoint};
// use crate::plotters_backend::common::{get_border, get_vertical_line, get_bar};
// use anyhow::Result;
// use chrono::{Duration, NaiveDate, Timelike};
// use plotters::coord::combinators::WithKeyPoints;
// use plotters::coord::ranged1d::SegmentedCoord;
// use plotters::coord::types::RangedCoordi64;
// use plotters::prelude::*;
// use crate::util::tick_calculation::calculate_seconds_ticks;
//
// const PLOT_HEIGHT: u32 = 400;
// const PLOT_WIDTH: u32 = 450;
// const Y_MARGIN_PERC: i64 = 5; // 10 minutes. TODO: Should adapt to scale
//
// // ----------------------------------------------------------------------------------------------
//
// fn draw_goal_regions(
//     chart: &mut ChartContext<
//         BitMapBackend,
//         Cartesian2d<SegmentedCoord<RangedDate<NaiveDate>>, WithKeyPoints<RangedCoordi64>>,
//     >,
//     points: &[TimeFramesPlotPoint],
//     y_axis_max: i64,
// ) -> Result<()> {
//     for (date, goal_time_frames) in points.iter().map(|p| (p.date, &p.goal_time_frames)) {
//         if goal_time_frames.is_empty() {
//             continue;
//         }
//
//         let mut last_end_time = 0;
//
//         for frame in goal_time_frames {
//             let current_start_time = frame.start.num_seconds_from_midnight() as i64;
//             let current_end_time = frame.end.num_seconds_from_midnight() as i64;
//
//             chart.draw_series([get_shaded_area(date, last_end_time, current_start_time)])?;
//
//             chart.draw_series([
//                 get_vertical_line(date, current_start_time, GOAL_LINE_COLOR.mix(0.7)),
//                 get_vertical_line(date, current_end_time, GOAL_LINE_COLOR.mix(0.7)),
//             ])?;
//
//             last_end_time = current_end_time;
//         }
//
//         chart.draw_series([get_shaded_area(date, last_end_time, y_axis_max)])?;
//     }
//     Ok(())
// }
//
// fn get_shaded_area(
//     date: NaiveDate,
//     y_0: i64,
//     y_1: i64,
// ) -> Rectangle<(SegmentValue<NaiveDate>, i64)> {
//     const SHADING_COLOR: RGBAColor = RGBAColor(200, 200, 200, 0.1);
//
//     Rectangle::new(
//         [
//             (SegmentValue::Exact(date), y_0),
//             (SegmentValue::Exact(date + Duration::days(1)), y_1),
//         ],
//         SHADING_COLOR.filled(),
//     )
// }
//
// fn get_y_range(data: &[TimeFramesPlotPoint]) -> (i64, i64) {
//     let y_max = data.iter()
//         .flat_map(|point| [&point.time_frames, &point.goal_time_frames])
//         .filter_map(|tf_list|
//             tf_list.get_last_end_time()
//                 .map(|time| time.num_seconds_from_midnight() as i64)
//         )
//         .max()
//         .unwrap_or(0);
//
//     let y_min = data.iter()
//         .flat_map(|point| [&point.time_frames, &point.goal_time_frames])
//         .filter_map(|tf_list|
//             tf_list.get_first_start_time()
//                 .map(|time| time.num_seconds_from_midnight() as i64)
//         )
//         .min()
//         .unwrap_or(0);
//
//     (y_min, y_max)
// }
//
// pub fn plot_time_frames(
//     save_path: &std::path::Path,
//     title: &str,
//     date_start: NaiveDate,
//     date_end: NaiveDate,
//     data: &TimeFramesPlotData,
// ) -> Result<()> {
//     let root = BitMapBackend::new(save_path, (PLOT_WIDTH, PLOT_HEIGHT)).into_drawing_area();
//     root.fill(BACKGROUND_COLOR)?;
//
//     let (y_min, y_max) = get_y_range(&data.data_points);
//
//     let y_margin = Y_MARGIN_PERC * (y_max - y_min) / 100;
//     let y_axis_min = y_min - y_margin;
//     let y_axis_max = y_max + y_margin;
//
//     let mut chart = ChartBuilder::on(&root)
//         .x_label_area_size(25)
//         .y_label_area_size(52)
//         .margin_right(7)
//         .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
//         .build_cartesian_2d(
//             (date_start..date_end).into_segmented(),
//             (y_axis_min..y_axis_max).with_key_points(calculate_seconds_ticks(
//                 y_axis_min,
//                 y_axis_max,
//                 PLOT_HEIGHT,
//             )),
//         )?;
//
//     chart
//         .configure_mesh()
//         .x_labels(data.data_points.len() / 2)
//         .disable_x_mesh()
//         .bold_line_style(WHITE.mix(0.2))
//         .x_label_formatter(&|seg| match seg {
//             SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
//             SegmentValue::CenterOf(d) => d.format("%d.%m").to_string(),
//             SegmentValue::Last => "".to_string(),
//         })
//         .y_label_formatter(&seconds_to_time_formatter)
//         .axis_style(data.border_color)
//         .label_style(("IPAMincho", 20, &WHITE))
//         .draw()?;
//
//     chart.draw_series(data.data_points.iter().flat_map(|point| {
//         point.time_frames.iter().map(|frame| {
//             get_bar(
//                 point.date,
//                 frame.start.num_seconds_from_midnight() as i64,
//                 frame.end.num_seconds_from_midnight() as i64,
//                 &point.color,
//             )
//         })
//     }))?;
//
//     draw_goal_regions(&mut chart, &data.data_points, y_axis_max)?;
//
//     chart.draw_series([
//     get_border(
//         &data.border_color,
//         date_start,
//         date_end,
//         y_axis_max,
//     )])?;
//
//     root.present()?;
//     Ok(())
// }
