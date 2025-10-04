use crate::plot::objects::mutable_image::MutableImage;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::plot::plot_backends::plotters::common::{
    add_y_padding, calculate_y_min_max, color_to_plotters, get_bar,
    get_border, get_horizontal_line, get_streak_text,
};
use crate::plot::plot_backends::plotters::constants::{
    BACKGROUND_COLOR, GOAL_LINE_COLOR, Y_MARGIN_PERCENT,
};
use anyhow::Result;
use chrono::NaiveDate;
use plotters::prelude::*;
use transfer_objects::image::Image;

pub fn barplot(
    title: &str,
    size: (usize, usize),
    data: Vec<PlotDataPoint<i64>>,
    _y_unit: &str,
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

        let y_axis_max = barplot_calculate_axis_max(&data);

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (x_min..x_max).into_segmented(),
                (0..y_axis_max).with_key_points(y_axis_conversion.get_ticks(0, y_axis_max, size.1)),
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

        // Draw bars
        chart.draw_series(data.iter().filter_map(|point| {
            point.value.map(|value| {
                get_bar(
                    point.date,
                    0,
                    value,
                    2,
                    &color_to_plotters(&point.color).to_rgba(),
                )
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

fn barplot_calculate_axis_max(data: &Vec<PlotDataPoint<i64>>) -> i64 {
    let minmax = calculate_y_min_max(&data).unwrap_or((0, 100));
    let (_, y_axis_max) = add_y_padding(0, minmax.1, Y_MARGIN_PERCENT);
    y_axis_max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot::objects::color;

    #[test]
    fn test_correct_maximum_case_value_more_than_goal() {
        let input = vec![
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 21).unwrap(),
                Some(60 * 60 + 30),
                Some(60 * 60),
                color::GREEN,
            ),
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 22).unwrap(),
                Some(61 * 60),
                Some(60 * 60),
                color::GREEN,
            ),
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 23).unwrap(),
                Some(65 * 60),
                Some(60 * 60),
                color::GREEN,
            ),
        ];

        let result = barplot_calculate_axis_max(&input);

        assert_eq!(result, 4387);
    }
}
