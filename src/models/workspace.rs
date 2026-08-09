use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use iced::widget::image;
use crate::engine::traits::{DocumentBackend, RenderQuality, TextQuad, TocItem};
use crate::models::session::{PageLayout, SidePanelTab};

const MAX_THUMBNAIL_CACHE_SIZE: usize = 30;
const TAB_TEXTURE_RAM_BUDGET_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub page_index: usize,
    pub quad: TextQuad,
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn malloc_trim(pad: usize) -> std::ffi::c_int;
}

pub fn trim_memory() {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            malloc_trim(0);
        }
    }
}

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
    pub page_input_text: String,
    pub zoom: f32,
    pub viewport_y: f32,
    pub is_zooming: bool,

    pub layout: PageLayout,
    pub is_continuous: bool,

    pub is_side_panel_open: bool,
    pub is_side_panel_pinned: bool,
    pub side_panel_tab: SidePanelTab,
    pub side_panel_scroll_offset: f32,

    pub scroll_sequence: usize,
    pub zoom_sequence: usize,
    pub last_accessed: Instant,

    pub texture_cache: HashMap<usize, CachedTexture>,
    pub loading_pages: HashSet<usize>,

    pub thumbnail_cache: HashMap<usize, image::Handle>,
    pub thumbnail_lru_order: VecDeque<usize>,
    pub loading_thumbnails: HashSet<usize>,

    pub text_cache: HashMap<usize, Vec<TextQuad>>,
    pub selection_start: Option<(usize, f32, f32)>,
    pub selection_end: Option<(usize, f32, f32)>,
    pub is_selecting: bool,
    pub selected_text: String,

    pub is_search_open: bool,
    pub search_query: String,
    pub search_match_case: bool,
    pub search_matches: Vec<SearchMatch>,
    pub current_search_idx: usize,
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
        let page_input_text = (current_page + 1).to_string();

        Self {
            id,
            title,
            file_path,
            backend,
            page_count,
            toc,
            current_page,
            page_input_text,
            zoom,
            viewport_y: 0.0,
            is_zooming: false,
            layout,
            is_continuous,
            is_side_panel_open,
            is_side_panel_pinned,
            side_panel_tab,
            side_panel_scroll_offset: 0.0,
            scroll_sequence: 0,
            zoom_sequence: 0,
            last_accessed: Instant::now(),
            texture_cache: HashMap::new(),
            loading_pages: HashSet::new(),
            thumbnail_cache: HashMap::new(),
            thumbnail_lru_order: VecDeque::new(),
            loading_thumbnails: HashSet::new(),
            text_cache: HashMap::new(),
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            selected_text: String::new(),
            is_search_open: false,
            search_query: String::new(),
            search_match_case: false,
            search_matches: Vec::new(),
            current_search_idx: 0,
        }
    }

    pub fn perform_search(&mut self) {
        self.search_matches.clear();
        self.current_search_idx = 0;

        let query = self.search_query.trim();
        if query.is_empty() {
            return;
        }

        let query_cmp = if self.search_match_case {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        for page_idx in 0..self.page_count {
            let quads = self.get_text_quads(page_idx).to_vec();
            for quad in quads {
                let quad_text = if self.search_match_case {
                    quad.text.clone()
                } else {
                    quad.text.to_lowercase()
                };

                if quad_text.contains(&query_cmp) {
                    self.search_matches.push(SearchMatch {
                        page_index: page_idx,
                        quad,
                    });
                }
            }
        }
    }

    pub fn get_search_matches_for_page(&mut self, page_index: usize) -> Vec<TextQuad> {
        self.search_matches
            .iter()
            .filter(|m| m.page_index == page_index)
            .map(|m| m.quad.clone())
            .collect()
    }

    pub fn get_active_search_match_for_page(&self, page_index: usize) -> Option<TextQuad> {
        if self.search_matches.is_empty() {
            return None;
        }
        let active_match = &self.search_matches[self.current_search_idx.min(self.search_matches.len() - 1)];
        if active_match.page_index == page_index {
            Some(active_match.quad.clone())
        } else {
            None
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    pub fn retain_only_current_page(&mut self) {
        let curr = self.current_page;
        self.texture_cache.retain(|&idx, _| idx == curr);
        self.thumbnail_cache.clear();
        self.loading_pages.clear();
        self.loading_thumbnails.clear();
    }

    pub fn offload_completely_from_ram(&mut self) {
        self.texture_cache.clear();
        self.thumbnail_cache.clear();
        self.loading_pages.clear();
        self.loading_thumbnails.clear();
    }

    pub fn get_text_quads(&mut self, page_index: usize) -> &[TextQuad] {
        if !self.text_cache.contains_key(&page_index) {
            let quads = self.backend.extract_text(page_index);
            self.text_cache.insert(page_index, quads);
        }
        self.text_cache.get(&page_index).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn start_selection(&mut self, page_index: usize, x: f32, y: f32) {
        self.selection_start = Some((page_index, x, y));
        self.selection_end = Some((page_index, x, y));
        self.is_selecting = true;
        self.update_selected_text();
    }

    pub fn update_selection(&mut self, page_index: usize, x: f32, y: f32) {
        if self.is_selecting || self.selection_start.is_some() {
            self.selection_end = Some((page_index, x, y));
            self.update_selected_text();
        }
    }

    pub fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
        self.selected_text.clear();
    }

    pub fn update_selected_text(&mut self) {
        if let (Some((p1, x1, y1)), Some((p2, x2, y2))) = (self.selection_start, self.selection_end) {
            if p1 != p2 {
                self.selected_text.clear();
                return;
            }

            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);

            let quads = self.get_text_quads(p1).to_vec();
            let mut matched = Vec::new();

            for quad in quads {
                let vertically_aligned = quad.y1 >= min_y && quad.y0 <= max_y;
                if vertically_aligned {
                    let is_first_line = quad.y0 <= min_y + 12.0;
                    let is_last_line = quad.y1 >= max_y - 12.0;

                    let horizontally_valid = if is_first_line && is_last_line {
                        quad.x1 >= min_x && quad.x0 <= max_x
                    } else if is_first_line {
                        quad.x1 >= min_x
                    } else if is_last_line {
                        quad.x0 <= max_x
                    } else {
                        true
                    };

                    if horizontally_valid {
                        matched.push(quad.text.clone());
                    }
                }
            }

            self.selected_text = matched.join(" ");
        } else {
            self.selected_text.clear();
        }
    }

    pub fn get_selected_quads_for_page(&mut self, page_index: usize) -> Vec<TextQuad> {
        if let (Some((p1, x1, y1)), Some((p2, x2, y2))) = (self.selection_start, self.selection_end) {
            if p1 != page_index || p2 != page_index {
                return Vec::new();
            }

            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);

            let quads = self.get_text_quads(page_index).to_vec();
            quads.into_iter()
                .filter(|quad| {
                    let vertically_aligned = quad.y1 >= min_y && quad.y0 <= max_y;
                    if !vertically_aligned {
                        return false;
                    }

                    let is_first_line = quad.y0 <= min_y + 12.0;
                    let is_last_line = quad.y1 >= max_y - 12.0;

                    if is_first_line && is_last_line {
                        quad.x1 >= min_x && quad.x0 <= max_x
                    } else if is_first_line {
                        quad.x1 >= min_x
                    } else if is_last_line {
                        quad.x0 <= max_x
                    } else {
                        true
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn side_panel_thumb_y(&self, page_index: usize) -> f32 {
        page_index as f32 * 210.0
    }

    pub fn page_at_y_offset(&self, offset_y: f32) -> usize {
        if self.page_count == 0 || !offset_y.is_finite() || offset_y <= 0.0 {
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
        let dynamic_budget = (TAB_TEXTURE_RAM_BUDGET_BYTES as f32 * (self.zoom * self.zoom).max(1.0)) as usize;

        while self.total_texture_ram_bytes() > dynamic_budget {
            let candidate = self.texture_cache.keys()
                .filter(|&&idx| idx != self.current_page && !required.contains(&idx))
                .cloned()
                .max_by_key(|&idx| (idx as isize - self.current_page as isize).abs());

            if let Some(evict_idx) = candidate {
                self.texture_cache.remove(&evict_idx);
            } else {
                break;
            }
        }
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
            let start = self.current_page.saturating_sub(2);
            let end = (self.current_page + 6).min(total);

            for p in start..end {
                let dist = (p as isize - self.current_page as isize).abs();
                let quality = if dist <= 1 {
                    RenderQuality::High
                } else if dist <= 3 {
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
                requests.push((self.current_page + 2, RenderQuality::High));
            }
            if self.current_page + 3 < total {
                requests.push((self.current_page + 3, RenderQuality::High));
            }
            if self.current_page + 4 < total {
                requests.push((self.current_page + 4, RenderQuality::Draft));
            }
            if self.current_page + 5 < total {
                requests.push((self.current_page + 5, RenderQuality::Draft));
            }

            if self.current_page >= 2 {
                requests.push((self.current_page - 2, RenderQuality::Draft));
                requests.push((self.current_page - 1, RenderQuality::Draft));
            }
        } else {
            requests.push((self.current_page, RenderQuality::High));

            if self.current_page + 1 < total {
                requests.push((self.current_page + 1, RenderQuality::High));
            }
            if self.current_page + 2 < total {
                requests.push((self.current_page + 2, RenderQuality::Draft));
            }
            if self.current_page + 3 < total {
                requests.push((self.current_page + 3, RenderQuality::Draft));
            }

            if self.current_page > 0 {
                requests.push((self.current_page - 1, RenderQuality::Draft));
            }
            if self.current_page > 1 {
                requests.push((self.current_page - 2, RenderQuality::Draft));
            }
        }

        requests
    }
}