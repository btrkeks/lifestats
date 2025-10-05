use anyhow::{anyhow, Result};
use chrono::Timelike;
use image::{ImageBuffer, Rgb};
use lifestats_domain::plot::axis::YAxis;
use lifestats_domain::plot::data::{DrawableData, DrawableGoal, PlotData};
use lifestats_domain::plot::renderer::{PlotRenderer, RenderConfig};
use lifestats_domain::plot::specification::PlotType;
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

        // Extract y-axis range from PlotData (provided by domain logic)
        let (y_min, y_max) = match &data.y_axis {
            YAxis::Integer { min, max } => (*min as f64, *max as f64),
            YAxis::Time { .. } => {
                return Err(anyhow!(
                    "BarChart plot requires Integer axis, but got Time axis"
                ))
            }
        };

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("IPAMincho", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(data.date_range.start()..data.date_range.end(), y_min..y_max)?;

        chart.configure_mesh().draw()?;

        // Draw bars using DrawableData and pre-calculated colors
        for point in &data.points {
            if let DrawableData::Integer(value) = &point.value {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([Rectangle::new(
                    [(point.date, 0.0), (point.date, *value as f64)],
                    color.filled(),
                )])?;
            }
        }

        // Draw goal lines
        for point in &data.points {
            if let Some(DrawableGoal::Integer(goal_number)) = &point.goal {
                chart.draw_series([PathElement::new(
                    [
                        (point.date, *goal_number as f64),
                        (point.date + chrono::Duration::days(1), *goal_number as f64),
                    ],
                    GOAL_LINE_COLOR.stroke_width(2),
                )])?;
            }
        }

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

        // Extract y-axis range from PlotData (provided by domain logic)
        let (y_min, y_max) = match &data.y_axis {
            YAxis::Integer { min, max } => (*min as f64, *max as f64),
            YAxis::Time { .. } => {
                return Err(anyhow!(
                    "LineChart plot requires Integer axis, but got Time axis"
                ))
            }
        };

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("IPAMincho", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(data.date_range.start()..data.date_range.end(), y_min..y_max)?;

        chart.configure_mesh().draw()?;

        // Extract line data using DrawableData
        let line_data: Vec<_> = data
            .points
            .iter()
            .filter_map(|point| {
                if let DrawableData::Integer(v) = &point.value {
                    Some((point.date, *v as f64))
                } else {
                    None
                }
            })
            .collect();

        chart.draw_series(LineSeries::new(line_data.clone(), &BLUE))?;

        // Draw points with pre-calculated colors
        for point in &data.points {
            if let DrawableData::Integer(value) = &point.value {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([Circle::new((point.date, *value as f64), 3, color.filled())])?;
            }
        }

        // Draw goal lines
        for point in &data.points {
            if let Some(DrawableGoal::Integer(goal_number)) = &point.goal {
                chart.draw_series([PathElement::new(
                    [
                        (point.date, *goal_number as f64),
                        (point.date + chrono::Duration::days(1), *goal_number as f64),
                    ],
                    GOAL_LINE_COLOR.stroke_width(2),
                )])?;
            }
        }

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

        // Extract y-axis range from PlotData (provided by domain logic)
        let (y_min, y_max) = match &data.y_axis {
            YAxis::Integer { min, max } => (*min as f64, *max as f64),
            YAxis::Time { .. } => {
                return Err(anyhow!(
                    "Scatter plot requires Integer axis, but got Time axis"
                ))
            }
        };

        let mut chart = ChartBuilder::on(&root)
            .caption(&data.title, ("IPAMincho", 30))
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(data.date_range.start()..data.date_range.end(), y_min..y_max)?;

        chart.configure_mesh().draw()?;

        // Draw scatter points with DrawableData and pre-calculated colors
        for point in &data.points {
            if let DrawableData::Integer(value) = &point.value {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([Circle::new((point.date, *value as f64), 5, color.filled())])?;
            }
        }

        // Draw goal lines
        for point in &data.points {
            if let Some(DrawableGoal::Integer(goal_number)) = &point.goal {
                chart.draw_series([PathElement::new(
                    [
                        (point.date, *goal_number as f64),
                        (point.date + chrono::Duration::days(1), *goal_number as f64),
                    ],
                    GOAL_LINE_COLOR.stroke_width(2),
                )])?;
            }
        }

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

        // Extract y-axis bounds from PlotData (provided by domain logic)
        let (y_axis_min, y_axis_max) = match &data.y_axis {
            YAxis::Time { min, max } => {
                let (min_seconds, max_seconds) = add_y_padding(
                    min.num_seconds_from_midnight() as i64,
                    max.num_seconds_from_midnight() as i64,
                    Y_MARGIN_PERCENT,
                );
                (min_seconds, max_seconds)
            }
            YAxis::Integer { .. } => {
                return Err(anyhow!(
                    "TimeFrames plot requires Time axis, but got Integer axis"
                ))
            }
        };

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("IPAMincho", 20).into_font().color(&WHITE))
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
            .label_style(("IPAMincho", 15, &WHITE))
            .draw()?;

        // Draw bars for each data point's time frames
        for point in &data.points {
            // Use pattern matching on DrawableData - type-safe!
            if let DrawableData::TimeFrames(time_frame_list) = &point.value {
                for time_frame in time_frame_list.iter() {
                    let start_seconds = time_frame.get_start().num_seconds_from_midnight() as i64;
                    let end_seconds = time_frame.get_end().num_seconds_from_midnight() as i64;

                    // Use pre-calculated color from the data point
                    let [r, g, b] = point.color;
                    let color = RGBColor(r, g, b);

                    chart.draw_series([get_bar(
                        point.date,
                        start_seconds,
                        end_seconds,
                        2,
                        &color.to_rgba(),
                    )])?;
                }
            }
        }

        // Draw goal lines and shading
        for point in &data.points {
            // Use pattern matching on DrawableGoal - no more JSON parsing!
            if let Some(DrawableGoal::TimeFrames(goal_time_frames)) = &point.goal {
                if goal_time_frames.is_empty() {
                    continue;
                }

                let mut last_end_time = 0;

                for frame in goal_time_frames.iter() {
                    let current_start_time = frame.get_start().num_seconds_from_midnight() as i64;
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
                        get_horizontal_line(point.date, current_end_time, GOAL_LINE_COLOR.mix(0.7)),
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

    #[test]
    fn test_render_line_chart() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                DrawableData::Integer(10),
                [0, 255, 0],
                None,
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                DrawableData::Integer(20),
                [0, 255, 0],
                None,
            ),
        ];

        let y_axis = YAxis::Integer { min: 0, max: 30 };
        let plot_data = PlotData::new(
            "Test Plot".to_string(),
            points,
            date_range,
            PlotType::LineChart,
            y_axis,
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
        use lifestats_domain::shared::time_frame_list::TimeFrameList;

        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Create TimeFrameList data
        let time_frame_list = serde_json::from_value::<TimeFrameList>(serde_json::json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "17:00:00"}
        ]))
        .unwrap();

        let goal_time_frames = serde_json::from_value::<TimeFrameList>(serde_json::json!([
            {"start": "08:00:00", "end": "18:00:00"}
        ]))
        .unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                DrawableData::TimeFrames(time_frame_list.clone()),
                [0, 255, 0],
                Some(DrawableGoal::TimeFrames(goal_time_frames.clone())),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                DrawableData::TimeFrames(time_frame_list.clone()),
                [0, 255, 0],
                Some(DrawableGoal::TimeFrames(goal_time_frames)),
            ),
        ];

        use chrono::NaiveTime;
        let y_axis = YAxis::Time {
            min: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            max: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        };
        let plot_data = PlotData::new(
            "Time Frames Test Plot".to_string(),
            points,
            date_range,
            PlotType::TimeFrames,
            y_axis,
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
