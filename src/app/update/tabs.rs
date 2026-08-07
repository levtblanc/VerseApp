use iced::widget::scrollable;
use iced::Task;
use rfd::AsyncFileDialog;
use std::path::PathBuf;
use std::sync::Arc;

use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::app::tasks::get_disk_cache;
use crate::engine::load_document;
use crate::engine::traits::DocumentBackend;
use crate::models::session::{PageLayout, SessionData, SidePanelTab};
use crate::models::workspace::RuntimeTab;

impl ReaderApp {
    pub fn handle_open_file_requested(&mut self) -> Task<Message> {
        Task::perform(
            async move {
                let selected_file = AsyncFileDialog::new()
                    .add_filter(
                        "Supported Documents (*.pdf, *.epub, *.docx, *.djvu, *.xps, *.cbz, *.mobi, *.fb2)",
                        &["pdf", "epub", "docx", "djvu", "xps", "mobi", "cbz", "fb2"],
                    )
                    .add_filter("All Files (*.*)", &["*"])
                    .pick_file()
                    .await;

                if let Some(file) = selected_file {
                    let path = file.path().to_path_buf();
                    tokio::task::spawn_blocking(move || {
                        load_document(&path).map(|backend| (path, backend))
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                } else {
                    Err("File selection canceled".to_string())
                }
            },
            |res| match res {
                Ok((path, backend)) => Message::FileOpened(Ok((path, backend))),
                Err(err) => Message::FileOpened(Err(err)),
            },
        )
    }

    pub fn handle_file_opened(&mut self, result: Result<(PathBuf, Arc<dyn DocumentBackend>), String>) -> Task<Message> {
        match result {
            Ok((path, backend)) => {
                let id = self.next_tab_id;
                self.next_tab_id += 1;

                let session = SessionData::load();
                let history = session.file_history.get(&path);

                let (current_page, zoom, layout, is_continuous) = if let Some(h) = history {
                    (h.current_page, h.zoom, h.layout, h.is_continuous)
                } else {
                    (0, self.settings.default_zoom, PageLayout::Single, false)
                };

                let tab = RuntimeTab::new(
                    id,
                    path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    path,
                    backend,
                    current_page,
                    zoom,
                    layout,
                    is_continuous,
                    false,
                    true,
                    SidePanelTab::TableOfContents,
                );

                let target_y = if is_continuous && current_page > 0 {
                    tab.y_offset_for_page(current_page)
                } else {
                    0.0
                };

                self.tabs.push(tab);
                self.active_tab_id = Some(id);
                self.purge_all_inactive_tabs();
                self.save_session();

                let render_task = self.request_missing_page_renders(id);

                if is_continuous && current_page > 0 {
                    let scroll_task = scrollable::scroll_to(
                        scrollable::Id::new(format!("viewer_scroll_{}", id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                    );
                    return Task::batch(vec![render_task, scroll_task]);
                }

                render_task
            }
            Err(err) => {
                if err != "File selection canceled" {
                    self.error_message = Some(format!("Failed to open document: {}", err));
                }
                Task::none()
            }
        }
    }

    pub fn handle_select_tab(&mut self, id: usize) -> Task<Message> {
        self.active_tab_id = Some(id);
        self.purge_all_inactive_tabs();
        self.save_session();

        let mut tasks = vec![self.request_missing_page_renders(id)];

        let active_info = self.tabs.iter().find(|t| t.id == id).map(|t| (
            t.is_side_panel_open,
            t.side_panel_tab,
            t.is_continuous,
            t.current_page,
            t.y_offset_for_page(t.current_page),
            t.side_panel_thumb_y(t.current_page),
        ));

        if let Some((side_open, side_tab, continuous, _current_page, target_y, side_thumb_y)) = active_info {
            if side_open {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("side_panel_scroll_{}", id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                ));

                if side_tab == SidePanelTab::Thumbnails {
                    tasks.push(self.request_missing_thumbnail_renders(id));
                }
            }

            if continuous {
                tasks.push(scrollable::scroll_to(
                    scrollable::Id::new(format!("viewer_scroll_{}", id)),
                    scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                ));
            }
        }

        Task::batch(tasks)
    }

    pub fn handle_close_tab(&mut self, id: usize) -> Task<Message> {
        let closing_pos = self.tabs.iter().position(|t| t.id == id);

        if let Some(pos) = closing_pos {
            let was_active = self.active_tab_id == Some(id);
            let closed_path = self.tabs[pos].file_path.clone();
            self.tabs.remove(pos);

            // Immediately purge disk cache for closed file if not open in another tab
            let is_shared = self.tabs.iter().any(|t| t.file_path == closed_path);
            if !is_shared {
                get_disk_cache().remove_for_file(&closed_path);
            }

            if was_active {
                if !self.tabs.is_empty() {
                    let new_idx = pos.min(self.tabs.len().saturating_sub(1));
                    let next_active_id = self.tabs[new_idx].id;
                    return self.update(Message::SelectTab(next_active_id));
                } else {
                    self.active_tab_id = None;
                }
            }
        }

        if self.split_secondary_tab_id == Some(id) {
            self.split_secondary_tab_id = None;
        }
        self.purge_all_inactive_tabs();
        self.save_session();
        Task::none()
    }

    pub fn handle_split_view_requested(&mut self, id: usize, _mode: bool) -> Task<Message> {
        self.split_secondary_tab_id = Some(id);
        Task::none()
    }

    pub fn handle_start_tab_drag(&mut self, tab_id: usize) -> Task<Message> {
        self.dragged_tab_id = Some(tab_id);
        if self.active_tab_id != Some(tab_id) {
            return self.update(Message::SelectTab(tab_id));
        }
        Task::none()
    }

    pub fn handle_tab_dragged_over(&mut self, target_id: usize) -> Task<Message> {
        if let Some(dragged_id) = self.dragged_tab_id {
            if dragged_id != target_id {
                let from_pos = self.tabs.iter().position(|t| t.id == dragged_id);
                let to_pos = self.tabs.iter().position(|t| t.id == target_id);

                if let (Some(from), Some(to)) = (from_pos, to_pos) {
                    self.tabs.swap(from, to);
                    self.save_session();
                }
            }
        }
        Task::none()
    }

    pub fn handle_end_tab_drag(&mut self) -> Task<Message> {
        self.dragged_tab_id = None;
        Task::none()
    }
}