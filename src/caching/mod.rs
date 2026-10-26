use std::time::Instant;

use dashmap::DashMap;
use image::DynamicImage;

use crate::{CONFIG, image_config::ImageConfig};

mod disk_cache;

#[derive(Default)]
pub struct ImageCache {
    memory_cache: DashMap<ImageConfig, MemoryCachedImage>,
    disk_cache: disk_cache::DiskCache,
}

impl ImageCache {
    pub fn get_cached(&self, config: &ImageConfig) -> Option<DynamicImage> {
        if let Some(mut cached_image) = self.memory_cache.get_mut(config) {
            cached_image.time = Instant::now();
            self.disk_cache.refresh_time(config);
            Some(cached_image.image.clone())
        } else if let Some(cached_image) = self.disk_cache.get_cached(config) {
            self.put_memory_only(config.clone(), &cached_image);
            Some(cached_image)
        } else {
            None
        }
    }

    pub fn put(&self, config: ImageConfig, image: &DynamicImage) {
        self.put_memory_only(config.clone(), image);
        self.disk_cache.put(config, image);
    }

    fn put_memory_only(&self, config: ImageConfig, image: &DynamicImage) {
        self.memory_cache.insert(
            config.clone(),
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
