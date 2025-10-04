use anyhow::Result;
use transfer_objects::image::Image;

#[derive(Debug, PartialEq, Eq)]
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

    pub fn get_rgb_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn into_immutable(self) -> Result<Image> {
        Image::new_from_buffer(self.width, self.height, self.buffer)
    }
}
