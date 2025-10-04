use anyhow::{anyhow, Result};
use chrono::{NaiveTime, Timelike};
use image::{ImageBuffer, Rgb};
use lifestats_domain::plot::data::PlotData;
use lifestats_domain::plot::renderer::{PlotRenderer, RenderConfig};
use lifestats_domain::plot::specification::PlotType;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use plotters::prelude::*;

use crate::common::{
    add_y_padding, calculate_seconds_ticks, get_bar, get_border, get_horizontal_line,
    seconds_to_time_formatter, BACKGROUND_COLOR, GOAL_LINE_COLOR, SHADING_COLOR, Y_MARGIN_PERCENT,
};

/// Concrete implementation of PlotRenderer using the plotters library
pub struct PlottersRenderer;

impl PlottersRenderer {
    pub fn new() -> Self {
        Self
    }

    fn render_bar_chart(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(
                data.date_range.start()..data.date_range.end(),
                0.0..100.0, // TODO: Calculate proper Y range from data
            )?;

        chart.configure_mesh().draw()?;

        // Draw bars
        let bar_data: Vec<_> = data
            .points
            .iter()
            .filter_map(|point| point.value.as_f64().map(|v| (point.date, v)))
            .collect();

        chart.draw_series(
            bar_data.iter().map(|(date, value)| {
                Rectangle::new([(*date, 0.0), (*date, *value)], BLUE.filled())
            }),
        )?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_line_chart(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(
                data.date_range.start()..data.date_range.end(),
                0.0..100.0, // TODO: Calculate proper Y range from data
            )?;

        chart.configure_mesh().draw()?;

        // Draw line
        let line_data: Vec<_> = data
            .points
            .iter()
            .filter_map(|point| point.value.as_f64().map(|v| (point.date, v)))
            .collect();

        chart.draw_series(LineSeries::new(line_data.clone(), &BLUE))?;

        // Draw points
        chart.draw_series(
            line_data
                .iter()
                .map(|(date, value)| Circle::new((*date, *value), 3, BLUE.filled())),
        )?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_scatter(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("sans-serif", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(
                data.date_range.start()..data.date_range.end(),
                0.0..100.0, // TODO: Calculate proper Y range from data
            )?;

        chart.configure_mesh().draw()?;

        // Draw scatter points
        let scatter_data: Vec<_> = data
            .points
            .iter()
            .filter_map(|point| point.value.as_f64().map(|v| (point.date, v)))
            .collect();

        chart.draw_series(
            scatter_data
                .iter()
                .map(|(date, value)| Circle::new((*date, *value), 5, BLUE.filled())),
        )?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_time_frames(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)?;

        // Calculate y-axis bounds from the data
        let (y_min, y_max) = calculate_y_axis_bounds_for_time_frames(data)?;
        let (y_axis_min, y_axis_max) = add_y_padding(
            y_min.num_seconds_from_midnight() as i64,
            y_max.num_seconds_from_midnight() as i64,
            Y_MARGIN_PERCENT,
        );

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("sans-serif", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (data.date_range.start()..data.date_range.end()).into_segmented(),
                (y_axis_min..y_axis_max).with_key_points(calculate_seconds_ticks(
                    y_axis_min,
                    y_axis_max,
                    config.height as usize,
                )),
            )?;

        chart
            .configure_mesh()
            .x_labels((data.date_range.end() - data.date_range.start()).num_days() as usize + 1)
            .disable_x_mesh()
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&seconds_to_time_formatter)
            .axis_style(WHITE)
            .label_style(("sans-serif", 15, &WHITE))
            .draw()?;

        // Draw bars for each data point's time frames
        for point in &data.points {
            if let Ok(time_frame_list) =
                serde_json::from_value::<TimeFrameList>(point.value.clone())
            {
                for time_frame in time_frame_list.iter() {
                    let start_seconds = time_frame.get_start().num_seconds_from_midnight() as i64;
                    let end_seconds = time_frame.get_end().num_seconds_from_midnight() as i64;

                    chart.draw_series([get_bar(
                        point.date,
                        start_seconds,
                        end_seconds,
                        2,
                        &BLUE.to_rgba(),
                    )])?;
                }
            }
        }

        // Draw goal lines and shading
        for point in &data.points {
            if let Some(goal_value) = &point.goal {
                if let Ok(goal_time_frames) =
                    serde_json::from_value::<TimeFrameList>(goal_value.clone())
                {
                    if goal_time_frames.is_empty() {
                        continue;
                    }

                    let mut last_end_time = 0;

                    for frame in goal_time_frames.iter() {
                        let current_start_time =
                            frame.get_start().num_seconds_from_midnight() as i64;
                        let current_end_time = frame.get_end().num_seconds_from_midnight() as i64;

                        // Shade the area between the last end time and the current start time
                        chart.draw_series([get_bar(
                            point.date,
                            last_end_time,
                            current_start_time,
                            0,
                            &SHADING_COLOR,
                        )])?;

                        // Draw the goal lines
                        chart.draw_series([
                            get_horizontal_line(
                                point.date,
                                current_start_time,
                                GOAL_LINE_COLOR.mix(0.7),
                            ),
                            get_horizontal_line(
                                point.date,
                                current_end_time,
                                GOAL_LINE_COLOR.mix(0.7),
                            ),
                        ])?;

                        last_end_time = current_end_time;
                    }

                    // Shade the area between the last end time and the y-axis max
                    chart.draw_series([get_bar(
                        point.date,
                        last_end_time,
                        y_axis_max,
                        0,
                        &SHADING_COLOR,
                    )])?;
                }
            }
        }

        // Draw border
        chart.draw_series([get_border(
            &WHITE,
            data.date_range.start(),
            data.date_range.end(),
            y_axis_max,
        )])?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }
}

/// Helper function to calculate y-axis bounds from TimeFrameList data
fn calculate_y_axis_bounds_for_time_frames(data: &PlotData) -> Result<(NaiveTime, NaiveTime)> {
    const DEFAULT_MIN: NaiveTime = match NaiveTime::from_num_seconds_from_midnight_opt(0, 0) {
        Some(t) => t,
        None => panic!("Invalid default min time"),
    };
    const DEFAULT_MAX: NaiveTime = match NaiveTime::from_num_seconds_from_midnight_opt(1000, 0) {
        Some(t) => t,
        None => panic!("Invalid default max time"),
    };

    let mut lower_values = Vec::new();
    let mut upper_values = Vec::new();

    // Collect from values
    for point in &data.points {
        if let Ok(time_frame_list) = serde_json::from_value::<TimeFrameList>(point.value.clone()) {
            if let Some(start) = time_frame_list.start_time() {
                lower_values.push(start);
            }
            if let Some(end) = time_frame_list.end_time() {
                upper_values.push(end);
            }
        }
    }

    // Collect from goals
    for point in &data.points {
        if let Some(goal_value) = &point.goal {
            if let Ok(goal_time_frames) =
                serde_json::from_value::<TimeFrameList>(goal_value.clone())
            {
                if let Some(start) = goal_time_frames.start_time() {
                    lower_values.push(start);
                }
                if let Some(end) = goal_time_frames.end_time() {
                    upper_values.push(end);
                }
            }
        }
    }

    let y_min = lower_values.into_iter().min().unwrap_or(DEFAULT_MIN);
    let y_max = upper_values.into_iter().max().unwrap_or(DEFAULT_MAX);

    Ok((y_min, y_max))
}

impl Default for PlottersRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotRenderer for PlottersRenderer {
    fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        match data.plot_type {
            PlotType::BarChart => self.render_bar_chart(data, config),
            PlotType::LineChart => self.render_line_chart(data, config),
            PlotType::Scatter => self.render_scatter(data, config),
            PlotType::TimeFrames => self.render_time_frames(data, config),
            PlotType::CircularProgress => {
                // TODO: Implement circular progress rendering
                Err(anyhow!("CircularProgress plot type not yet implemented"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_domain::plot::data::{DataPoint, PlotData};
    use lifestats_domain::shared::date_range::DateRange;
    use serde_json::Value;

    #[test]
    fn test_render_line_chart() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                Value::from(10.0),
                None,
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                Value::from(20.0),
                None,
            ),
        ];

        let plot_data = PlotData::new(
            "Test Plot".to_string(),
            points,
            date_range,
            PlotType::LineChart,
        );

        let config = RenderConfig::default();
        let result = renderer.render(&plot_data, &config);

        assert!(result.is_ok());
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());

        // Verify it's a valid PNG by checking magic bytes
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_render_time_frames_plot() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Create TimeFrameList data as JSON
        let time_frame_list_json = serde_json::json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "17:00:00"}
        ]);

        let goal_json = serde_json::json!([
            {"start": "08:00:00", "end": "18:00:00"}
        ]);

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                time_frame_list_json.clone(),
                Some(goal_json.clone()),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                time_frame_list_json.clone(),
                Some(goal_json),
            ),
        ];

        let plot_data = PlotData::new(
            "Time Frames Test Plot".to_string(),
            points,
            date_range,
            PlotType::TimeFrames,
        );

        let config = RenderConfig::default();
        let result = renderer.render(&plot_data, &config);

        assert!(result.is_ok(), "Rendering should succeed");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");

        // Verify it's a valid PNG by checking magic bytes
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "Should be a valid PNG"
        );
    }
}
