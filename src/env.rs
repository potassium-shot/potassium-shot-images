use std::{path::PathBuf, sync::LazyLock};

pub static BIND_IP: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("POTASSIUM_SHOT_IMAGES_BIND_IP").ok());

pub static PORT: LazyLock<Option<u16>> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
});

pub static IMAGES_DIRECTORY: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_DIRECTORY")
        .map(PathBuf::from)
        .ok()
});

pub static CACHE_DIRECTORY: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_CACHE_DIRECTORY")
        .map(PathBuf::from)
        .ok()
});

pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/usr/share/potassium-shot-images/config.toml"))
});
