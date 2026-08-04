use iced::window;
use iced::widget::image::Handle;
use std::path::PathBuf;
use std::sync::Arc;
use crate::engine::traits::{DocumentBackend, RenderQuality};
use crate::models::session::SidePanelTab;

#[derive(Debug, Clone)]
pub enum Message {
    OpenFileRequested,
    FileOpened(Result<(PathBuf, Arc<dyn DocumentBackend>), String>),
    SelectTab(usize),
    CloseTab(usize),
    SplitViewRequested(usize, bool),

    ChangePage(usize, usize),
    PageInputChanged(usize, String),
    PageInputSubmitted(usize),
    ChangeZoom(usize, f32),
    TogglePageLayout(usize),
    ToggleContinuous(usize),

    ToggleSidePanel(usize),
    ToggleSidePanelPin(usize),
    SetSidePanelTab(usize, SidePanelTab),

    ViewportScrolled { tab_id: usize, offset_y: f32 },

    ScrollSettled { tab_id: usize, sequence: usize },
    ZoomSettled { tab_id: usize, sequence: usize },

    ToggleFullscreen,
    ToggleTabBar,
    ApplyWindowMode(Option<window::Id>, window::Mode),

    OpenSettings,
    CloseSettings,
    StartRemapping(crate::models::session::Action),
    ToggleTheme,
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