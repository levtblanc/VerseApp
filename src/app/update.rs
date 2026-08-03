use iced::widget::scrollable;
use iced::window;
use iced::Task;
use rfd::AsyncFileDialog;

use crate::app::actions::key_to_string;
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::engine::load_document;
use crate::engine::traits::RenderQuality;
use crate::models::session::{KeyBinding, PageLayout, SessionData, SidePanelTab, ThemeMode};
use crate::models::workspace::RuntimeTab;

impl ReaderApp {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ClearError => {
                self.error_message = None;
                Task::none()
            }

            Message::ToggleSidePanel(tab_id) => {
                let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
                    tab.is_side_panel_open = !tab.is_side_panel_open;
                    let target_y = tab.y_offset_for_page(tab.current_page);
                    (tab.is_side_panel_open, tab.side_panel_tab, tab.is_continuous, target_y, tab.current_page)
                });

                if let Some((is_open, side_tab, is_continuous, target_y, current_page)) = tab_info {
                    self.save_session();
                    let mut tasks = Vec::new();

                    if is_continuous {
                        tasks.push(scrollable::scroll_to(
                            scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                            scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                        ));
                    }

                    if is_open {
                        // Auto-scroll side panel to the current page's thumbnail
                        let side_thumb_y = current_page as f32 * 208.0;
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

            Message::ToggleSidePanelPin(tab_id) => {
                let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
                    tab.is_side_panel_pinned = !tab.is_side_panel_pinned;
                    let target_y = tab.y_offset_for_page(tab.current_page);
                    (tab.is_continuous, target_y)
                });

                if let Some((is_continuous, target_y)) = tab_info {
                    self.save_session();
                    if is_continuous {
                        return scrollable::scroll_to(
                            scrollable::Id::new(format!("viewer_scroll_{}", tab_id)),
                            scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                        );
                    }
                }
                Task::none()
            }

            Message::SetSidePanelTab(tab_id, side_tab) => {
                let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
                    tab.side_panel_tab = side_tab;
                    tab.current_page
                });

                if let Some(current_page) = tab_info {
                    self.save_session();
                    let mut tasks = Vec::new();

                    if side_tab == SidePanelTab::Thumbnails {
                        tasks.push(self.request_missing_thumbnail_renders(tab_id));

                        // Auto-scroll side panel to current page thumbnail on sub-tab switch
                        let side_thumb_y = current_page as f32 * 208.0;
                        tasks.push(scrollable::scroll_to(
                            scrollable::Id::new(format!("side_panel_scroll_{}", tab_id)),
                            scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                        ));
                    }
                    return Task::batch(tasks);
                }
                Task::none()
            }

            Message::ToggleFullscreen => {
                self.is_fullscreen = !self.is_fullscreen;
                let target_mode = if self.is_fullscreen {
                    window::Mode::Fullscreen
                } else {
                    window::Mode::Windowed
                };
                window::get_latest().map(move |id_opt| Message::ApplyWindowMode(id_opt, target_mode))
            }

            Message::ApplyWindowMode(id_opt, mode) => {
                if let Some(id) = id_opt {
                    window::change_mode(id, mode)
                } else {
                    Task::none()
                }
            }

            Message::ToggleTabBar => {
                self.is_tab_bar_visible = !self.is_tab_bar_visible;
                Task::none()
            }

            Message::OpenFileRequested => Task::perform(
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
            ),

            Message::FileOpened(Ok((path, backend))) => {
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

                return render_task;
            }

            Message::FileOpened(Err(err)) => {
                if err != "File selection canceled" {
                    self.error_message = Some(format!("Failed to open document: {}", err));
                }
                Task::none()
            }

            Message::EventOccurred(event) => {
                match event {
                    iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                        self.active_modifiers = modifiers;
                    }
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
                        let key_str = key_to_string(&key);
                        if let Some(action_to_remap) = self.remapping_action {
                            let new_binding = KeyBinding {
                                key: key_str,
                                ctrl: self.active_modifiers.control(),
                                shift: self.active_modifiers.shift(),
                                alt: self.active_modifiers.alt(),
                            };
                            self.settings.keybindings.insert(action_to_remap, new_binding);
                            self.remapping_action = None;
                            self.save_session();
                            return Task::none();
                        }

                        let matched_action = self.settings.keybindings.iter().find_map(|(action, binding)| {
                            if binding.key.eq_ignore_ascii_case(&key_str)
                                && binding.ctrl == self.active_modifiers.control()
                                && binding.shift == self.active_modifiers.shift()
                                && binding.alt == self.active_modifiers.alt()
                            {
                                Some(*action)
                            } else {
                                None
                            }
                        });

                        if let Some(action) = matched_action {
                            return self.handle_action(action);
                        }
                    }
                    iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                        if self.active_modifiers.control() {
                            if let Some(tab_id) = self.active_tab_id {
                                if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                                    let scroll_y = match delta {
                                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                                        iced::mouse::ScrollDelta::Pixels { y, .. } => y / 35.0,
                                    };
                                    let delta_zoom = if scroll_y > 0.0 { 0.15 } else { -0.15 };
                                    let new_zoom = (tab.zoom + delta_zoom).clamp(0.2, 5.0);
                                    return self.update(Message::ChangeZoom(tab_id, new_zoom));
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::ViewportScrolled { tab_id, offset_y } => {
                let mut tasks = Vec::new();

                let should_render = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    if tab.is_continuous {
                        let scrolled_page = tab.page_at_y_offset(offset_y);

                        if tab.current_page != scrolled_page {
                            tab.current_page = scrolled_page;
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
                        .map(|t| (t.is_side_panel_open, t.side_panel_tab, t.current_page));

                    if let Some((side_open, side_tab, current_page)) = side_info {
                        if side_open && side_tab == SidePanelTab::Thumbnails {
                            tasks.push(self.request_missing_thumbnail_renders(tab_id));

                            let side_thumb_y = current_page as f32 * 208.0;
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

            Message::ScrollSettled { tab_id, sequence } => {
                if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                    if tab.scroll_sequence == sequence {
                        return self.request_page_renders_with_quality(tab_id, RenderQuality::High);
                    }
                }
                Task::none()
            }

            Message::ChangeZoom(tab_id, new_zoom) => {
                let seq = if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
                    tab.zoom = new_zoom;
                    tab.zoom_sequence += 1;
                    Some((tab.zoom_sequence, self.request_page_renders_with_quality(tab_id, RenderQuality::Draft)))
                } else {
                    None
                };

                if let Some((sequence, draft_task)) = seq {
                    self.save_session();

                    let debounce_task = Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            sequence
                        },
                        move |seq| Message::ZoomSettled { tab_id, sequence: seq },
                    );

                    return Task::batch(vec![draft_task, debounce_task]);
                }
                Task::none()
            }

            Message::ZoomSettled { tab_id, sequence } => {
                if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                    if tab.zoom_sequence == sequence {
                        return self.request_page_renders_with_quality(tab_id, RenderQuality::High);
                    }
                }
                Task::none()
            }

            Message::ChangePage(tab_id, new_page) => {
                let tab_info = self.tabs.iter_mut().find(|t| t.id == tab_id).map(|tab| {
                    tab.current_page = new_page;
                    let target_y = tab.y_offset_for_page(new_page);
                    (tab.is_continuous, tab.layout, tab.zoom, tab.is_side_panel_open, tab.side_panel_tab, target_y)
                });

                if let Some((is_continuous, _layout, _zoom, side_open, side_tab, target_y)) = tab_info {
                    self.save_session();
                    let mut tasks = vec![self.request_missing_page_renders(tab_id)];

                    if side_open && side_tab == SidePanelTab::Thumbnails {
                        tasks.push(self.request_missing_thumbnail_renders(tab_id));

                        let side_thumb_y = new_page as f32 * 208.0;
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

            Message::TogglePageLayout(tab_id) => {
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

            Message::ToggleContinuous(tab_id) => {
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

            Message::SelectTab(id) => {
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
                ));

                if let Some((side_open, side_tab, continuous, _current_page, target_y)) = active_info {
                    if side_open && side_tab == SidePanelTab::Thumbnails {
                        tasks.push(self.request_missing_thumbnail_renders(id));
                    }

                    if continuous {
                        tasks.push(scrollable::scroll_to(
                            scrollable::Id::new(format!("viewer_scroll_{}", id)),
                            scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                        ));
                    }
                }

                return Task::batch(tasks);
            }

            Message::CloseTab(id) => {
                self.tabs.retain(|t| t.id != id);
                if self.active_tab_id == Some(id) {
                    self.active_tab_id = self.tabs.first().map(|t| t.id);
                }
                if self.split_secondary_tab_id == Some(id) {
                    self.split_secondary_tab_id = None;
                }
                self.purge_all_inactive_tabs();
                self.save_session();
                Task::none()
            }

            Message::SplitViewRequested(id, _) => {
                self.split_secondary_tab_id = Some(id);
                Task::none()
            }

            Message::OpenSettings => {
                self.is_settings_open = true;
                Task::none()
            }
            Message::CloseSettings => {
                self.is_settings_open = false;
                self.remapping_action = None;
                Task::none()
            }
            Message::StartRemapping(action) => {
                self.remapping_action = Some(action);
                Task::none()
            }

            Message::ToggleTheme => {
                self.settings.theme = match self.settings.theme {
                    ThemeMode::Light => ThemeMode::Dark,
                    ThemeMode::Dark => ThemeMode::Light,
                };
                self.save_session();
                Task::none()
            }

            Message::PageRenderFinished { tab_id, page_index, quality, result } => {
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

            Message::ThumbnailRenderFinished { tab_id, page_index, result } => {
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
    }
}