use crate::plot::objects::color;
use crate::plot::plot_backends::own::image_builder::{DrawableImage, TextAlignment};
use std::f32::consts::PI;
use transfer_objects::image::Image;

pub fn circular_progress_plot(
    _title: String,
    size: (usize, usize),
    value: i64,
    max_value: i64,
    color: color::Color,
) -> anyhow::Result<Image> {
    let (radius, thickness) = calculate_radius_and_thickness(size);
    let start_angle = -PI / 2.0;
    let end_angle = (value as f32 / max_value as f32) * 2.0 * PI + start_angle;

    let (center_x, center_y) = (size.0 as f32 / 2.0, size.1 as f32 / 2.0);

    let text = format!("{}/{}", value, max_value);
    let font_size = calculate_font_size(size);

    // TODO: Add title
    Ok(DrawableImage::new(size.0, size.1)
        .fill(color::LIGHT_BLACK)
        .draw_arc(
            (center_x, center_y),
            radius as f32,
            thickness as f32,
            start_angle,
            end_angle,
            color,
        )
        .draw_arc(
            (center_x, center_y),
            radius as f32,
            thickness as f32,
            end_angle,
            start_angle + 2.0 * PI,
            color::GREY,
        )
        .draw_text(
            &text,
            font_size,
            (center_x, center_y),
            TextAlignment::Center,
            color::WHITE,
        )
        .build())
}

fn calculate_radius_and_thickness(_size: (usize, usize)) -> (u32, u32) {
    // TODO: Implement
    (100, 10)
}

fn calculate_font_size(_size: (usize, usize)) -> u32 {
    // TODO: Implement
    50
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use image::{ImageBuffer, Rgb};
    use std::path::{Path, PathBuf};
    use transfer_objects::image::Image;

    /// Returns the path to the test resources directory
    fn test_resources_dir() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir).join("tests").join("resources")
    }

    fn load_test_image(string: &str) -> Result<Image> {
        let path = test_resources_dir().join(format!("input/{}.png", string));
        let img = image::open(&path)?;
        Image::from_dynamic_image(img)
    }

    fn save_test_image(image: &Image, name: &str) {
        let path = test_resources_dir()
            .join("output")
            .join(format!("{}.png", name));
        save_buffer_to_file(image, &path);
    }

    fn save_buffer_to_file(image: &Image, path: &Path) {
        let img_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(
            image.width() as u32,
            image.height() as u32,
            image.get_rgb_buffer(),
        )
        .unwrap();

        img_buffer.save(path).unwrap();
    }

    #[test]
    fn test_circular_progress_plot() {
        let size = (400, 400);
        let value = 40;
        let max_value = 100;
        let color = color::Color::from_rgb(0, 0, 255);

        let image =
            circular_progress_plot("Test".to_string(), size, value, max_value, color).unwrap();

        save_test_image(&image, "test_circular_progress_plot_1");
    }
}
