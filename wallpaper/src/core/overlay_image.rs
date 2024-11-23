use std::path::Path;
use image::{GenericImage, GenericImageView};

/// Overlays a smaller image onto a larger background image at the specified position
///
/// # Arguments
/// * `background_path` - Path to the large background image
/// * `overlay_path` - Path to the smaller overlay image
/// * `x` - X coordinate where the overlay should be placed
/// * `y` - Y coordinate where the overlay should be placed
/// * `output_path` - Path where the resulting image should be saved
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Ok(()) on success, Error on failure
pub(crate) fn overlay_images<P: AsRef<Path>>(
    background_path: P,
    overlay_path: P,
    x: u32,
    y: u32,
    output_path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the background and overlay images
    let mut background = image::open(background_path)?;
    let overlay = image::open(overlay_path)?;

    let (overlay_width, overlay_height) = overlay.dimensions();

    // Fast pixel copy with direct buffer access
    for overlay_y in 0..overlay_height {
        for overlay_x in 0..overlay_width {
            let pixel = overlay.get_pixel(overlay_x, overlay_y);
            background.put_pixel(x + overlay_x, y + overlay_y, pixel);
        }
    }

    // Save the result
    background.save(output_path)?;

    Ok(())
}