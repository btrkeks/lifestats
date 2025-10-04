use anyhow::Result;

#[derive(Debug, PartialEq, Eq)]
pub struct Image {
    width: usize,
    height: usize,
    buffer: Box<[u8]>, // RGB format
}

impl Image {
    pub fn new_from_buffer(width: usize, height: usize, buffer: Box<[u8]>) -> Result<Image> {
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
        let buffer: Box<[u8]> = img.into_rgb8().into_raw().into_boxed_slice();

        Ok(Self {
            width,
            height,
            buffer,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn get_rgb_buffer(&self) -> &[u8] {
        &self.buffer
    }
}
