use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Deserialize;

#[derive(serde::Deserialize)]
#[serde(default)]
pub struct Config {
    bind_ip: String,
    port: u16,
    root_directory: PathBuf,
    #[serde(deserialize_with = "deserialize_duration")]
    pub memory_cache_duration: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub disk_cache_initial_duration: Duration,
    pub disk_cache_multiplier: f32,
    #[serde(deserialize_with = "deserialize_duration")]
    pub disk_cache_max_duration: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".into(),
            port: 3000,
            root_directory: PathBuf::from("/usr/share/potassium-shot-images/images"),
            memory_cache_duration: Duration::from_mins(10),
            disk_cache_initial_duration: Duration::from_mins(10),
            disk_cache_multiplier: 4.0,
            disk_cache_max_duration: Duration::from_hours(72),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(&*crate::env::CONFIG_PATH)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|e| {
            tracing::error!("Couldn't load config file, using default config: {}", e);
            Self::default()
        })
    }

    pub fn get_bind_ip(&self) -> &str {
        crate::env::BIND_IP
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(&self.bind_ip)
    }

    pub fn get_port(&self) -> u16 {
        crate::env::PORT.unwrap_or(self.port)
    }

    pub fn get_root_directory(&self) -> &Path {
        crate::env::IMAGES_DIRECTORY
            .as_ref()
            .map(|p| p.as_path())
            .unwrap_or(&self.root_directory)
    }
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    let s = String::deserialize(deserializer)?;
    parse_duration::parse(s.as_str()).map_err(serde::de::Error::custom)
}
