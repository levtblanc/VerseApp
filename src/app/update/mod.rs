pub mod navigation;
pub mod renders;
pub mod settings;
pub mod side_panel;
pub mod tabs;

use iced::Task;
use crate::app::messages::Message;
use crate::app::state::ReaderApp;

impl ReaderApp {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // Tab Management
            Message::OpenFileRequested => self.handle_open_file_requested(),
            Message::FileOpened(result) => self.handle_file_opened(result),
            Message::SelectTab(id) => self.handle_select_tab(id),
            Message::CloseTab(id) => self.handle_close_tab(id),
            Message::SplitViewRequested(id, mode) => self.handle_split_view_requested(id, mode),
            Message::StartTabDrag(id) => self.handle_start_tab_drag(id),
            Message::TabDraggedOver(id) => self.handle_tab_dragged_over(id),
            Message::EndTabDrag => self.handle_end_tab_drag(),

            // Navigation, Zoom & Input
            Message::ChangePage(tab_id, page) => self.handle_change_page(tab_id, page),
            Message::PageInputChanged(tab_id, val) => self.handle_page_input_changed(tab_id, val),
            Message::PageInputSubmitted(tab_id) => self.handle_page_input_submitted(tab_id),
            Message::ChangeZoom(tab_id, zoom) => self.handle_change_zoom(tab_id, zoom),
            Message::ZoomSettled { tab_id, sequence } => self.handle_zoom_settled(tab_id, sequence),
            Message::ViewportScrolled { tab_id, offset_y } => self.handle_viewport_scrolled(tab_id, offset_y),
            Message::ScrollSettled { tab_id, sequence } => self.handle_scroll_settled(tab_id, sequence),
            Message::TogglePageLayout(tab_id) => self.handle_toggle_page_layout(tab_id),
            Message::ToggleContinuous(tab_id) => self.handle_toggle_continuous(tab_id),

            // Text Selection & Clipboard
            Message::StartTextSelection { page_index, x, y } => self.handle_start_text_selection(page_index, x, y),
            Message::UpdateTextSelection { page_index, x, y } => self.handle_update_text_selection(page_index, x, y),
            Message::EndTextSelection => self.handle_end_text_selection(),
            Message::CopySelectedText => self.handle_copy_selected_text(),

            // Document Search
            Message::ToggleSearch => self.handle_toggle_search(),
            Message::CloseSearch => self.handle_close_search(),
            Message::SearchQueryChanged(query) => self.handle_search_query_changed(query),
            Message::SearchCompleted { tab_id, query, matches } => self.handle_search_completed(tab_id, query, matches),
            Message::ToggleSearchMatchCase => self.handle_toggle_search_match_case(),
            Message::NextSearchMatch => self.handle_next_search_match(),
            Message::PrevSearchMatch => self.handle_prev_search_match(),

            // Side Panel
            Message::ToggleSidePanel(tab_id) => self.handle_toggle_side_panel(tab_id),
            Message::ToggleSidePanelPin(tab_id) => self.handle_toggle_side_panel_pin(tab_id),
            Message::SetSidePanelTab(tab_id, tab) => self.handle_set_side_panel_tab(tab_id, tab),
            Message::SidePanelScrolled { tab_id, offset_y } => self.handle_side_panel_scrolled(tab_id, offset_y),

            // UI & Settings
            Message::ToggleFullscreen => self.handle_toggle_fullscreen(),
            Message::ApplyWindowMode(id, mode) => self.handle_apply_window_mode(id, mode),
            Message::ToggleTabBar => self.handle_toggle_tab_bar(),
            Message::OpenSettings => self.handle_open_settings(),
            Message::CloseSettings => self.handle_close_settings(),
            Message::StartRemapping(action) => self.handle_start_remapping(action),
            Message::ToggleTheme => self.handle_toggle_theme(),
            Message::ToggleNightMode => self.handle_toggle_night_mode(),
            Message::ClearError => self.handle_clear_error(),
            Message::EventOccurred(event) => self.handle_event_occurred(event),

            // Render Task Completion
            Message::PageRenderFinished { tab_id, page_index, quality, result } => {
                self.handle_page_render_finished(tab_id, page_index, quality, result)
            }
            Message::ThumbnailRenderFinished { tab_id, page_index, result } => {
                self.handle_thumbnail_render_finished(tab_id, page_index, result)
            }
        }
    }
}