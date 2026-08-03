use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use iced::widget::image;
use crate::engine::traits::{DocumentBackend, RenderQuality, TocItem};
use crate::models::session::{PageLayout, SidePanelTab};

const MAX_THUMBNAIL_CACHE_SIZE: usize = 150;
const TAB_TEXTURE_RAM_BUDGET_BYTES: usize = 60 * 1024 * 1024; // 60 MB budget for continuous prefetching

#[derive(Clone)]
pub struct CachedTexture {
    pub handle: image::Handle,
    pub quality: RenderQuality,
    pub zoom: f32,
    pub width: u32,
    pub height: u32,
}

impl CachedTexture {
    pub fn memory_size_bytes(&self) -> usize {
        (self.width * self.height * 4) as usize
    }
}

pub struct RuntimeTab {
    pub id: usize,
    pub title: String,
    pub file_path: PathBuf,
    pub backend: Arc<dyn DocumentBackend>,
    
    pub page_count: usize,
    pub toc: Vec<TocItem>,

    pub current_page: usize,
    pub zoom: f32,

    pub layout: PageLayout,
    pub is_continuous: bool,

    pub is_side_panel_open: bool,
    pub is_side_panel_pinned: bool,
    pub side_panel_tab: SidePanelTab,

    pub scroll_sequence: usize,
    pub zoom_sequence: usize,

    pub texture_cache: HashMap<usize, CachedTexture>,
    pub loading_pages: HashSet<usize>,

    pub thumbnail_cache: HashMap<usize, image::Handle>,
    pub thumbnail_lru_order: VecDeque<usize>,
    pub loading_thumbnails: HashSet<usize>,
}

impl RuntimeTab {
    pub fn new(
        id: usize,
        title: String,
        file_path: PathBuf,
        backend: Arc<dyn DocumentBackend>,
        current_page: usize,
        zoom: f32,
        layout: PageLayout,
        is_continuous: bool,
        is_side_panel_open: bool,
        is_side_panel_pinned: bool,
        side_panel_tab: SidePanelTab,
    ) -> Self {
        let page_count = backend.page_count();
        let toc = backend.table_of_contents();

        Self {
            id,
            title,
            file_path,
            backend,
            page_count,
            toc,
            current_page,
            zoom,
            layout,
            is_continuous,
            is_side_panel_open,
            is_side_panel_pinned,
            side_panel_tab,
            scroll_sequence: 0,
            zoom_sequence: 0,
            texture_cache: HashMap::new(),
            loading_pages: HashSet::new(),
            thumbnail_cache: HashMap::new(),
            thumbnail_lru_order: VecDeque::new(),
            loading_thumbnails: HashSet::new(),
        }
    }

    /// Calculates the exact page index visible at a given vertical pixel offset
    pub fn page_at_y_offset(&self, offset_y: f32) -> usize {
        if self.page_count == 0 {
            return 0;
        }

        let spacing = 12.0;
        let mut accumulated_y = 0.0;

        if self.layout == PageLayout::Double && self.is_continuous {
            let total_rows = (self.page_count + 1) / 2;
            for row_idx in 0..total_rows {
                let left_page = row_idx * 2;
                let right_page = left_page + 1;

                let (_, left_h) = self.backend.page_dimensions(left_page);
                let right_h = if right_page < self.page_count {
                    let (_, rh) = self.backend.page_dimensions(right_page);
                    rh
                } else {
                    0.0
                };

                let row_h = left_h.max(right_h) * self.zoom + spacing;
                if accumulated_y + (row_h * 0.7) >= offset_y {
                    return (row_idx * 2).min(self.page_count - 1);
                }
                accumulated_y += row_h;
            }
            (total_rows - 1) * 2
        } else {
            for page_idx in 0..self.page_count {
                let (_, doc_h) = self.backend.page_dimensions(page_idx);
                let page_h = doc_h * self.zoom + spacing;
                if accumulated_y + (page_h * 0.7) >= offset_y {
                    return page_idx;
                }
                accumulated_y += page_h;
            }
            self.page_count.saturating_sub(1)
        }
    }

    /// Calculates the exact pixel Y-coordinate required to scroll to a specific page
    pub fn y_offset_for_page(&self, page_index: usize) -> f32 {
        if self.page_count == 0 {
            return 0.0;
        }

        let spacing = 12.0;
        let mut accumulated_y = 0.0;

        if self.layout == PageLayout::Double && self.is_continuous {
            let target_row = page_index / 2;
            for row_idx in 0..target_row {
                let left_page = row_idx * 2;
                let right_page = left_page + 1;

                let (_, left_h) = self.backend.page_dimensions(left_page);
                let right_h = if right_page < self.page_count {
                    let (_, rh) = self.backend.page_dimensions(right_page);
                    rh
                } else {
                    0.0
                };

                let row_h = left_h.max(right_h) * self.zoom + spacing;
                accumulated_y += row_h;
            }
        } else {
            let target = page_index.min(self.page_count.saturating_sub(1));
            for idx in 0..target {
                let (_, doc_h) = self.backend.page_dimensions(idx);
                accumulated_y += doc_h * self.zoom + spacing;
            }
        }

        accumulated_y
    }

    pub fn total_texture_ram_bytes(&self) -> usize {
        self.texture_cache.values().map(|c| c.memory_size_bytes()).sum()
    }

    pub fn enforce_memory_budget(&mut self) {
        let required = self.required_pages();

        while self.total_texture_ram_bytes() > TAB_TEXTURE_RAM_BUDGET_BYTES {
            let candidate = self.texture_cache.keys()
                .filter(|&&idx| !required.contains(&idx))
                .cloned()
                .max_by_key(|&idx| (idx as isize - self.current_page as isize).abs());

            if let Some(evict_idx) = candidate {
                self.texture_cache.remove(&evict_idx);
            } else {
                break;
            }
        }
    }

    pub fn purge_inactive_cache(&mut self) {
        self.texture_cache.retain(|&page_idx, _| {
            page_idx == self.current_page || page_idx == self.current_page.saturating_sub(1) || page_idx == self.current_page + 1
        });
        self.loading_thumbnails.clear();
        self.loading_pages.clear();
    }

    pub fn insert_texture(&mut self, page_index: usize, handle: image::Handle, quality: RenderQuality) {
        let (w, h) = match quality {
            RenderQuality::Fuzzy => (300, 400),
            RenderQuality::Draft => (1000, 1330),
            RenderQuality::High => (2000, 2660),
        };
        let current_zoom = self.zoom;
        self.insert_texture_with_size(page_index, handle, quality, current_zoom, w, h);
    }

    pub fn insert_texture_with_size(
        &mut self,
        page_index: usize,
        handle: image::Handle,
        quality: RenderQuality,
        zoom: f32,
        width: u32,
        height: u32,
    ) {
        if let Some(existing) = self.texture_cache.get(&page_index) {
            if (existing.zoom - zoom).abs() < 0.01 && existing.quality >= quality {
                return;
            }
        }

        self.texture_cache.insert(page_index, CachedTexture { handle, quality, zoom, width, height });
        self.enforce_memory_budget();
    }

    pub fn insert_thumbnail(&mut self, page_index: usize, handle: image::Handle) {
        if self.thumbnail_cache.contains_key(&page_index) {
            self.thumbnail_lru_order.retain(|&idx| idx != page_index);
        } else if self.thumbnail_cache.len() >= MAX_THUMBNAIL_CACHE_SIZE {
            if let Some(oldest_thumb) = self.thumbnail_lru_order.pop_front() {
                self.thumbnail_cache.remove(&oldest_thumb);
            }
        }

        self.thumbnail_cache.insert(page_index, handle);
        self.thumbnail_lru_order.push_back(page_index);
    }

    pub fn get_texture(&self, page_index: usize) -> Option<&image::Handle> {
        self.texture_cache.get(&page_index).map(|c| &c.handle)
    }

    pub fn required_pages(&self) -> Vec<usize> {
        self.required_pages_with_quality()
            .into_iter()
            .map(|(page, _)| page)
            .collect()
    }

    pub fn required_pages_with_quality(&self) -> Vec<(usize, RenderQuality)> {
        let total = self.page_count;
        if total == 0 {
            return Vec::new();
        }

        let mut requests = Vec::new();

        if self.is_continuous {
            // Predictive forward prefetching: 3 pages behind, 8 pages ahead
            let start = self.current_page.saturating_sub(3);
            let end = (self.current_page + 9).min(total);

            for p in start..end {
                let dist = (p as isize - self.current_page as isize).abs();
                let quality = if dist <= 1 {
                    RenderQuality::High
                } else if dist <= 4 {
                    RenderQuality::Draft
                } else {
                    RenderQuality::Fuzzy
                };
                requests.push((p, quality));
            }
        } else if self.layout == PageLayout::Double {
            requests.push((self.current_page, RenderQuality::High));
            if self.current_page + 1 < total {
                requests.push((self.current_page + 1, RenderQuality::High));
            }
            if self.current_page + 2 < total {
                requests.push((self.current_page + 2, RenderQuality::Draft));
            }
            if self.current_page > 0 {
                requests.push((self.current_page - 1, RenderQuality::Draft));
            }
        } else {
            requests.push((self.current_page, RenderQuality::High));
            if self.current_page + 1 < total {
                requests.push((self.current_page + 1, RenderQuality::Draft));
            }
            if self.current_page > 0 {
                requests.push((self.current_page - 1, RenderQuality::Draft));
            }
        }

        requests
    }
}