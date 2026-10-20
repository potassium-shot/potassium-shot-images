use std::{path::PathBuf, sync::LazyLock};

pub static BIND_IP: LazyLock<String> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_BIND_IP").unwrap_or_else(|_| "0.0.0.0".into())
});

pub static PORT: LazyLock<u16> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_PORT")
        .map_err(anyhow::Error::from)
        .and_then(|v| v.parse::<u16>().map_err(anyhow::Error::from))
        .unwrap_or(3000)
});

pub static IMAGES_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var("POTASSIUM_SHOT_IMAGES_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/usr/share/potassium-shot-images/images"))
});
