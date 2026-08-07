use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use iced::widget::image::Handle;

#[derive(Debug, Clone)]
pub struct UsefulCacheEntry {
    pub file_path: PathBuf,
    pub current_page: usize,
}

pub struct DiskCache {
    cache_dir: PathBuf,
}

impl DiskCache {
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("verseapp_disk_cache");
        let _ = fs::create_dir_all(&cache_dir);
        let cache = Self { cache_dir };
        cache.purge_old_entries();
        cache
    }

    fn build_path(&self, file_path: &Path, page_index: usize, zoom: f32, suffix: &str) -> PathBuf {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        file_path.hash(&mut hasher);
        let path_hash = hasher.finish();
        let zoom_int = (zoom * 100.0) as u32;

        self.cache_dir.join(format!("{:x}_{}_{}_{}.jpg", path_hash, page_index, zoom_int, suffix))
    }

    pub fn get_page(&self, file_path: &Path, page_index: usize, zoom: f32, suffix: &str) -> Option<(Handle, u32, u32)> {
        let path = self.build_path(file_path, page_index, zoom, suffix);
        if path.exists() {
            if let Ok(img) = image::open(&path) {
                let rgba = img.to_rgba8();
                let w = rgba.width();
                let h = rgba.height();
                let handle = Handle::from_rgba(w, h, rgba.into_raw());
                return Some((handle, w, h));
            }
        }
        None
    }

    pub fn save_page(&self, file_path: &Path, page_index: usize, zoom: f32, suffix: &str, rgba_bytes: &[u8], width: u32, height: u32) {
        let path = self.build_path(file_path, page_index, zoom, suffix);
        if let Some(img_buf) = image::RgbaImage::from_raw(width, height, rgba_bytes.to_vec()) {
            let _ = img_buf.save_with_format(&path, image::ImageFormat::Jpeg);
        }
    }

    /// Retains only cache files belonging to open tabs within a +/- 3 page window of `current_page`.
    /// Automatically deletes all other temporary data when app closes / session saves.
    pub fn retain_useful_cache(&self, active_tabs: &[UsefulCacheEntry]) {
        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            let mut allowed_hashes: HashMap<u64, (usize, usize)> = HashMap::new();
            for tab in active_tabs {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                tab.file_path.hash(&mut hasher);
                let file_hash = hasher.finish();

                let min_page = tab.current_page.saturating_sub(2);
                let max_page = tab.current_page + 3;
                allowed_hashes.insert(file_hash, (min_page, max_page));
            }

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }

                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let parts: Vec<&str> = filename.split('_').collect();
                    if parts.len() >= 3 {
                        let hash_str = parts[0];
                        let page_str = parts[1];

                        let hash_matches = u64::from_str_radix(hash_str, 16).ok();
                        let page_num = page_str.parse::<usize>().ok();

                        if let (Some(hash_val), Some(page_idx)) = (hash_matches, page_num) {
                            if let Some(&(min_p, max_p)) = allowed_hashes.get(&hash_val) {
                                if page_idx >= min_p && page_idx <= max_p {
                                    // Useful entry for active tab -> KEEP
                                    continue;
                                }
                            }
                        }
                    }
                }

                // Temporary or stale cache entry -> DELETE
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// Purges all cached files for a specific file when its tab is closed.
    pub fn remove_for_file(&self, file_path: &Path) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        file_path.hash(&mut hasher);
        let target_hash = hasher.finish();
        let prefix = format!("{:x}_", target_hash);

        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.starts_with(&prefix) {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }

    pub fn purge_old_entries(&self) {
        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            let mut files: Vec<(PathBuf, u64)> = entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    if meta.is_file() {
                        Some((e.path(), meta.len()))
                    } else {
                        None
                    }
                })
                .collect();

            let total_size: u64 = files.iter().map(|(_, sz)| sz).sum();
            const MAX_DISK_BYTES: u64 = 150 * 1024 * 1024;

            if total_size > MAX_DISK_BYTES {
                files.sort_by_key(|(p, _)| {
                    fs::metadata(p).and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });

                let mut current_size = total_size;
                for (p, sz) in files {
                    if current_size <= MAX_DISK_BYTES { break; }
                    if fs::remove_file(&p).is_ok() {
                        current_size = current_size.saturating_sub(sz);
                    }
                }
            }
        }
    }
}