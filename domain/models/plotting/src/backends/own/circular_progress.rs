use std::f32::consts::PI;

/// Draws a thick arc into an RGB buffer
///
/// # Arguments
/// * `buffer` - RGB buffer as a flat vector of u8 values
/// * `width` - Width of the image
/// * `height` - Height of the image
/// * `center_x` - X coordinate of arc center
/// * `center_y` - Y coordinate of arc center
/// * `radius` - Radius of the arc
/// * `thickness` - Thickness of the arc in pixels
/// * `start_angle` - Starting angle in radians
/// * `end_angle` - Ending angle in radians
/// * `color` - RGB color as [u8; 3]
pub fn draw_thick_arc(
    buffer: &mut [u8],
    width: usize,
    height: usize,
    center_x: f32,
    center_y: f32,
    radius: f32,
    thickness: f32,
    start_angle: f32,
    end_angle: f32,
    color: [u8; 3],
) {
    // Pre-calculate bounds for faster checking
    let inner_radius_sq = (radius - thickness/2.0).powi(2);
    let outer_radius_sq = (radius + thickness/2.0).powi(2);

    // Calculate bounding box for the arc to avoid checking every pixel
    let bbox_min_x = ((center_x - radius - thickness/2.0).floor() as isize).max(0) as usize;
    let bbox_max_x = ((center_x + radius + thickness/2.0).ceil() as isize).min(width as isize - 1) as usize;
    let bbox_min_y = ((center_y - radius - thickness/2.0).floor() as isize).max(0) as usize;
    let bbox_max_y = ((center_y + radius + thickness/2.0).ceil() as isize).min(height as isize - 1) as usize;

    // Normalize angles to [0, 2π]
    let start_angle = normalize_angle(start_angle);
    let mut end_angle = normalize_angle(end_angle);
    if end_angle <= start_angle {
        end_angle += 2.0 * PI;
    }

    // Iterate only through the bounding box
    for y in bbox_min_y..=bbox_max_y {
        let y_offset = y * width * 3;
        let dy = y as f32 - center_y;

        for x in bbox_min_x..=bbox_max_x {
            let dx = x as f32 - center_x;
            let dist_sq = dx * dx + dy * dy;

            // Check if point is within the thickness bounds
            if dist_sq >= inner_radius_sq && dist_sq <= outer_radius_sq {
                // Calculate angle of current point
                let mut angle = f32::atan2(dy, dx);
                if angle < 0.0 {
                    angle += 2.0 * PI;
                }

                // Check if angle is within arc bounds
                if angle >= start_angle && angle <= end_angle {
                    let pixel_idx = (y_offset + x * 3) as usize;
                    buffer[pixel_idx] = color[0];
                    buffer[pixel_idx + 1] = color[1];
                    buffer[pixel_idx + 2] = color[2];
                }
            }
        }
    }
}

#[inline]
fn normalize_angle(angle: f32) -> f32 {
    let mut result = angle % (2.0 * PI);
    if result < 0.0 {
        result += 2.0 * PI;
    }
    result
}

// Example usage and benchmarking
#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::image::Image;

    #[test]
    fn test_1() -> anyhow::Result<()> {
        let width = 500;
        let height = 500;
        let mut image = Image::new_with_size((width, height));

        draw_thick_arc(
            image.get_buffer(),
            width,
            height,
            250.0,
            250.0,
            200.0,
            20.0,
            PI / 2.0,
            2.0 *  PI,
            [255, 0, 0]
        );

        // Save the plot to a temporary file for verification
        let temp_path = std::path::Path::new("test_circular.png");
        image.save(&temp_path)?;

        // Verify the file exists and has non-zero size
        assert!(temp_path.exists());
        assert!(temp_path.metadata()?.len() > 0);

        // Clean up
        // std::fs::remove_file(temp_path)?;

        Ok(())
    }

}