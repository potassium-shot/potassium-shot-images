use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Deserialize;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    bind_ip: String,
    port: u16,
    images_directory: PathBuf,
    cache_directory: PathBuf,
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub memory_cache_duration: Duration,
    #[serde(
        serialize_with = "serialize_size",
        deserialize_with = "deserialize_size"
    )]
    pub memory_cache_budget: u64,
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub disk_cache_initial_duration: Duration,
    pub disk_cache_multiplier: u32,
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub disk_cache_max_duration: Duration,
    #[serde(
        serialize_with = "serialize_size",
        deserialize_with = "deserialize_size"
    )]
    pub disk_cache_budget: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_ip: "0.0.0.0".into(),
            port: 3000,
            images_directory: PathBuf::from("/usr/share/potassium-shot-images/images"),
            cache_directory: PathBuf::from("/usr/share/potassium-shot-images/cache"),
            memory_cache_duration: Duration::from_mins(10),
            memory_cache_budget: 32 * 1024 * 1024,
            disk_cache_initial_duration: Duration::from_mins(10),
            disk_cache_multiplier: 4,
            disk_cache_max_duration: Duration::from_hours(72),
            disk_cache_budget: 512 * 1024 * 1024,
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
            tracing::warn!("Couldn't load config file, writing default config: {}", e);
            let def = Self::default();

            let try_write = || -> anyhow::Result<()> {
                let s = toml::to_string_pretty(&def)?;

                if std::fs::exists(&*crate::env::CONFIG_PATH).is_ok_and(std::convert::identity) {
                    let mut bak_name = crate::env::CONFIG_PATH.clone();
                    bak_name.add_extension(".bak");

                    match std::fs::rename(&*crate::env::CONFIG_PATH, bak_name) {
                        Ok(_) => tracing::warn!("Config file already existed, and was backed up."),
                        Err(e) => tracing::error!("Could not back up existing config file: {}", e),
                    }
                }

                std::fs::write(&*crate::env::CONFIG_PATH, s)?;
                Ok(())
            };

            if let Err(e) = try_write() {
                tracing::error!("Couldn't write default config: {}", e);
            }

            def
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

    pub fn get_images_directory(&self) -> &Path {
        crate::env::IMAGES_DIRECTORY
            .as_ref()
            .map(|p| p.as_path())
            .unwrap_or(&self.images_directory)
    }

    pub fn get_cache_directory(&self) -> &Path {
        crate::env::CACHE_DIRECTORY
            .as_ref()
            .map(|p| p.as_path())
            .unwrap_or(&self.cache_directory)
    }
}

fn serialize_duration<S>(dur: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(format!("{}m", dur.as_secs() / 60).as_str())
}

fn serialize_size<S>(size: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(format!("{}MB", size / 1_048_576).as_str())
}

fn deserialize_duration<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Duration, D::Error> {
    let s = String::deserialize(deserializer)?;
    parse_duration::parse(s.as_str()).map_err(serde::de::Error::custom)
}

fn deserialize_size<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let s = String::deserialize(deserializer)?;
    parse_size::parse_size(s.as_str()).map_err(serde::de::Error::custom)
}
