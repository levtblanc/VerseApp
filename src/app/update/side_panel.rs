use iced::widget::scrollable;
use iced::Task;

use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::models::session::SidePanelTab;

impl ReaderApp {
    pub fn handle_toggle_side_panel(&mut self, tab_id: usize) -> Task<Message> {
        let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
            tab.is_side_panel_open = !tab.is_side_panel_open;
            let target_y = tab.y_offset_for_page(tab.current_page);
            let side_thumb_y = tab.side_panel_thumb_y(tab.current_page);
            tab.side_panel_scroll_offset = side_thumb_y;
            (tab.is_side_panel_open, tab.side_panel_tab, tab.is_continuous, target_y, side_thumb_y)
        });

        if let Some((is_open, side_tab, is_continuous, target_y, side_thumb_y)) = tab_info {
            self.save_session();
            let mut tasks = Vec::new();

            if is_continuous {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                ));
            }

            if is_open {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                ));

                if side_tab == SidePanelTab::Thumbnails {
                    tasks.push(self.request_missing_thumbnail_renders(tab_id));
                }
            }

            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn handle_toggle_side_panel_pin(&mut self, tab_id: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_side_panel_pinned = !tab.is_side_panel_pinned;
            self.save_session();
        }
        Task::none()
    }

    pub fn handle_set_side_panel_tab(&mut self, tab_id: usize, side_tab: SidePanelTab) -> Task<Message> {
        let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
            tab.side_panel_tab = side_tab;
            let side_thumb_y = tab.side_panel_thumb_y(tab.current_page);
            tab.side_panel_scroll_offset = side_thumb_y;
            side_thumb_y
        });

        if let Some(side_thumb_y) = tab_info {
            self.save_session();
            let mut tasks = Vec::new();

            if side_tab == SidePanelTab::Thumbnails {
                tasks.push(self.request_missing_thumbnail_renders(tab_id));

                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                ));
            }
            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn handle_side_panel_scrolled(&mut self, tab_id: usize, offset_y: f32) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if offset_y == 0.0 && tab.current_page > 1 {
                return Task::none();
            }
            tab.side_panel_scroll_offset = offset_y;
            return self.request_missing_thumbnail_renders(tab_id);
        }
        Task::none()
    }
}