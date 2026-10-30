use std::{path::PathBuf, time::Instant};

use dashmap::DashMap;
use image::DynamicImage;

use crate::{CONFIG, image_config::ImageConfig};

mod disk_cache;

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
struct ImageCacheKey {
    path: PathBuf,
    config: ImageConfig,
}

#[derive(Default)]
pub struct ImageCache {
    memory_cache: DashMap<ImageCacheKey, MemoryCachedImage>,
    disk_cache: disk_cache::DiskCache,
}

impl ImageCache {
    pub fn get_cached(&self, path: PathBuf, config: &ImageConfig) -> Option<DynamicImage> {
        let ckey = ImageCacheKey {
            path,
            config: config.clone(),
        };
        if let Some(mut cached_image) = self.memory_cache.get_mut(&ckey) {
            cached_image.time = Instant::now();
            self.disk_cache.refresh_time(&ckey);
            Some(cached_image.image.clone())
        } else if let Some(cached_image) = self.disk_cache.get_cached(&ckey) {
            self.put_memory_only(ckey, &cached_image);
            Some(cached_image)
        } else {
            None
        }
    }

    pub fn put(&self, path: PathBuf, config: ImageConfig, image: &DynamicImage) {
        let ckey = ImageCacheKey {
            path,
            config: config.clone(),
        };
        self.put_memory_only(ckey.clone(), image);
        self.disk_cache.put(ckey, image);
    }

    fn put_memory_only(&self, ckey: ImageCacheKey, image: &DynamicImage) {
        self.memory_cache.insert(
            ckey,
            MemoryCachedImage {
                time: Instant::now(),
                image: image.clone(),
            },
        );
    }

    pub fn update(&self) {
        let mut to_remove = Vec::new();
        let mut total_size = 0;

        for cached_image in self.memory_cache.iter() {
            if cached_image.time + CONFIG.memory_cache_duration < Instant::now() {
                to_remove.push(cached_image.key().clone());
            } else {
                total_size += cached_image.image.as_bytes().len();
            }
        }

        for key in to_remove {
            self.memory_cache.remove(&key);
        }

        if total_size as u64 > CONFIG.memory_cache_budget {
            let mut images_by_age = self
                .memory_cache
                .iter()
                .map(|entry| (entry.time, entry.key().clone()))
                .collect::<Vec<_>>();
            images_by_age.sort_unstable_by_key(|(time, _)| *time);
            images_by_age.reverse();

            while total_size as u64 > CONFIG.memory_cache_budget {
                let Some((_, key)) = images_by_age.pop() else {
                    break;
                };

                let Some((_, image)) = self.memory_cache.remove(&key) else {
                    continue;
                };
                total_size -= image.image.as_bytes().len();
            }
        }

        self.disk_cache.update();
    }
}

struct MemoryCachedImage {
    time: Instant,
    image: DynamicImage,
}
