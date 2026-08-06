use iced::widget::scrollable;
use iced::Task;

use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::engine::traits::RenderQuality;
use crate::models::session::{PageLayout, SidePanelTab};

impl ReaderApp {
    pub fn handle_change_page(&mut self, tab_id: usize, new_page: usize) -> Task<Message> {
        let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
            tab.current_page = new_page;
            tab.page_input_text = (new_page + 1).to_string();
            let target_y = tab.y_offset_for_page(new_page);
            let side_thumb_y = tab.side_panel_thumb_y(new_page);
            tab.side_panel_scroll_offset = side_thumb_y;
            (tab.is_continuous, tab.layout, tab.zoom, tab.is_side_panel_open, tab.side_panel_tab, target_y, side_thumb_y)
        });

        if let Some((is_continuous, _layout, _zoom, side_open, side_tab, target_y, side_thumb_y)) = tab_info {
            self.save_session();
            let mut tasks = vec![self.request_missing_page_renders(tab_id)];

            if side_open && side_tab == SidePanelTab::Thumbnails {
                tasks.push(self.request_missing_thumbnail_renders(tab_id));

                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                ));
            }

            if is_continuous {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                ));
            } else {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                ));
            }
            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn handle_page_input_changed(&mut self, tab_id: usize, value: String) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.page_input_text = value;
        }
        Task::none()
    }

    pub fn handle_page_input_submitted(&mut self, tab_id: usize) -> Task<Message> {
        let (target_y, side_thumb_y, is_continuous) = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let page_count = tab.page_count;
            if page_count == 0 {
                tab.page_input_text = "1".to_string();
                return Task::none();
            }

            let target_page = match tab.page_input_text.trim().parse::<usize>() {
                Ok(parsed) => parsed.saturating_sub(1).min(page_count.saturating_sub(1)),
                Err(_) => tab.current_page,
            };

            tab.current_page = target_page;
            tab.page_input_text = (target_page + 1).to_string();
            let y_off = tab.y_offset_for_page(target_page);
            let side_thumb_y = tab.side_panel_thumb_y(target_page);
            tab.side_panel_scroll_offset = side_thumb_y;
            (y_off, side_thumb_y, tab.is_continuous)
        } else {
            return Task::none();
        };

        self.save_session();
        let mut tasks = vec![self.request_missing_page_renders(tab_id)];

        let side_info = self.tabs.iter().find(|t| t.id == tab_id)
            .map(|t| (t.is_side_panel_open, t.side_panel_tab));

        if let Some((side_open, side_tab)) = side_info {
            if side_open && side_tab == SidePanelTab::Thumbnails {
                tasks.push(self.request_missing_thumbnail_renders(tab_id));

                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                ));
            }
        }

        if is_continuous {
            tasks.push(scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: target_y },
            ));
        } else {
            tasks.push(scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
            ));
        }

        Task::batch(tasks)
    }

    pub fn handle_change_zoom(&mut self, tab_id: usize, new_zoom: f32) -> Task<Message> {
        let (seq, target_y, is_continuous) = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.zoom = new_zoom;
            tab.zoom_sequence += 1;
            let target_y = tab.y_offset_for_page(tab.current_page);
            (tab.zoom_sequence, target_y, tab.is_continuous)
        } else {
            return Task::none();
        };

        let draft_task = self.request_page_renders_with_quality(tab_id, RenderQuality::Draft);
        self.save_session();

        let debounce_task = Task::perform(
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                seq
            },
            move |sequence| Message::ZoomSettled { tab_id, sequence },
        );

        let mut tasks = vec![draft_task, debounce_task];

        if is_continuous {
            tasks.push(scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: target_y },
            ));
        }

        Task::batch(tasks)
    }

    pub fn handle_zoom_settled(&mut self, tab_id: usize, sequence: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
            if tab.zoom_sequence == sequence {
                return self.request_page_renders_with_quality(tab_id, RenderQuality::High);
            }
        }
        Task::none()
    }

    pub fn handle_viewport_scrolled(&mut self, tab_id: usize, offset_y: f32) -> Task<Message> {
        let mut tasks = Vec::new();

        let should_render = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if offset_y == 0.0 && tab.current_page > 0 {
                false
            } else if tab.is_continuous {
                let scrolled_page = tab.page_at_y_offset(offset_y);

                if tab.current_page != scrolled_page {
                    tab.current_page = scrolled_page;
                    tab.page_input_text = (scrolled_page + 1).to_string();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_render {
            self.save_session();
            tasks.push(self.request_page_renders_with_quality(tab_id, RenderQuality::Draft));

            let side_info = self.tabs.iter().find(|t| t.id == tab_id)
                .map(|t| (t.is_side_panel_open, t.side_panel_tab, t.side_panel_thumb_y(t.current_page)));

            if let Some((side_open, side_tab, side_thumb_y)) = side_info {
                if side_open && side_tab == SidePanelTab::Thumbnails {
                    if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.side_panel_scroll_offset = side_thumb_y;
                    }
                    tasks.push(self.request_missing_thumbnail_renders(tab_id));

                    tasks.push(scrollable::scroll_to(
                        scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                    ));
                }
            }

            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                tab.scroll_sequence += 1;
                let seq = tab.scroll_sequence;

                tasks.push(Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
                        seq
                    },
                    move |sequence| Message::ScrollSettled { tab_id, sequence },
                ));
            }

            return Task::batch(tasks);
        }
        Task::none()
    }

    pub fn handle_scroll_settled(&mut self, tab_id: usize, sequence: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
            if tab.scroll_sequence == sequence {
                return self.request_page_renders_with_quality(tab_id, RenderQuality::High);
            }
        }
        Task::none()
    }

    pub fn handle_toggle_page_layout(&mut self, tab_id: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.layout = match tab.layout {
                PageLayout::Single => PageLayout::Double,
                PageLayout::Double => PageLayout::Single,
            };
            self.save_session();
            return self.request_missing_page_renders(tab_id);
        }
        Task::none()
    }

    pub fn handle_toggle_continuous(&mut self, tab_id: usize) -> Task<Message> {
        let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
            tab.is_continuous = !tab.is_continuous;
            let target_y = tab.y_offset_for_page(tab.current_page);
            (tab.is_continuous, tab.current_page, target_y)
        });

        if let Some((is_continuous, _current_page, target_y)) = tab_info {
            self.save_session();
            let render_task = self.request_missing_page_renders(tab_id);

            if is_continuous {
                let scroll_task = scrollable::scroll_to(
                    scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                );
                return Task::batch(vec![render_task, scroll_task]);
            }

            return render_task;
        }
        Task::none()
    }
}