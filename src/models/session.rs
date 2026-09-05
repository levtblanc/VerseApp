use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePanelTab {
    TableOfContents,
    Thumbnails,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    ZoomIn,
    ZoomOut,
    NextPage,
    PrevPage,
    ToggleTheme,
    ToggleNightMode,
    CopySelectedText,
    OpenSearch,
    OpenSettings,
    CloseActiveTab,
    TogglePageLayout,
    ToggleContinuous,
    OpenFile,
    NextTab,
    PrevTab,
    ToggleFullscreen,
    ToggleTabBar,
    ToggleSidePanel,
}

impl Action {
    pub fn display_name(&self) -> &'static str {
        match self {
            Action::ZoomIn => "Zoom In",
            Action::ZoomOut => "Zoom Out",
            Action::NextPage => "Next Page",
            Action::PrevPage => "Previous Page",
            Action::ToggleTheme => "Toggle UI Theme (Light / Dark)",
            Action::ToggleNightMode => "Toggle Page Night Mode (ON / OFF)",
            Action::CopySelectedText => "Copy Selected Text (Ctrl+C)",
            Action::OpenSearch => "Search Document (Ctrl+F)",
            Action::OpenSettings => "Open Settings",
            Action::CloseActiveTab => "Close Active Tab",
            Action::TogglePageLayout => "Toggle Single/Double View",
            Action::ToggleContinuous => "Toggle Continuous Scrolling (Ctrl+Shift+C)",
            Action::OpenFile => "Open File",
            Action::NextTab => "Next Tab (Ctrl+Tab)",
            Action::PrevTab => "Previous Tab (Ctrl+Shift+Tab)",
            Action::ToggleFullscreen => "Toggle Full Screen (F11)",
            Action::ToggleTabBar => "Toggle Tab Bar (Ctrl+B)",
            Action::ToggleSidePanel => "Toggle Navigation Panel (Ctrl+N)",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub fn new(key: &str, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self {
            key: key.to_string(),
            ctrl,
            shift,
            alt,
        }
    }

    pub fn to_display_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Alt"); }
        if self.shift { parts.push("Shift"); }
        parts.push(&self.key);
        parts.join(" + ")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLayout { Single, Double }

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode { Light, Dark }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub is_night_mode: bool,
    pub default_zoom: f32,
    pub keybindings: HashMap<Action, KeyBinding>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut keybindings = HashMap::new();
        keybindings.insert(Action::OpenFile, KeyBinding::new("O", true, false, false));
        keybindings.insert(Action::OpenSearch, KeyBinding::new("F", true, false, false));
        keybindings.insert(Action::NextTab, KeyBinding::new("Tab", true, false, false));
        keybindings.insert(Action::PrevTab, KeyBinding::new("Tab", true, true, false));
        keybindings.insert(Action::CloseActiveTab, KeyBinding::new("W", true, false, false));
        keybindings.insert(Action::ToggleTabBar, KeyBinding::new("B", true, false, false));
        keybindings.insert(Action::ToggleFullscreen, KeyBinding::new("F11", false, false, false));
        keybindings.insert(Action::ToggleSidePanel, KeyBinding::new("N", true, false, false));
        keybindings.insert(Action::ToggleNightMode, KeyBinding::new("N", true, true, false));
        keybindings.insert(Action::CopySelectedText, KeyBinding::new("C", true, false, false));
        keybindings.insert(Action::ZoomIn, KeyBinding::new("Equal", true, false, false));
        keybindings.insert(Action::ZoomOut, KeyBinding::new("Minus", true, false, false));
        keybindings.insert(Action::NextPage, KeyBinding::new("Down", false, false, false));
        keybindings.insert(Action::PrevPage, KeyBinding::new("Up", false, false, false));
        keybindings.insert(Action::TogglePageLayout, KeyBinding::new("D", true, false, false));
        keybindings.insert(Action::ToggleContinuous, KeyBinding::new("C", true, true, false));
        keybindings.insert(Action::ToggleTheme, KeyBinding::new("T", true, false, false));
        keybindings.insert(Action::OpenSettings, KeyBinding::new("Comma", true, false, false));

        Self {
            theme: ThemeMode::Dark,
            is_night_mode: false,
            default_zoom: 1.0,
            keybindings,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TabSession {
    pub file_path: PathBuf,
    pub current_page: usize,
    pub zoom: f32,
    pub layout: PageLayout,
    pub is_continuous: bool,
    pub is_side_panel_open: bool,
    pub is_side_panel_pinned: bool,
    pub side_panel_tab: SidePanelTab,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileHistoryRecord {
    pub current_page: usize,
    pub zoom: f32,
    pub layout: PageLayout,
    pub is_continuous: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SessionData {
    pub settings: AppSettings,
    pub open_tabs: Vec<TabSession>,
    pub active_tab_index: usize,
    pub layout: Option<String>,
    #[serde(default)]
    pub file_history: HashMap<PathBuf, FileHistoryRecord>,
}

impl SessionData {
    pub fn config_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("rust_reader");
        fs::create_dir_all(&p).ok();
        p.push("session.json");
        p
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut session: Self = if path.exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        };

        let defaults = AppSettings::default();
        for (action, binding) in defaults.keybindings {
            session.settings.keybindings.entry(action).or_insert(binding);
        }

        session
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }
}
