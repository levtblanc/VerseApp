use iced::widget::scrollable;
use iced::window;
use iced::Task;

use crate::app::actions::{is_modifier_key, key_to_string};
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::models::session::{Action, KeyBinding, ThemeMode};

impl ReaderApp {
    pub fn handle_toggle_fullscreen(&mut self) -> Task<Message> {
        self.is_fullscreen = !self.is_fullscreen;
        let target_mode = if self.is_fullscreen {
            window::Mode::Fullscreen
        } else {
            window::Mode::Windowed
        };
        window::get_latest().map(move |id_opt| Message::ApplyWindowMode(id_opt, target_mode))
    }

    pub fn handle_apply_window_mode(&mut self, id_opt: Option<window::Id>, mode: window::Mode) -> Task<Message> {
        if let Some(id) = id_opt {
            window::change_mode(id, mode)
        } else {
            Task::none()
        }
    }

    pub fn handle_toggle_tab_bar(&mut self) -> Task<Message> {
        self.is_tab_bar_visible = !self.is_tab_bar_visible;
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                if tab.is_side_panel_open {
                    let side_thumb_y = tab.side_panel_thumb_y(tab.current_page);
                    return scrollable::scroll_to(
                        scrollable::Id::new(format!("side_panel_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                    );
                }
            }
        }
        Task::none()
    }

    pub fn handle_open_settings(&mut self) -> Task<Message> {
        self.is_settings_open = true;
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                let mut tasks = Vec::new();

                if tab.is_continuous {
                    let target_y = tab.y_offset_for_page(tab.current_page);
                    tasks.push(scrollable::scroll_to(
                        scrollable::Id::new(format!("viewer_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                    ));
                }

                if tab.is_side_panel_open {
                    let side_thumb_y = tab.side_panel_thumb_y(tab.current_page);
                    tasks.push(scrollable::scroll_to(
                        scrollable::Id::new(format!("side_panel_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                    ));
                }

                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    pub fn handle_close_settings(&mut self) -> Task<Message> {
        self.is_settings_open = false;
        self.remapping_action = None;
        if let Some(active_id) = self.active_tab_id {
            if let Some(tab) = self.tabs.iter().find(|t| t.id == active_id) {
                let mut tasks = Vec::new();

                if tab.is_continuous {
                    let target_y = tab.y_offset_for_page(tab.current_page);
                    tasks.push(scrollable::scroll_to(
                        scrollable::Id::new(format!("viewer_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                    ));
                }

                if tab.is_side_panel_open {
                    let side_thumb_y = tab.side_panel_thumb_y(tab.current_page);
                    tasks.push(scrollable::scroll_to(
                        scrollable::Id::new(format!("side_panel_scroll_{}", active_id)),
                        scrollable::AbsoluteOffset { x: 0.0, y: side_thumb_y },
                    ));
                }

                return Task::batch(tasks);
            }
        }
        Task::none()
    }

    pub fn handle_start_remapping(&mut self, action: Action) -> Task<Message> {
        self.remapping_action = Some(action);
        Task::none()
    }

    pub fn handle_toggle_theme(&mut self) -> Task<Message> {
        self.settings.theme = match self.settings.theme {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.save_session();
        Task::none()
    }

    pub fn handle_clear_error(&mut self) -> Task<Message> {
        self.error_message = None;
        Task::none()
    }

    pub fn handle_event_occurred(&mut self, event: iced::Event) -> Task<Message> {
        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => {
                if self.dragged_tab_id.is_some() {
                    self.dragged_tab_id = None;
                }
            }
            iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                self.active_modifiers = modifiers;
            }
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let ctrl = modifiers.control() || self.active_modifiers.control();
                let shift = modifiers.shift() || self.active_modifiers.shift();
                let alt = modifiers.alt() || self.active_modifiers.alt();

                if let Some(action_to_remap) = self.remapping_action {
                    if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                        self.remapping_action = None;
                        return Task::none();
                    }

                    if is_modifier_key(&key) {
                        return Task::none();
                    }

                    let key_str = key_to_string(&key);
                    let new_binding = KeyBinding {
                        key: key_str,
                        ctrl,
                        shift,
                        alt,
                    };
                    self.settings.keybindings.insert(action_to_remap, new_binding);
                    self.remapping_action = None;
                    self.save_session();
                    return Task::none();
                }

                let key_str = key_to_string(&key);
                let matched_action = self.settings.keybindings.iter().find_map(|(action, binding)| {
                    if binding.key.eq_ignore_ascii_case(&key_str)
                        && binding.ctrl == ctrl
                        && binding.shift == shift
                        && binding.alt == alt
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
}