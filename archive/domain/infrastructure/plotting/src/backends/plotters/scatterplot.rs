use crate::plot::objects::mutable_image::MutableImage;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::plot::plot_backends::plotters::common::{
    calculate_y_axis_bounds, color_to_plotters, get_border, get_horizontal_line, get_streak_text,
};
use crate::plot::plot_backends::plotters::constants::{BACKGROUND_COLOR, GOAL_LINE_COLOR};
use anyhow::Result;
use chrono::NaiveDate;
use plotters::prelude::*;
use transfer_objects::image::Image;

pub fn scatter_plot(
    title: String,
    size: (usize, usize),
    data: Vec<PlotDataPoint<i64>>,
    _y_unit: String,
    border_color: RGBColor,
    y_axis_conversion: YAxisConversion,
    x_min: NaiveDate,
    x_max: NaiveDate,
    streak: Option<u32>,
) -> Result<Image> {
    let mut image = MutableImage::new_with_size(size.0, size.1);
    {
        let root =
            BitMapBackend::with_buffer(image.get_rgb_buffer_mut(), (size.0 as u32, size.1 as u32))
                .into_drawing_area();
        root.fill(BACKGROUND_COLOR)?;

        let (y_axis_min, y_axis_max) = calculate_y_axis_bounds(&data);

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (x_min..x_max).into_segmented(),
                (y_axis_min..y_axis_max)
                    .with_key_points(y_axis_conversion.get_ticks(y_axis_min, y_axis_max, size.1)),
            )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .x_labels((x_max - x_min).num_days() as usize + 1)
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(y_axis_conversion.get_formatter())
            .axis_style(border_color)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Draw horizontal lines
        chart.draw_series(data.iter().filter_map(|point| {
            point.value.map(|value| {
                get_horizontal_line(point.date, value, color_to_plotters(&point.color).to_rgba())
            })
        }))?;

        // Draw goal lines
        chart.draw_series(
            data.iter()
                .filter_map(|point| point.goal.map(|goal| (point.date, goal)))
                .map(|(date, goal)| get_horizontal_line(date, goal, GOAL_LINE_COLOR)),
        )?;

        // Draw border
        chart.draw_series([get_border(&border_color, x_min, x_max, y_axis_max)])?;

        // Draw streak
        if let Some(streak_value) = streak {
            root.draw(&get_streak_text(size.0 as u32, streak_value))?;
        }

        root.present()?;
    }
    image.into_immutable()
}
