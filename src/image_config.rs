use image::DynamicImage;

#[derive(serde::Deserialize, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct ImageConfig {
    max_width: Option<u32>,
    max_height: Option<u32>,
}

impl ImageConfig {
    pub fn treat_image(&self, image: DynamicImage) -> DynamicImage {
        image.thumbnail(
            self.max_width.unwrap_or(u32::MAX),
            self.max_height.unwrap_or(u32::MAX),
        )
    }
}
