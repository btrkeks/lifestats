use crate::plot::objects;
use crate::plot::plot_backends::own;
use anyhow::Result;
use transfer_objects::image::Image;

pub fn circular_progress_plot(
    title: String,
    size: (usize, usize),
    value: i64,
    max_value: i64,
    color: objects::color::Color,
) -> Result<Image> {
    own::circular_progress_plot::circular_progress_plot(title, size, value, max_value, color)
}
