use iced::Task;
use iced::widget::image::Handle;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality};
use crate::models::disk_cache::DiskCache;
use crate::models::workspace::SearchMatch;
use std::sync::atomic::{AtomicBool, Ordering};

static GLOBAL_DISK_CACHE: LazyLock<DiskCache> = LazyLock::new(DiskCache::new);
const MAX_CONCURRENT_THUMBNAILS: usize = 4;

pub fn get_disk_cache() -> &'static DiskCache {
    &GLOBAL_DISK_CACHE
}

pub fn spawn_search_task(
    tab_id: usize,
    backend: Arc<dyn DocumentBackend>,
    page_count: usize,
    query: String,
    match_case: bool,
    cancel_token: Arc<AtomicBool>,
) -> Task<Message> {
    Task::perform(
        async move {
            let query_trimmed = query.trim().to_string();
            if query_trimmed.is_empty() {
                return (tab_id, query_trimmed, Vec::new());
            }

            let query_for_error = query_trimmed.clone();
            let token = cancel_token.clone();

            tokio::task::spawn_blocking(move || {
                let mut matches = Vec::new();
                let query_cmp = if match_case {
                    query_trimmed.clone()
                } else {
                    query_trimmed.to_lowercase()
                };

                for page_idx in 0..page_count {
                    // Check if a newer search query canceled this task
                    if token.load(Ordering::Relaxed) {
                        return (tab_id, query_trimmed, Vec::new());
                    }

                    let quads = backend.extract_text(page_idx);
                    for quad in quads {
                        let quad_text = if match_case {
                            quad.text.clone()
                        } else {
                            quad.text.to_lowercase()
                        };

                        if quad_text.contains(&query_cmp) {
                            matches.push(SearchMatch {
                                page_index: page_idx,
                                quad,
                            });
                        }
                    }
                }
                (tab_id, query_trimmed, matches)
            })
            .await
            .unwrap_or_else(|_| (tab_id, query_for_error, Vec::new()))
        },
        |(tab_id, query, matches)| Message::SearchCompleted { tab_id, query, matches },
    )
}

pub fn spawn_render_task(
    tab_id: usize,
    file_path: PathBuf,
    page_index: usize,
    zoom: f32,
    quality: RenderQuality,
    is_night_mode: bool,
    backend: Arc<dyn DocumentBackend>,
) -> Task<Message> {
    Task::perform(
        async move {
            let is_image_doc = backend.is_image_based();
            let prefix = if is_night_mode && !is_image_doc { "night_" } else { "" };
            let quality_tag = format!("{}{}", prefix, match quality {
                RenderQuality::Fuzzy => "fuzzy",
                RenderQuality::Draft => "draft",
                RenderQuality::High => "high",
            });

            if let Some((handle, w, h)) = GLOBAL_DISK_CACHE.get_page(&file_path, page_index, zoom, &quality_tag) {
                return Ok((handle, w, h));
            }

            let req = PageRenderRequest {
                page_index,
                zoom,
                rotation: 0,
                quality,
                max_dimensions: Some((3840, 3840)),
                is_night_mode,
                is_image_based: is_image_doc,
            };
            tokio::task::spawn_blocking(move || {
                let rgba = backend.render_page(&req)?;
                let w = rgba.width();
                let h = rgba.height();
                let raw_bytes = rgba.as_raw();

                GLOBAL_DISK_CACHE.save_page(&file_path, page_index, zoom, &quality_tag, raw_bytes, w, h);

                let handle = Handle::from_rgba(w, h, rgba.into_raw());
                Ok((handle, w, h))
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
        },
        move |result| Message::PageRenderFinished { tab_id, page_index, quality, result },
    )
}

pub fn spawn_thumbnail_render_task(
    tab_id: usize,
    file_path: PathBuf,
    page_index: usize,
    is_night_mode: bool,
    backend: Arc<dyn DocumentBackend>,
) -> Task<Message> {
    Task::perform(
        async move {
            let is_image_doc = backend.is_image_based();
            let thumb_tag = if is_night_mode && !is_image_doc { "night_thumb" } else { "thumb" };
            if let Some((handle, _, _)) = GLOBAL_DISK_CACHE.get_page(&file_path, page_index, 0.5, thumb_tag) {
                return Ok(handle);
            }

            let req = PageRenderRequest {
                page_index,
                zoom: 0.5,
                rotation: 0,
                quality: RenderQuality::Draft,
                max_dimensions: Some((300, 400)),
                is_night_mode,
                is_image_based: is_image_doc,
            };
            tokio::task::spawn_blocking(move || {
                let rgba = backend.render_page(&req)?;
                let w = rgba.width();
                let h = rgba.height();
                GLOBAL_DISK_CACHE.save_page(&file_path, page_index, 0.5, thumb_tag, rgba.as_raw(), w, h);

                let handle = Handle::from_rgba(w, h, rgba.into_raw());
                Ok(handle)
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
        },
        move |result| Message::ThumbnailRenderFinished { tab_id, page_index, result },
    )
}

impl ReaderApp {
    pub fn request_missing_page_renders(&mut self, tab_id: usize) -> Task<Message> {
        self.request_page_renders_with_quality(tab_id, RenderQuality::High)
    }

    pub fn request_page_renders_with_quality(&mut self, tab_id: usize, _quality: RenderQuality) -> Task<Message> {
        let is_night = self.settings.is_night_mode;

        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let requests = tab.required_pages_with_quality();
            let mut tasks = Vec::new();
            let current_zoom = tab.zoom;
            let file_path = tab.file_path.clone();

            for (page_idx, target_quality) in requests {
                if !tab.loading_pages.contains(&page_idx) {
                    let needs_render = match tab.texture_cache.get(&page_idx) {
                        Some(cached) => (cached.zoom - current_zoom).abs() > 0.01 || cached.quality < target_quality,
                        None => true,
                    };

                    if needs_render {
                        tab.loading_pages.insert(page_idx);
                        tasks.push(spawn_render_task(
                            tab_id,
                            file_path.clone(),
                            page_idx,
                            tab.zoom,
                            target_quality,
                            is_night,
                            tab.backend.clone(),
                        ));
                    }
                }
            }
            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn request_missing_thumbnail_renders(&mut self, tab_id: usize) -> Task<Message> {
        let is_night = self.settings.is_night_mode;

        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if tab.loading_thumbnails.len() >= MAX_CONCURRENT_THUMBNAILS {
                return Task::none();
            }

            let page_count = tab.page_count;
            if page_count == 0 {
                return Task::none();
            }

            let center_page = if tab.side_panel_scroll_offset > 0.0 {
                (tab.side_panel_scroll_offset / 210.0).floor() as usize
            } else {
                tab.current_page
            };

            let start = center_page.saturating_sub(6);
            let end = (center_page + 8).min(page_count);

            let mut candidates: Vec<usize> = (start..end)
                .filter(|&idx| {
                    !tab.thumbnail_cache.contains_key(&idx) && !tab.loading_thumbnails.contains(&idx)
                })
                .collect();

            candidates.sort_by_key(|&idx| (idx as isize - center_page as isize).abs());

            let available_slots = MAX_CONCURRENT_THUMBNAILS.saturating_sub(tab.loading_thumbnails.len());
            let to_spawn: Vec<usize> = candidates.into_iter().take(available_slots).collect();

            let file_path = tab.file_path.clone();
            let mut tasks = Vec::new();
            for page_idx in to_spawn {
                tab.loading_thumbnails.insert(page_idx);
                tasks.push(spawn_thumbnail_render_task(
                    tab_id,
                    file_path.clone(),
                    page_idx,
                    is_night,
                    tab.backend.clone(),
                ));
            }

            return Task::batch(tasks);
        }
        Task::none()
    }
}
