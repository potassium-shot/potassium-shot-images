use std::str::FromStr;

use image::DynamicImage;
use serde::Deserialize;

#[derive(serde::Serialize, serde::Deserialize, Default, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(default)]
pub struct ImageConfig {
    #[serde(deserialize_with = "deserialize_option_workaround")]
    max_width: Option<u32>,
    #[serde(deserialize_with = "deserialize_option_workaround")]
    max_height: Option<u32>,
}

impl ImageConfig {
    pub fn treat_image(&self, mut image: DynamicImage) -> DynamicImage {
        if self.max_width.is_some() || self.max_height.is_some() {
            image = image.thumbnail(
                self.max_width.unwrap_or(image.width()),
                self.max_height.unwrap_or(image.height()),
            )
        }

        image
    }
}

fn deserialize_option_workaround<'de, T: FromStr<Err = E>, E: std::fmt::Display, D>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Some(T::from_str(&s).map_err(serde::de::Error::custom)?))
}
