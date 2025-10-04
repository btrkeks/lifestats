use anyhow::{anyhow, Result};
use image::{ImageBuffer, Rgb};
use lifestats_domain::plot::data::PlotData;
use lifestats_domain::plot::renderer::{PlotRenderer, RenderConfig};
use lifestats_domain::plot::specification::PlotType;
use plotters::prelude::*;

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
            PlotType::TimeFrames => {
                // TODO: Implement time frames rendering
                Err(anyhow!("TimeFrames plot type not yet implemented"))
            }
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
}
