use std::{
    path::PathBuf,
    sync::LazyLock,
    time::{Duration, SystemTime},
};

use dashmap::DashMap;
use image::DynamicImage;
use tracing::warn;
use uuid::Uuid;

use crate::{CONFIG, caching::ImageCacheKey};

static INDEX_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| CONFIG.get_cache_directory().join("index.bin"));
static IMAGES_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| CONFIG.get_cache_directory().join("images"));

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct DiskCache {
    cached_images: DashMap<ImageCacheKey, DiskCachedImage>,
    eligible_for_time_refresh: DashMap<ImageCacheKey, EligibleForTimeRefresh>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct DiskCachedImage {
    end_time: SystemTime,
    last_duration: Duration,
    id: Uuid,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct EligibleForTimeRefresh {
    end_time: SystemTime,
    last_duration: Duration,
}

impl DiskCache {
    pub fn get_cached(&self, ckey: &ImageCacheKey) -> Option<DynamicImage> {
        let result = if let Some(mut cached_image) = self.cached_images.get_mut(ckey) {
            cached_image.last_duration = (cached_image.last_duration
                * CONFIG.disk_cache_multiplier)
                .min(CONFIG.disk_cache_max_duration);
            cached_image.end_time = SystemTime::now() + cached_image.last_duration;

            match Self::load_image(cached_image.id) {
                Ok(image) => Some(image),
                Err(e) => {
                    tracing::error!(
                        "Couldn't load cached image with id {}: {}",
                        cached_image.id,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        if result.is_some() {
            self.save();
        }

        result
    }

    pub fn refresh_time(&self, ckey: &ImageCacheKey) {
        let mut must_save = false;

        if let Some(mut cached_image) = self.cached_images.get_mut(ckey) {
            cached_image.last_duration = (cached_image.last_duration
                * CONFIG.disk_cache_multiplier)
                .min(CONFIG.disk_cache_max_duration);
            cached_image.end_time = SystemTime::now() + cached_image.last_duration;
            must_save = true;
        }

        if must_save {
            self.save();
        }
    }

    pub fn put(&self, ckey: ImageCacheKey, image: &DynamicImage) {
        let Some(id) = Self::save_image(image) else {
            if let Some(mut time_limit) = self.eligible_for_time_refresh.get_mut(&ckey) {
                time_limit.last_duration = (time_limit.last_duration
                    * CONFIG.disk_cache_multiplier)
                    .min(CONFIG.disk_cache_max_duration);
                time_limit.end_time = SystemTime::now() + time_limit.last_duration;
            }
            self.save();
            return;
        };

        let duration = if let Some((_, time_limit)) = self.eligible_for_time_refresh.remove(&ckey) {
            time_limit.last_duration
        } else {
            CONFIG.disk_cache_initial_duration
        };

        self.cached_images.insert(
            ckey,
            DiskCachedImage {
                end_time: SystemTime::now() + duration,
                last_duration: duration,
                id,
            },
        );

        self.save();
    }

    pub fn update(&self) {
        let mut must_save = false;
        let mut to_remove_eligible = Vec::new();

        for eligible in self.eligible_for_time_refresh.iter() {
            if eligible.end_time < SystemTime::now() {
                to_remove_eligible.push(eligible.key().clone());
            }
        }

        for key in to_remove_eligible {
            self.eligible_for_time_refresh.remove(&key);
            must_save = true;
        }

        let mut to_remove = Vec::new();
        let mut total_size = 0;

        for cached_image in self.cached_images.iter() {
            if cached_image.end_time < SystemTime::now() {
                to_remove.push(cached_image.key().clone());
                self.eligible_for_time_refresh.insert(
                    cached_image.key().clone(),
                    EligibleForTimeRefresh {
                        end_time: SystemTime::now() + CONFIG.disk_cache_max_duration,
                        last_duration: cached_image.last_duration,
                    },
                );
                Self::delete_image(cached_image.id);
            } else {
                match std::fs::metadata(IMAGES_PATH.join(format!("{}.png", cached_image.id))) {
                    Ok(meta) => {
                        total_size += meta.len();
                    }
                    Err(e) => tracing::error!("Couldn't get size of cached image: {}", e),
                }
            }
        }

        for key in to_remove {
            self.cached_images.remove(&key);
            must_save = true;
        }

        if total_size > CONFIG.disk_cache_budget {
            let mut images_by_age = self
                .cached_images
                .iter()
                .map(|entry| (entry.end_time, entry.key().clone()))
                .collect::<Vec<_>>();
            images_by_age.sort_unstable_by_key(|(time, _)| *time);
            images_by_age.reverse();

            while total_size > CONFIG.disk_cache_budget {
                let Some((_, key)) = images_by_age.pop() else {
                    break;
                };

                let Some((_, cached_image)) = self.cached_images.remove(&key) else {
                    continue;
                };

                match std::fs::metadata(IMAGES_PATH.join(format!("{}.png", cached_image.id))) {
                    Ok(meta) => {
                        total_size -= meta.len();
                    }
                    Err(e) => tracing::error!("Couldn't get size of cached image: {}", e),
                }

                Self::delete_image(cached_image.id);
            }
        }

        if must_save {
            self.save();
        }
    }

    fn save(&self) {
        let bytes = postcard::to_stdvec(&self).unwrap();

        if let Err(e) = std::fs::write(&*INDEX_PATH, bytes) {
            tracing::error!("Could not write cache index: {}", e);
        }
    }

    fn load() -> Self {
        fn try_load() -> anyhow::Result<DiskCache> {
            let bytes = std::fs::read(&*INDEX_PATH)?;
            Ok(postcard::from_bytes(&bytes)?)
        }

        match try_load() {
            Ok(dc) => dc,
            Err(e) => {
                warn!(
                    "Disk cache file could no be loaded, using empty cache: {}",
                    e
                );
                Self::clear_images();
                Self {
                    cached_images: DashMap::new(),
                    eligible_for_time_refresh: DashMap::new(),
                }
            }
        }
    }

    fn save_image(image: &DynamicImage) -> Option<Uuid> {
        let id = Uuid::new_v4();

        let try_save_image = || -> anyhow::Result<()> {
            std::fs::create_dir_all(&*IMAGES_PATH)?;
            image.save(IMAGES_PATH.join(format!("{}.png", id)))?;
            Ok(())
        };

        if let Err(e) = try_save_image() {
            tracing::error!("Could not save image to cache: {}", e);
            None
        } else {
            Some(id)
        }
    }

    fn load_image(id: Uuid) -> anyhow::Result<DynamicImage> {
        Ok(image::open(IMAGES_PATH.join(format!("{}.png", id)))?)
    }

    fn delete_image(id: Uuid) {
        if let Err(e) = std::fs::remove_file(IMAGES_PATH.join(format!("{}.png", id))) {
            tracing::error!("Couldn't delete cached image: {}", e);
        }
    }

    fn clear_images() {
        fn try_clear_images() -> anyhow::Result<()> {
            if !std::fs::exists(&*IMAGES_PATH)? {
                return Ok(());
            }

            for image in std::fs::read_dir(&*IMAGES_PATH)? {
                let image = image?;
                let r#type = image.file_type()?;

                if r#type.is_dir() {
                    std::fs::remove_dir_all(image.path())?;
                } else {
                    std::fs::remove_file(image.path())?;
                }
            }

            Ok(())
        }

        if let Err(e) = try_clear_images() {
            tracing::error!("Could not clear the cached images: {}", e);
        }
    }
}

impl Default for DiskCache {
    fn default() -> Self {
        Self::load()
    }
}
