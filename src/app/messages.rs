use iced::window;
use iced::widget::image::Handle;
use std::path::PathBuf;
use std::sync::Arc;
use crate::engine::traits::{DocumentBackend, RenderQuality};
use crate::models::session::SidePanelTab;
use crate::models::workspace::SearchMatch;

#[derive(Debug, Clone)]
pub enum Message {
    OpenFileRequested,
    FileOpened(Result<(PathBuf, Arc<dyn DocumentBackend>), String>),
    SelectTab(usize),
    CloseTab(usize),
    SplitViewRequested(usize, bool),

    StartTabDrag(usize),
    TabDraggedOver(usize),
    EndTabDrag,

    ChangePage(usize, usize),
    PageInputChanged(usize, String),
    PageInputSubmitted(usize),
    ChangeZoom(usize, f32),
    TogglePageLayout(usize),
    ToggleContinuous(usize),

    // Text Selection & Clipboard Messages
    StartTextSelection { page_index: usize, x: f32, y: f32 },
    UpdateTextSelection { page_index: usize, x: f32, y: f32 },
    EndTextSelection,
    CopySelectedText,

    // Document Search Messages
    ToggleSearch,
    CloseSearch,
    SearchQueryChanged(String),
    SearchCompleted { tab_id: usize, query: String, matches: Vec<SearchMatch> },
    ToggleSearchMatchCase,
    NextSearchMatch,
    PrevSearchMatch,

    ToggleSidePanel(usize),
    ToggleSidePanelPin(usize),
    SetSidePanelTab(usize, SidePanelTab),

    ViewportScrolled { tab_id: usize, offset_y: f32, viewport_width: f32, viewport_height: f32 },
    SidePanelScrolled { tab_id: usize, offset_y: f32 },

    ScrollSettled { tab_id: usize, sequence: usize },
    ZoomSettled { tab_id: usize, sequence: usize },

    ToggleFullscreen,
    ToggleTabBar,
    ApplyWindowMode(Option<window::Id>, window::Mode),

    OpenSettings,
    CloseSettings,
    StartRemapping(crate::models::session::Action),
    ToggleTheme,
    ToggleNightMode,
    ClearError,

    EventOccurred(iced::Event),

    PageRenderFinished {
        tab_id: usize,
        page_index: usize,
        quality: RenderQuality,
        result: Result<(Handle, u32, u32), String>,
    },
    ThumbnailRenderFinished {
        tab_id: usize,
        page_index: usize,
        result: Result<Handle, String>,
    },
}
