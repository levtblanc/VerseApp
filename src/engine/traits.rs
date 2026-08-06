use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RenderQuality {
    Fuzzy, // 0.20x scale: <1ms pre-render
    Draft, // 0.55x scale: Fast 60 FPS scrolling
    High,  // 1.25x scale: Sharp high-DPI text
}

#[derive(Debug, Clone)]
pub struct PageRenderRequest {
    pub page_index: usize,
    pub zoom: f32,
    pub rotation: u16,
    pub quality: RenderQuality,
    pub max_dimensions: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
    pub title: String,
    pub page_index: usize,
    pub children: Vec<TocItem>,
}

pub trait DocumentBackend: Send + Sync + std::fmt::Debug {
    fn page_count(&self) -> usize;
    fn page_dimensions(&self, page_index: usize) -> (f32, f32);
    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String>;
    fn table_of_contents(&self) -> Vec<TocItem>;
}