use image::DynamicImage;

use crate::image_config::ImageConfig;

#[derive(Default)]
pub struct ImageCache {}

impl ImageCache {
    pub fn get_cached(&self, config: &ImageConfig) -> Option<DynamicImage> {
        None
    }

    pub fn put(&self, config: ImageConfig, image: &DynamicImage) {}
}
