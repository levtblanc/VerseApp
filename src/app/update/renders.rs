use iced::widget::image::Handle;
use iced::Task;

use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::engine::traits::RenderQuality;
use crate::models::session::SidePanelTab;

impl ReaderApp {
    pub fn handle_page_render_finished(
        &mut self,
        tab_id: usize,
        page_index: usize,
        quality: RenderQuality,
        result: Result<(Handle, u32, u32), String>,
    ) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.loading_pages.remove(&page_index);
            match result {
                Ok((handle, width, height)) => {
                    let current_zoom = tab.zoom;
                    tab.insert_texture_with_size(page_index, handle, quality, current_zoom, width, height);
                }
                Err(err) => {
                    self.error_message = Some(format!("Page {} rendering error: {}", page_index + 1, err));
                }
            }
        }
        Task::none()
    }

    pub fn handle_thumbnail_render_finished(
        &mut self,
        tab_id: usize,
        page_index: usize,
        result: Result<Handle, String>,
    ) -> Task<Message> {
        let mut follow_up_task = Task::none();

        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.loading_thumbnails.remove(&page_index);
            if let Ok(handle) = result {
                tab.insert_thumbnail(page_index, handle);
            }
        }

        if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
            if tab.is_side_panel_open && tab.side_panel_tab == SidePanelTab::Thumbnails {
                follow_up_task = self.request_missing_thumbnail_renders(tab_id);
            }
        }

        follow_up_task
    }
}