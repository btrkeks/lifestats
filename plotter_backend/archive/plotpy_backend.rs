// use plotpy::{Barplot, Plot};
// use chrono::{Local, NaiveTime, Timelike};
// use backend::objects::LabeledTimeFrame;
// use anyhow::Result;
//
// pub(crate) fn visualize_timeline(timeline: Vec<LabeledTimeFrame>) -> Result<()> {
//     let mut barplot = Barplot::new();
//     let date = Local::now().date_naive();
//
//     // Convert timeline data to plottable format
//     let mut labels: Vec<&str> = Vec::new();
//     let mut bottoms: Vec<f64> = Vec::new();
//     let mut durations = Vec::new();
//     let mut color_strings = Vec::new();
//     let mut color_refs = Vec::new();
//
//     for (index, frame) in timeline.iter().enumerate() {
//         labels.push(frame.label.as_str());
//         let start = frame.time_frame.start.unwrap_or(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
//         let end = frame.time_frame.end.unwrap_or(NaiveTime::from_hms_opt(23, 59, 59).unwrap());
//         let duration = end.num_seconds_from_midnight() - start.num_seconds_from_midnight();
//
//         bottoms.push(start.num_seconds_from_midnight() as f64);
//         durations.push(duration);
//         color_strings.push(generate_color(index));
//     }
//
//     for color in &color_strings {
//         color_refs.push(color.as_str());
//     }
//
//     // Configure and draw the barplot
//     barplot
//         .set_horizontal(false)
//         .set_bottom(&bottoms)
//         .set_colors(&color_refs)
//         .set_with_text("center")
//         .draw(date.to_string(), &durations);
//
//     // Create the plot and add the barplot
//     let mut plot = Plot::new();
//     plot.add(&barplot)
//         .set_title("Daily Timeline")
//         .set_label_x("Duration (minutes)")
//         .set_label_y("Activities")
//         .set_yrange(6.0 * 60.0 * 60.0, 22.0 * 60.0 * 60.0);
//         // .set_x_ticks(&[0.0, 6.0*60.0, 12.0*60.0, 18.0*60.0, 24.0*60.0], Some(&["00:00", "06:00", "12:00", "18:00", "24:00"]), &plotpy::Props::default());
//
//     // Save the plot
//     plot.save("/tmp/timeline.png")
//         .map_err(|e| anyhow::anyhow!("Failed to save plot: {}", e))?;
//
//     Ok(())
// }
//
// fn time_to_minutes(time: NaiveTime) -> f64 {
//     (time.hour() as f64 * 60.0) + time.minute() as f64 + (time.second() as f64 / 60.0)
// }
//
// fn generate_color(index: usize) -> String {
//     let hue = (index as f64 * 137.5) % 360.0;
//     format!("#{:02X}{:02X}{:02X}",
//             (hue.sin() * 127.0 + 128.0) as u8,
//             ((hue + 120.0).sin() * 127.0 + 128.0) as u8,
//             ((hue + 240.0).sin() * 127.0 + 128.0) as u8
//     )
// }