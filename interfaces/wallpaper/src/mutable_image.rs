use anyhow::Context;
use anyhow::Result;
use image::{ImageBuffer, Rgb};
use std::path::Path;
use transfer_objects::image::Image;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MutableImage {
    width: usize,
    height: usize,
    buffer: Box<[u8]>, // RGB format
}

impl MutableImage {
    pub fn new_with_size(width: usize, height: usize) -> Self {
        let size = width * height * 3; // RGB format
        Self {
            width,
            height,
            buffer: unsafe { Box::new_uninit_slice(size).assume_init() },
        }
    }

    pub fn new_from_buffer(width: usize, height: usize, buffer: Box<[u8]>) -> Result<Self> {
        if buffer.len() != width * height * 3 {
            return Err(anyhow::anyhow!("Buffer size does not match dimensions"));
        }

        Ok(Self {
            width,
            height,
            buffer,
        })
    }

    pub fn from_dynamic_image(img: image::DynamicImage) -> Result<Self> {
        let width = img.width() as usize;
        let height = img.height() as usize;
        let buffer = img.into_rgb8().into_raw().into_boxed_slice();

        Ok(Self {
            width,
            height,
            buffer,
        })
    }

    pub fn get_rgb_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn get_rgb_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Create an ImageBuffer from our raw pixel data
        let img_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(
            self.width as u32,
            self.height as u32,
            &*self.buffer,
        )
        .context("Failed to create image buffer from raw data")?;

        // Save the image buffer as PNG
        img_buffer
            .save(path)
            .context("Failed to save image to file")?;

        Ok(())
    }

    /// Overlays another image onto this image at the specified coordinates.
    /// The overlay is performed by copying RGB values from the source image
    /// directly into this image's buffer at the specified position.
    ///
    /// # Arguments
    ///
    /// * `other` - The source image to overlay onto this image
    /// * `x` - The x coordinate where the overlay should begin
    /// * `y` - The y coordinate where the overlay should begin
    ///
    /// # Panics
    ///
    /// This method will panic if:
    /// * The overlay image would extend beyond the boundaries of the target image
    /// * Either image's buffer is not properly sized for its dimensions
    pub fn overlay(&mut self, other: &Image, x: usize, y: usize) {
        // Skip if completely out of bounds vertically
        if y >= self.height {
            return;
        }

        // Calculate visible rows
        let max_rows = if y + other.height() > self.height {
            self.height - y
        } else {
            other.height()
        };

        // Skip if completely out of bounds horizontally
        if x >= self.width {
            return;
        }

        // Calculate visible columns
        let max_cols = if x + other.width() > self.width {
            self.width - x
        } else {
            other.width()
        };

        let other_buffer = other.get_rgb_buffer();
        // Process only the visible portion of the image
        for row in 0..max_rows {
            let src_start = row * other.width() * 3;
            let dst_start = ((y + row) * self.width + x) * 3;

            for col in 0..max_cols {
                let src_pos = src_start + col * 3;
                let dst_pos = dst_start + col * 3;

                // Copy R, G, and B values
                self.buffer[dst_pos] = other_buffer[src_pos];
                self.buffer[dst_pos + 1] = other_buffer[src_pos + 1];
                self.buffer[dst_pos + 2] = other_buffer[src_pos + 2];
            }
        }
    }

    pub fn into_image(self) -> Result<Image> {
        Image::new_from_buffer(self.width, self.height, self.buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use image::Rgb;
    use std::path::PathBuf;

    /// Returns the path to the test resources directory
    fn test_resources_dir() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir).join("tests").join("resources")
    }

    /// Load a test image from the resources directory
    fn load_test_image(relative_path: &str) -> Result<MutableImage> {
        let path = test_resources_dir().join(relative_path);
        let img = image::open(&path)?;
        MutableImage::from_dynamic_image(img)
    }

    // fn store_test_image(img: MutableImage, relative_path: &str) {
    //     let path = test_resources_dir().join(relative_path);
    //     img.save(&*path).unwrap();
    // }

    // Helper function to create a test image with specified dimensions and color
    fn create_test_image(width: usize, height: usize, color: Rgb<u8>) -> MutableImage {
        let mut buffer = Vec::with_capacity(width * height * 3);
        for _ in 0..(width * height) {
            buffer.extend_from_slice(&[color.0[0], color.0[1], color.0[2]]);
        }

        MutableImage::new_from_buffer(width, height, buffer.into_boxed_slice()).unwrap()
    }

    #[test]
    fn test_image_overlay() {
        // Create a black background image
        let mut background = create_test_image(100, 100, Rgb([0, 0, 0]));

        // Create a white overlay image
        let overlay = create_test_image(50, 50, Rgb([255, 255, 255]));

        // Overlay at position (25, 25)
        background.overlay(&overlay.into_image().unwrap(), 25, 25);

        let test_image = load_test_image("output/white_square_on_black_background.png").unwrap();
        assert_eq!(background, test_image);
    }

    #[test]
    fn test_image_overlay_bounds_check() {
        let mut background = create_test_image(100, 100, Rgb([0, 0, 0]));
        let overlay = create_test_image(100, 100, Rgb([255, 255, 255]));

        background.overlay(&overlay.into_image().unwrap(), 80, 80);

        let test_image = load_test_image("output/out_of_bounds.png").unwrap();
        assert_eq!(background, test_image);
    }

    #[test]
    fn test_overlay_complex_image() {
        let mut background = create_test_image(3000, 3000, Rgb([0, 0, 0]));
        let overlay_image = load_test_image("input/dogs.png").unwrap();

        background.overlay(&overlay_image.into_image().unwrap(), 500, 800);

        let test_image = load_test_image("output/dogs_on_black.png").unwrap();
        assert_eq!(background, test_image);
    }
}
