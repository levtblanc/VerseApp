use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality, TocItem};
use image::RgbaImage;
use std::path::Path;

pub struct DjVuBackend {
    doc: djvu::Document,
}

impl DjVuBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let path_str = path.to_str().ok_or("Invalid Unicode file path")?;
        let doc = djvu::Document::open(path_str)
            .map_err(|e| format!("Failed to open DjVu file: {}", e))?;
        Ok(Self { doc })
    }
}

impl DocumentBackend for DjVuBackend {
    fn page_count(&self) -> usize {
        self.doc.page_count() as usize
    }

    fn page_dimensions(&self, page_index: usize) -> (f32, f32) {
        if let Ok(page) = self.doc.page(page_index) {
            return (page.width() as f32, page.height() as f32);
        }
        (600.0, 800.0)
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        let page = self.doc.page(request.page_index)
            .map_err(|e| format!("Failed to load DjVu page {}: {}", request.page_index, e))?;

        let (doc_w, doc_h) = (page.width() as f32, page.height() as f32);

        let quality_scale = match request.quality {
            RenderQuality::Fuzzy => 0.25,
            RenderQuality::Draft => 0.8,
            RenderQuality::High => 1.5,
        };

        let desired_scale = request.zoom * quality_scale;
        let (max_w, max_h) = request.max_dimensions.unwrap_or((3840, 3840));
        let _clamped_scale = desired_scale
            .min(max_w as f32 / doc_w.max(1.0))
            .min(max_h as f32 / doc_h.max(1.0));

        let pixmap = page.render()
            .map_err(|e| format!("DjVu rendering error: {}", e))?;

        RgbaImage::from_raw(pixmap.width as u32, pixmap.height as u32, pixmap.data)
            .ok_or_else(|| "Failed to create RGBA buffer for DjVu page".to_string())
    }

    fn table_of_contents(&self) -> Vec<TocItem> {
        Vec::new()
    }
}

impl std::fmt::Debug for DjVuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjVuBackend").finish()
    }
}