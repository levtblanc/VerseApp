use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
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

        // Include file modification time and size to auto-invalidate cache on file edit
        if let Ok(metadata) = fs::metadata(file_path) {
            metadata.len().hash(&mut hasher);
            if let Ok(mtime) = metadata.modified() {
                mtime.hash(&mut hasher);
            }
        }

        let path_hash = hasher.finish();
        let zoom_int = (zoom * 100.0) as u32;

        self.cache_dir.join(format!("{:x}_{}_{}_{}.raw", path_hash, page_index, zoom_int, suffix))
    }

    /// Reads raw binary RGBA cache from SSD (<1ms execution time, zero CPU image decoding)
    pub fn get_page(&self, file_path: &Path, page_index: usize, zoom: f32, suffix: &str) -> Option<(Handle, u32, u32)> {
        let path = self.build_path(file_path, page_index, zoom, suffix);
        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut header = [0u8; 8];
                if file.read_exact(&mut header).is_ok() {
                    let w = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                    let h = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

                    let expected_bytes = (w * h * 4) as usize;
                    let mut rgba_bytes = vec![0u8; expected_bytes];

                    if file.read_exact(&mut rgba_bytes).is_ok() {
                        let handle = Handle::from_rgba(w, h, rgba_bytes);
                        return Some((handle, w, h));
                    }
                }
            }
        }
        None
    }

    /// Writes raw RGBA bytes directly to disk (<1ms execution time, zero CPU image encoding)
    pub fn save_page(&self, file_path: &Path, page_index: usize, zoom: f32, suffix: &str, rgba_bytes: &[u8], width: u32, height: u32) {
        let path = self.build_path(file_path, page_index, zoom, suffix);
        if let Ok(mut file) = File::create(&path) {
            let mut header = [0u8; 8];
            header[0..4].copy_from_slice(&width.to_le_bytes());
            header[4..8].copy_from_slice(&height.to_le_bytes());

            let _ = file.write_all(&header);
            let _ = file.write_all(rgba_bytes);
        }
    }

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
                                    continue;
                                }
                            }
                        }
                    }
                }

                let _ = fs::remove_file(&path);
            }
        }
    }

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
            const MAX_DISK_BYTES: u64 = 200 * 1024 * 1024;

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
