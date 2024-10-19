// use std::collections::HashMap;
// use plotly::{Bar, Layout, Plot};
// use chrono::{Local, NaiveTime, Timelike};
// use anyhow::Result;
// use plotly::common::{HoverInfo, Marker};
// use plotly::layout::BarMode;
// use backend::objects::LabeledTimeFrame;
//
// pub fn visualize_timeline(timeline: Vec<LabeledTimeFrame>) -> Result<()> {
//     let mut plot = Plot::new();
//     let today = Local::now().date_naive();
//
//     let mut color_map = HashMap::new();
//     let colors = vec!["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf"];
//     let mut color_index = 0;
//
//     let mut previous_end = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
//     for frame in timeline.iter() {
//         let start = frame.time_frame.start.unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
//         let end = frame.time_frame.end.unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
//
//         let start_seconds = start.num_seconds_from_midnight() as f64;
//         let end_seconds = end.num_seconds_from_midnight() as f64;
//         let duration = end_seconds - start_seconds;
//
//         let gap = start_seconds - previous_end.num_seconds_from_midnight() as f64;
//
//         // Invisible gap
//         let base_bar = Bar::new(vec![today.format("%Y-%m-%d").to_string()], vec![gap])
//             .name("")
//             .hover_info(HoverInfo::None)
//             .marker(Marker::new().color("rgba(0,0,0,0)"));
//
//         let color = color_map.entry(frame.label.clone()).or_insert_with(|| {
//             let color = colors[color_index % colors.len()];
//             color_index += 1;
//             color.to_string()
//         });
//
//         let duration_bar = Bar::new(vec![today.format("%Y-%m-%d").to_string()], vec![duration])
//             .name(&frame.label)
//             .hover_info(HoverInfo::Text)
//             .text(frame.label.clone())
//             .show_legend(false)
//             .marker(Marker::new().color(color.clone()));
//
//         plot.add_trace(base_bar);
//         plot.add_trace(duration_bar);
//
//         previous_end = end;
//     }
//
//     let tick_values: Vec<f64> = (0..=24).map(|h| (h * 3600) as f64).collect();
//     let tick_text: Vec<String> = (0..=24).map(|h| format!("{:02}:00", h)).collect();
//
//     let layout = Layout::new()
//         .title("Timeline for Today")
//         .x_axis(plotly::layout::Axis::new().title("Date"))
//         .y_axis(plotly::layout::Axis::new()
//             .title("Time")
//             .range(vec![6.0*60.0*60.0, 22.0*60.0*60.0])
//             .tick_values(tick_values)
//             .tick_text(tick_text)
//         )
//         .bar_mode(BarMode::Stack); // Stack bars
//
//     plot.set_layout(layout);
//     plot.show();
//
//     Ok(())
// }
//
// fn time_to_str(time: NaiveTime) -> String {
//     format!("{:02}:{:02}", time.hour(), time.minute())
// }
//
// fn minutes_since_midnight(time: NaiveTime) -> u32 {
//     time.hour() * 60 + time.minute()
// }