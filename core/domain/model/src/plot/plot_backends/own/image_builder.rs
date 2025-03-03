use crate::plot::objects::color::Color;
use crate::plot::objects::mutable_image::MutableImage;
use ab_glyph::{FontArc, PxScale};
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{draw_text_mut, text_size};
use std::f32::consts::PI;
use transfer_objects::image::Image;

pub struct DrawableImage {
    image: MutableImage,
    font: FontArc,
}

impl DrawableImage {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            image: MutableImage::new_with_size(width, height),
            font: Self::load_font(),
        }
    }

    fn load_font() -> FontArc {
        FontArc::try_from_slice(include_bytes!(
            "../../../../assets/fonts/NotoSansJP-Regular.otf"
        ))
        .expect("Error loading font")
    }

    pub fn build(self) -> Image {
        self.image.into_immutable().unwrap()
    }

    pub fn fill(mut self, color: Color) -> Self {
        let buffer = self.image.get_rgb_buffer_mut();

        for pixel in buffer.array_chunks_mut::<3>() {
            let (r, g, b) = color.get_rgb();
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }

        self
    }

    /// Starts at (1, 0) viewed from `center` and goes clockwise.
    pub fn draw_arc(
        mut self,
        center: (f32, f32),
        radius: f32,
        thickness: f32,
        start_angle: f32,
        end_angle: f32,
        color: Color,
    ) -> Self {
        let width = self.image.width();
        let half_thickness = thickness * 0.5;

        // Pre-calculate trigonometric values
        let (start_sin, start_cos) = start_angle.sin_cos();
        let (end_sin, end_cos) = end_angle.sin_cos();

        // Pre-fetch color components
        let (r, g, b) = color.get_rgb();

        let inner_radius_sq = (radius - half_thickness).powi(2);
        let outer_radius_sq = (radius + half_thickness).powi(2);

        let bbox = calculate_bbox(center, radius, half_thickness, width, self.image.height());

        let buffer = self.image.get_rgb_buffer_mut();

        for y in bbox.min_y..=bbox.max_y {
            let y_offset = y * width * 3;
            let dy = y as f32 - center.1;

            for x in bbox.min_x..=bbox.max_x {
                let dx = x as f32 - center.0;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq >= inner_radius_sq && dist_sq <= outer_radius_sq {
                    // Use dot product instead of atan2
                    if is_point_in_arc(dx, dy, start_sin, start_cos, end_sin, end_cos) {
                        let pixel_idx = y_offset + x * 3;
                        buffer[pixel_idx] = r;
                        buffer[pixel_idx + 1] = g;
                        buffer[pixel_idx + 2] = b;
                    }
                }
            }
        }
        self
    }

    pub fn draw_text(
        mut self,
        text: &str,
        font_size: u32,
        coord: (f32, f32),
        alignment: TextAlignment,
        color: Color,
    ) -> Self {
        // Create the scale for the desired font size
        let scale = PxScale::from(font_size as f32);

        let (x, y) = self.get_coord_for_alignment(coord, alignment, text, scale);

        // Convert our buffer to ImageBuffer
        let mut image: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(
            self.image.width() as u32,
            self.image.height() as u32,
            self.image.get_rgb_buffer_mut().to_vec(),
        )
        .expect("Failed to create ImageBuffer");

        // Draw the text
        let (r, g, b) = color.get_rgb();
        draw_text_mut(
            &mut image,
            Rgb([r, g, b]),
            x as i32,
            y as i32,
            scale,
            &self.font,
            text,
        );

        // Convert back to our buffer
        self.image = MutableImage::new_with_size(self.image.width(), self.image.height());
        self.image
            .get_rgb_buffer_mut()
            .copy_from_slice(&image.into_raw());

        self
    }

    fn get_coord_for_alignment(
        &self,
        coord: (f32, f32),
        alignment: TextAlignment,
        text: &str,
        scale: impl Into<PxScale> + Copy,
    ) -> (f32, f32) {
        let (text_width, text_height) = text_size(scale, &self.font, text);

        match alignment {
            TextAlignment::Center => (
                coord.0 - text_width as f32 / 2.0,
                coord.1 - text_height as f32 / 2.0,
            ),
            _ => unimplemented!(),
        }
    }
}

#[inline]
fn is_point_in_arc(
    dx: f32,
    dy: f32,
    start_sin: f32,
    start_cos: f32,
    end_sin: f32,
    end_cos: f32,
) -> bool {
    // Calculate the angle of the point
    let point_angle = f32::atan2(dy, dx);
    let point_angle = if point_angle < 0.0 {
        point_angle + 2.0 * PI
    } else {
        point_angle
    };

    // Calculate the start and end angles from the sin/cos values
    let start_angle = f32::atan2(start_sin, start_cos);
    let start_angle = if start_angle < 0.0 {
        start_angle + 2.0 * PI
    } else {
        start_angle
    };

    let mut end_angle = f32::atan2(end_sin, end_cos);
    end_angle = if end_angle < 0.0 {
        end_angle + 2.0 * PI
    } else {
        end_angle
    };

    // Handle wraparound case
    if end_angle < start_angle {
        end_angle += 2.0 * PI;
    }

    // Handle wraparound for point angle
    let mut extended_point_angle = point_angle;
    if point_angle < start_angle {
        extended_point_angle += 2.0 * PI;
    }

    extended_point_angle >= start_angle && extended_point_angle <= end_angle
}

struct BBox {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

#[inline]
fn calculate_bbox(
    center: (f32, f32),
    radius: f32,
    half_thickness: f32,
    width: usize,
    height: usize,
) -> BBox {
    let total_radius = radius + half_thickness;
    BBox {
        min_x: ((center.0 - total_radius).floor() as isize).max(0) as usize,
        max_x: ((center.0 + total_radius).ceil() as isize).min(width as isize - 1) as usize,
        min_y: ((center.1 - total_radius).floor() as isize).max(0) as usize,
        max_y: ((center.1 + total_radius).ceil() as isize).min(height as isize - 1) as usize,
    }
}

pub enum TextAlignment {
    Center,
    // TopLeft,
    // TopRight,
    // BottomLeft,
    // BottomRight,
}
