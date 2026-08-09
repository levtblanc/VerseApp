use iced::widget::scrollable;
use iced::Task;

use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::app::tasks::spawn_search_task;
use crate::engine::traits::RenderQuality;
use crate::models::session::{PageLayout, SidePanelTab};
use crate::models::workspace::SearchMatch;

impl ReaderApp {
    pub fn handle_change_page(&mut self, tab_id: usize, new_page: usize) -> Task<Message> {
        let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
            tab.current_page = new_page;
            tab.page_input_text = (new_page + 1).to_string();
            tab.clear_selection();
            let target_y = tab.y_offset_for_page(new_page);
            tab.viewport_y = target_y;
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
            tab.clear_selection();
            let y_off = tab.y_offset_for_page(target_page);
            tab.viewport_y = y_off;
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
        let clamped_zoom = new_zoom.clamp(0.2, 5.0);

        let (seq, new_scroll_y, is_continuous) = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let old_zoom = tab.zoom;
            if (old_zoom - clamped_zoom).abs() < 0.001 {
                return Task::none();
            }

            let ratio = clamped_zoom / old_zoom.max(0.01);
            let new_scroll_y = tab.viewport_y * ratio;

            tab.zoom = clamped_zoom;
            tab.viewport_y = new_scroll_y;
            tab.is_zooming = true;
            tab.zoom_sequence += 1;

            (tab.zoom_sequence, new_scroll_y, tab.is_continuous)
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
                scrollable::AbsoluteOffset { x: 0.0, y: new_scroll_y },
            ));
        }

        Task::batch(tasks)
    }

    pub fn handle_zoom_settled(&mut self, tab_id: usize, sequence: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if tab.zoom_sequence == sequence {
                tab.is_zooming = false;
                return self.request_page_renders_with_quality(tab_id, RenderQuality::High);
            }
        }
        Task::none()
    }

    pub fn handle_viewport_scrolled(&mut self, tab_id: usize, offset_y: f32) -> Task<Message> {
        let mut tasks = Vec::new();

        let should_render = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            let safe_y = if offset_y.is_finite() { offset_y.max(0.0) } else { 0.0 };
            tab.viewport_y = safe_y;

            if safe_y == 0.0 && tab.current_page > 0 {
                false
            } else if tab.is_continuous {
                let scrolled_page = tab.page_at_y_offset(safe_y);

                // Suppress intermediate page index flips during zoom layout transitions
                let page_diff = (scrolled_page as isize - tab.current_page as isize).abs();
                if tab.is_zooming && page_diff > 1 {
                    false
                } else if tab.current_page != scrolled_page {
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
        let (is_continuous, target_y) = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.layout = match tab.layout {
                PageLayout::Single => PageLayout::Double,
                PageLayout::Double => PageLayout::Single,
            };
            let target_y = tab.y_offset_for_page(tab.current_page);
            tab.viewport_y = target_y;
            (tab.is_continuous, target_y)
        } else {
            return Task::none();
        };

        self.save_session();
        let render_task = self.request_missing_page_renders(tab_id);

        if is_continuous {
            let scroll_task = scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: target_y },
            );
            return Task::batch(vec![render_task, scroll_task]);
        } else {
            let scroll_task = scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
            );
            return Task::batch(vec![render_task, scroll_task]);
        }
    }

    pub fn handle_toggle_continuous(&mut self, tab_id: usize) -> Task<Message> {
        let (is_continuous, target_y) = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_continuous = !tab.is_continuous;
            let target_y = tab.y_offset_for_page(tab.current_page);
            tab.viewport_y = target_y;
            (tab.is_continuous, target_y)
        } else {
            return Task::none();
        };

        self.save_session();
        let render_task = self.request_missing_page_renders(tab_id);

        if is_continuous {
            let scroll_task = scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: target_y },
            );
            return Task::batch(vec![render_task, scroll_task]);
        } else {
            let scroll_task = scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
            );
            return Task::batch(vec![render_task, scroll_task]);
        }
    }

    pub fn handle_start_text_selection(&mut self, page_index: usize, x: f32, y: f32) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            let is_shift = self.active_modifiers.shift();
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                let unscaled_x = x / tab.zoom;
                let unscaled_y = y / tab.zoom;

                if is_shift && tab.selection_start.is_some() {
                    tab.update_selection(page_index, unscaled_x, unscaled_y);
                } else {
                    tab.start_selection(page_index, unscaled_x, unscaled_y);
                }
            }
        }
        Task::none()
    }

    pub fn handle_update_text_selection(&mut self, page_index: usize, x: f32, y: f32) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                let unscaled_x = x / tab.zoom;
                let unscaled_y = y / tab.zoom;
                tab.update_selection(page_index, unscaled_x, unscaled_y);
            }
        }
        Task::none()
    }

    pub fn handle_end_text_selection(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.end_selection();
            }
        }
        Task::none()
    }

    pub fn handle_copy_selected_text(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                if !tab.selected_text.is_empty() {
                    return iced::clipboard::write(tab.selected_text.clone());
                }
            }
        }
        Task::none()
    }

    pub fn handle_toggle_search(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.is_search_open = !tab.is_search_open;
                if tab.is_continuous {
                    let target_y = tab.viewport_y;
                    return scrollable::scroll_to(
                        scrollable::Id::new(format!("viewer_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                    );
                }
            }
        }
        Task::none()
    }

    pub fn handle_close_search(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.is_search_open = false;
            }
        }
        Task::none()
    }

    pub fn handle_search_query_changed(&mut self, query: String) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.search_query = query.clone();
                return spawn_search_task(
                    active_id,
                    tab.backend.clone(),
                    tab.page_count,
                    query,
                    tab.search_match_case,
                );
            }
        }
        Task::none()
    }

    pub fn handle_search_completed(&mut self, tab_id: usize, query: String, matches: Vec<SearchMatch>) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if tab.search_query.trim() == query {
                tab.search_matches = matches;
                tab.current_search_idx = 0;
            }
        }
        Task::none()
    }

    pub fn handle_toggle_search_match_case(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.search_match_case = !tab.search_match_case;
                return spawn_search_task(
                    active_id,
                    tab.backend.clone(),
                    tab.page_count,
                    tab.search_query.clone(),
                    tab.search_match_case,
                );
            }
        }
        Task::none()
    }

    pub fn handle_next_search_match(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                if !tab.search_matches.is_empty() {
                    tab.current_search_idx = (tab.current_search_idx + 1) % tab.search_matches.len();
                }
            }
            return self.jump_to_current_search_match(active_id);
        }
        Task::none()
    }

    pub fn handle_prev_search_match(&mut self) -> Task<Message> {
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                if !tab.search_matches.is_empty() {
                    tab.current_search_idx = if tab.current_search_idx == 0 {
                        tab.search_matches.len() - 1
                    } else {
                        tab.current_search_idx - 1
                    };
                }
            }
            return self.jump_to_current_search_match(active_id);
        }
        Task::none()
    }

    pub fn jump_to_current_search_match(&mut self, tab_id: usize) -> Task<Message> {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            if tab.search_matches.is_empty() {
                return Task::none();
            }

            let match_idx = tab.current_search_idx.min(tab.search_matches.len().saturating_sub(1));
            let search_match = tab.search_matches[match_idx].clone();

            tab.current_page = search_match.page_index;
            tab.page_input_text = (search_match.page_index + 1).to_string();

            let target_y = if tab.is_continuous {
                let page_base_y = tab.y_offset_for_page(search_match.page_index);
                let quad_y = search_match.quad.y0 * tab.zoom;
                let match_center_y = page_base_y + quad_y;

                (match_center_y - 350.0).max(0.0)
            } else {
                0.0
            };

            tab.viewport_y = target_y;
            let render_task = self.request_missing_page_renders(tab_id);

            let scroll_task = scrollable::scroll_to(
                scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                scrollable::AbsoluteOffset { x: 0.0, y: target_y },
            );

            return Task::batch(vec![render_task, scroll_task]);
        }
        Task::none()
    }
}