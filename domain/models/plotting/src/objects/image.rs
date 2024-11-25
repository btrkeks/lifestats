use std::path::Path;
use image::{ImageBuffer, Rgb};
use anyhow::Result;
use anyhow::Context;

pub struct Image {
    width: usize,
    height: usize,
    buffer: Vec<u8>
}

impl Image {
    pub fn new_with_size(size: (usize, usize)) -> Self {
        Self {
            width: size.0,
            height: size.1,
            buffer: vec![0; size.0 * size.1 * 3] // RGB format
        }
    }

    pub fn get_buffer(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Create an ImageBuffer from our raw pixel data
        let img_buffer = ImageBuffer::<Rgb<u8>, _>::from_raw(
            self.width as u32,
            self.height as u32,
            self.buffer.clone()
        ).context("Failed to create image buffer from raw data")?;

        // Save the image buffer as PNG
        img_buffer.save(path)
            .context("Failed to save image to file")?;

        Ok(())
    }
}