use iced::keyboard::Modifiers;
use iced::{Subscription, Task, Theme};
use std::time::Instant;

use crate::app::messages::Message;
use crate::app::tasks::get_disk_cache;
use crate::engine::load_document;
use crate::models::disk_cache::UsefulCacheEntry;
use crate::models::session::{AppSettings, FileHistoryRecord, SessionData};
use crate::models::workspace::{trim_memory, RuntimeTab};
use crate::ui::theme::get_iced_theme;

const INACTIVE_COOLDOWN_SECS: u64 = 180; // 3 minutes soft retention in RAM
const TOTAL_APP_RAM_BUDGET_BYTES: usize = 60 * 1024 * 1024; // 60 MB global memory budget across all tabs

pub struct ReaderApp {
    pub settings: AppSettings,
    pub tabs: Vec<RuntimeTab>,
    pub active_tab_id: Option<usize>,
    pub next_tab_id: usize,
    pub split_secondary_tab_id: Option<usize>,
    pub dragged_tab_id: Option<usize>,

    pub is_settings_open: bool,
    pub remapping_action: Option<crate::models::session::Action>,
    pub active_modifiers: Modifiers,

    pub is_fullscreen: bool,
    pub is_tab_bar_visible: bool,
    pub error_message: Option<String>,
}

impl ReaderApp {
    pub fn new() -> (Self, Task<Message>) {
        let session = SessionData::load();
        let mut restored_tabs = Vec::new();
        let mut next_tab_id = 1;

        for tab_session in &session.open_tabs {
            let load_result = std::panic::catch_unwind(|| load_document(&tab_session.file_path));

            if let Ok(Ok(backend)) = load_result {
                let id = next_tab_id;
                next_tab_id += 1;

                let tab = RuntimeTab::new(
                    id,
                    tab_session.file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    tab_session.file_path.clone(),
                    backend,
                    tab_session.current_page,
                    tab_session.zoom,
                    tab_session.layout,
                    tab_session.is_continuous,
                    tab_session.is_side_panel_open,
                    tab_session.is_side_panel_pinned,
                    tab_session.side_panel_tab,
                );

                restored_tabs.push(tab);
            }
        }

        let active_tab_id = if !restored_tabs.is_empty() {
            restored_tabs.iter().find(|t| t.id == session.active_tab_index).map(|t| t.id).or_else(|| restored_tabs.first().map(|t| t.id))
        } else {
            None
        };

        let mut app = Self {
            settings: session.settings,
            tabs: restored_tabs,
            active_tab_id,
            next_tab_id,
            split_secondary_tab_id: None,
            dragged_tab_id: None,
            is_settings_open: false,
            remapping_action: None,
            active_modifiers: Modifiers::default(),
            is_fullscreen: false,
            is_tab_bar_visible: true,
            error_message: None,
        };

        app.purge_all_inactive_tabs();

        let mut initial_tasks = Vec::new();
        if let Some(active_id) = app.active_tab_id {
            initial_tasks.push(app.request_missing_page_renders(active_id));

            let active_info = app.tabs.iter().find(|t| t.id == active_id).map(|t| (
                t.is_side_panel_open,
                t.side_panel_tab,
                t.is_continuous,
                t.current_page,
            ));

            if let Some((side_open, side_tab, continuous, current_page)) = active_info {
                if side_open && side_tab == crate::models::session::SidePanelTab::Thumbnails {
                    initial_tasks.push(app.request_missing_thumbnail_renders(active_id));
                }

                if continuous && current_page > 0 {
                    let target_y = app.tabs.iter().find(|t| t.id == active_id).unwrap().y_offset_for_page(current_page);

                    initial_tasks.push(iced::widget::scrollable::scroll_to(
                        iced::widget::scrollable::Id::new(format!("viewer_scroll_{}", active_id)),
                        iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: target_y },
                    ));
                }
            }
        }

        (app, Task::batch(initial_tasks))
    }

    /// Smart Multi-Tab Memory Management:
    /// - Keeps recently viewed tabs (<3 mins) 100% intact in RAM for instant 0ms tab switching.
    /// - Only evicts background tabs if total RAM allocation across all tabs exceeds 60 MB
    ///   or if a tab has been untouched for > 3 minutes.
    pub fn purge_all_inactive_tabs(&mut self) {
        let active_id = self.active_tab_id;
        let now = Instant::now();

        // 1. Enforce active tab budget
        if let Some(active_id) = active_id {
            if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == active_id) {
                tab.enforce_memory_budget();
            }
        }

        // 2. Check total RAM usage across all open tabs
        let total_ram: usize = self.tabs.iter().map(|t| t.total_texture_ram_bytes()).sum();

        if total_ram > TOTAL_APP_RAM_BUDGET_BYTES {
            // Evict oldest background tabs first until memory drops below budget
            let mut inactive_indices: Vec<usize> = self.tabs.iter()
                .enumerate()
                .filter(|(_, t)| Some(t.id) != active_id)
                .map(|(idx, _)| idx)
                .collect();

            inactive_indices.sort_by_key(|&idx| self.tabs[idx].last_accessed);

            for idx in inactive_indices {
                let current_total: usize = self.tabs.iter().map(|t| t.total_texture_ram_bytes()).sum();
                if current_total <= TOTAL_APP_RAM_BUDGET_BYTES {
                    break;
                }

                let elapsed = now.duration_since(self.tabs[idx].last_accessed).as_secs();
                if elapsed < INACTIVE_COOLDOWN_SECS {
                    self.tabs[idx].retain_only_current_page();
                } else {
                    self.tabs[idx].offload_completely_from_ram();
                }
            }
        } else {
            // Total memory is well within budget: Only offload truly stale tabs (> 3 mins)
            for tab in &mut self.tabs {
                if Some(tab.id) != active_id {
                    let elapsed = now.duration_since(tab.last_accessed).as_secs();
                    if elapsed >= INACTIVE_COOLDOWN_SECS {
                        tab.offload_completely_from_ram();
                    }
                }
            }
        }

        trim_memory();
    }

    pub fn theme(&self) -> Theme {
        get_iced_theme(&self.settings.theme)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen().map(Message::EventOccurred)
    }

    pub fn save_session(&self) {
        let mut session = SessionData::load();
        session.settings = self.settings.clone();

        session.open_tabs = self.tabs.iter().map(|t| crate::models::session::TabSession {
            file_path: t.file_path.clone(),
            current_page: t.current_page,
            zoom: t.zoom,
            layout: t.layout,
            is_continuous: t.is_continuous,
            is_side_panel_open: t.is_side_panel_open,
            is_side_panel_pinned: t.is_side_panel_pinned,
            side_panel_tab: t.side_panel_tab,
        }).collect();

        let useful_entries: Vec<UsefulCacheEntry> = self.tabs.iter().map(|t| UsefulCacheEntry {
            file_path: t.file_path.clone(),
            current_page: t.current_page,
        }).collect();

        get_disk_cache().retain_useful_cache(&useful_entries);

        for t in &self.tabs {
            session.file_history.insert(t.file_path.clone(), FileHistoryRecord {
                current_page: t.current_page,
                zoom: t.zoom,
                layout: t.layout,
                is_continuous: t.is_continuous,
            });
        }

        session.active_tab_index = self.active_tab_id.unwrap_or(0);
        let _ = session.save();
        trim_memory();
    }
}