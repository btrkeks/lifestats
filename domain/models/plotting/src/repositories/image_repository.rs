use crate::objects::image::Image;
use anyhow::Result;

pub trait ImageRepository {
    fn get_by_id(&self, id: u32) -> Result<Image>;
    fn save(&self, image: Image) -> Result<()>;
}