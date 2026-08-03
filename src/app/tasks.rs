use iced::Task;
use iced::widget::image::Handle;
use std::sync::Arc;
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality};

const MAX_CONCURRENT_THUMBNAILS: usize = 5;

pub fn spawn_render_task(
    tab_id: usize,
    page_index: usize,
    zoom: f32,
    quality: RenderQuality,
    backend: Arc<dyn DocumentBackend>,
) -> Task<Message> {
    Task::perform(
        async move {
            let req = PageRenderRequest {
                page_index,
                zoom,
                rotation: 0,
                quality,
                max_dimensions: Some((3840, 3840)),
            };
            tokio::task::spawn_blocking(move || {
                let rgba = backend.render_page(&req)?;
                let w = rgba.width();
                let h = rgba.height();
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
    page_index: usize,
    backend: Arc<dyn DocumentBackend>,
) -> Task<Message> {
    Task::perform(
        async move {
            // Crisp Middle-Ground Thumbnail Request:
            // RenderQuality::Draft with 300x400 max bounds for High-DPI sharpness
            let req = PageRenderRequest {
                page_index,
                zoom: 0.5,
                rotation: 0,
                quality: RenderQuality::Draft,
                max_dimensions: Some((300, 400)),
            };
            tokio::task::spawn_blocking(move || {
                let rgba = backend.render_page(&req)?;
                let handle = Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw());
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

    pub fn request_page_renders_with_quality(&mut self, tab_id: usize, quality: RenderQuality) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let requests = tab.required_pages_with_quality();
            let mut tasks = Vec::new();
            let current_zoom = tab.zoom;

            for (page_idx, target_quality) in requests {
                if !tab.loading_pages.contains(&page_idx) {
                    let needs_render = match tab.texture_cache.get(&page_idx) {
                        Some(cached) => (cached.zoom - current_zoom).abs() > 0.01 || cached.quality < target_quality,
                        None => true,
                    };

                    if needs_render {
                        tab.loading_pages.insert(page_idx);
                        tasks.push(spawn_render_task(tab_id, page_idx, tab.zoom, target_quality, tab.backend.clone()));
                    }
                }
            }
            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn request_missing_thumbnail_renders(&mut self, tab_id: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            // Yield if main page is still rasterizing
            if !tab.loading_pages.is_empty() || tab.loading_thumbnails.len() >= MAX_CONCURRENT_THUMBNAILS {
                return Task::none();
            }

            let page_count = tab.page_count;
            if page_count == 0 {
                return Task::none();
            }

            let start = tab.current_page.saturating_sub(15);
            let end = (tab.current_page + 16).min(page_count);

            let mut candidates: Vec<usize> = (start..end)
                .filter(|&idx| {
                    !tab.thumbnail_cache.contains_key(&idx) && !tab.loading_thumbnails.contains(&idx)
                })
                .collect();

            // Prioritize thumbnails closest to current reading page
            candidates.sort_by_key(|&idx| (idx as isize - tab.current_page as isize).abs());

            let available_slots = MAX_CONCURRENT_THUMBNAILS.saturating_sub(tab.loading_thumbnails.len());
            let to_spawn: Vec<usize> = candidates.into_iter().take(available_slots).collect();

            let mut tasks = Vec::new();
            for page_idx in to_spawn {
                tab.loading_thumbnails.insert(page_idx);
                tasks.push(spawn_thumbnail_render_task(
                    tab_id,
                    page_idx,
                    tab.backend.clone(),
                ));
            }

            return Task::batch(tasks);
        }
        Task::none()
    }
}