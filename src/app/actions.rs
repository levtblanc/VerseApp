use iced::keyboard;
use iced::Task;
use crate::app::messages::Message;
use crate::app::state::ReaderApp;
use crate::models::session::{Action, PageLayout};

pub fn is_modifier_key(key: &keyboard::Key) -> bool {
    match key {
        keyboard::Key::Named(named) => matches!(
            named,
            keyboard::key::Named::Control
                | keyboard::key::Named::Shift
                | keyboard::key::Named::Alt
                | keyboard::key::Named::Meta
                | keyboard::key::Named::Super
                | keyboard::key::Named::Hyper
                | keyboard::key::Named::AltGraph
        ),
        keyboard::Key::Character(c) => {
            let s = c.as_str();
            s.eq_ignore_ascii_case("control")
                || s.eq_ignore_ascii_case("shift")
                || s.eq_ignore_ascii_case("alt")
                || s.eq_ignore_ascii_case("meta")
                || s.eq_ignore_ascii_case("super")
        }
        _ => false,
    }
}

pub fn key_to_string(key: &keyboard::Key) -> String {
    match key {
        keyboard::Key::Named(named) => match named {
            keyboard::key::Named::Tab => "Tab".to_string(),
            keyboard::key::Named::ArrowRight => "Right".to_string(),
            keyboard::key::Named::ArrowLeft => "Left".to_string(),
            keyboard::key::Named::ArrowDown => "Down".to_string(),
            keyboard::key::Named::ArrowUp => "Up".to_string(),
            keyboard::key::Named::PageUp => "PageUp".to_string(),
            keyboard::key::Named::PageDown => "PageDown".to_string(),
            keyboard::key::Named::Enter => "Enter".to_string(),
            keyboard::key::Named::Space => "Space".to_string(),
            keyboard::key::Named::Escape => "Escape".to_string(),
            keyboard::key::Named::Backspace => "Backspace".to_string(),
            keyboard::key::Named::F11 => "F11".to_string(),
            _ => format!("{:?}", named),
        },
        keyboard::Key::Character(c) => match c.as_str() {
            "=" => "Equal".to_string(),
            "-" => "Minus".to_string(),
            "," => "Comma".to_string(),
            _ => c.to_uppercase(),
        },
        keyboard::Key::Unidentified => "Unknown".to_string(),
    }
}

impl ReaderApp {
    pub fn handle_action(&mut self, action: Action) -> Task<Message> {
        match action {
            Action::OpenFile => return self.update(Message::OpenFileRequested),
            Action::ToggleFullscreen => return self.update(Message::ToggleFullscreen),
            Action::ToggleTabBar => return self.update(Message::ToggleTabBar),
            Action::ToggleSidePanel => {
                if let Some(tab_id) = self.active_tab_id {
                    return self.update(Message::ToggleSidePanel(tab_id));
                }
            }

            Action::NextTab => {
                if !self.tabs.is_empty() {
                    if let Some(active_id) = self.active_tab_id {
                        if let Some(pos) = self.tabs.iter().position(|t| t.id == active_id) {
                            let next_id = self.tabs[(pos + 1) % self.tabs.len()].id;
                            return self.update(Message::SelectTab(next_id));
                        }
                    }
                }
            }

            Action::PrevTab => {
                if !self.tabs.is_empty() {
                    if let Some(active_id) = self.active_tab_id {
                        if let Some(pos) = self.tabs.iter().position(|t| t.id == active_id) {
                            let prev_id = self.tabs[(pos + self.tabs.len() - 1) % self.tabs.len()].id;
                            return self.update(Message::SelectTab(prev_id));
                        }
                    }
                }
            }

            Action::ZoomIn => {
                if let Some(tab_id) = self.active_tab_id {
                    if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                        return self.update(Message::ChangeZoom(tab_id, tab.zoom + 0.15));
                    }
                }
            }
            Action::ZoomOut => {
                if let Some(tab_id) = self.active_tab_id {
                    if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                        return self.update(Message::ChangeZoom(tab_id, (tab.zoom - 0.15).max(0.2)));
                    }
                }
            }
            Action::NextPage => {
                if let Some(tab_id) = self.active_tab_id {
                    if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                        let step = if tab.layout == PageLayout::Double { 2 } else { 1 };
                        let max_page = tab.page_count;
                        return self.update(Message::ChangePage(tab_id, (tab.current_page + step).min(max_page.saturating_sub(1))));
                    }
                }
            }
            Action::PrevPage => {
                if let Some(tab_id) = self.active_tab_id {
                    if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                        let step = if tab.layout == PageLayout::Double { 2 } else { 1 };
                        return self.update(Message::ChangePage(tab_id, tab.current_page.saturating_sub(step)));
                    }
                }
            }
            Action::TogglePageLayout => {
                if let Some(tab_id) = self.active_tab_id {
                    return self.update(Message::TogglePageLayout(tab_id));
                }
            }
            Action::ToggleContinuous => {
                if let Some(tab_id) = self.active_tab_id {
                    return self.update(Message::ToggleContinuous(tab_id));
                }
            }
            Action::ToggleTheme => return self.update(Message::ToggleTheme),
            Action::OpenSettings => return self.update(Message::OpenSettings),
            Action::CloseActiveTab => {
                if let Some(tab_id) = self.active_tab_id {
                    return self.update(Message::CloseTab(tab_id));
                }
            }
        }
        Task::none()
    }
}