use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextQuad {
    pub text: String,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RenderQuality {
    Fuzzy,
    Draft,
    High,
}

#[derive(Debug, Clone)]
pub struct PageRenderRequest {
    pub page_index: usize,
    pub zoom: f32,
    pub rotation: u16,
    pub quality: RenderQuality,
    pub max_dimensions: Option<(u32, u32)>,
    pub is_night_mode: bool,
    pub is_image_based: bool,
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
    fn is_image_based(&self) -> bool { false }
    fn extract_text(&self, _page_index: usize) -> Vec<TextQuad> { Vec::new() }
}