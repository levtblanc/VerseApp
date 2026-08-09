use crate::engine::mupdf::apply_smart_night_mode_filter;
use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality, TocItem};
use image::imageops::FilterType;
use image::RgbaImage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

pub struct DjVuBackend {
    file_name: String,
    total_pages: usize,
    default_dimensions: (f32, f32),
    dimensions_cache: Mutex<HashMap<usize, (f32, f32)>>,
    doc: Mutex<djvu::Document>,
}

impl DjVuBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let path_str = path.to_str().ok_or("Invalid Unicode file path")?;

        let doc = djvu::Document::open(path_str)
            .map_err(|e| format!("Failed to open DjVu file: {}", e))?;

        let total_pages = doc.page_count() as usize;

        let default_dimensions = if let Ok(page) = doc.page(0) {
            (page.width() as f32, page.height() as f32)
        } else {
            (600.0, 800.0)
        };

        Ok(Self {
            file_name,
            total_pages,
            default_dimensions,
            dimensions_cache: Mutex::new(HashMap::new()),
            doc: Mutex::new(doc),
        })
    }
}

impl DocumentBackend for DjVuBackend {
    fn page_count(&self) -> usize {
        self.total_pages
    }

    fn page_dimensions(&self, page_index: usize) -> (f32, f32) {
        if let Ok(guard) = self.dimensions_cache.lock() {
            if let Some(&dim) = guard.get(&page_index) {
                return dim;
            }
        }
        self.default_dimensions
    }

    fn is_image_based(&self) -> bool {
        false
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        let guard = self.doc.lock().map_err(|_| "Failed to acquire DjVu lock".to_string())?;

        let page = guard.page(request.page_index)
            .map_err(|e| format!("Failed to load DjVu page {}: {}", request.page_index, e))?;

        let (doc_w, doc_h) = (page.width() as f32, page.height() as f32);

        if let Ok(mut cache_guard) = self.dimensions_cache.lock() {
            cache_guard.insert(request.page_index, (doc_w, doc_h));
        }

        let quality_scale = match request.quality {
            RenderQuality::Fuzzy => 0.20,
            RenderQuality::Draft => 0.50,
            RenderQuality::High => 1.25, // Reverted to 1.25x scale
        };

        let target_zoom = (request.zoom * quality_scale).clamp(0.1, 2.5);
        let (max_w, max_h) = request.max_dimensions.unwrap_or((3840, 3840)); // Reverted to 3840x3840

        let target_w = ((doc_w * target_zoom) as u32).clamp(60, max_w);
        let target_h = ((doc_h * target_zoom) as u32).clamp(60, max_h);

        let pixmap = page.render()
            .map_err(|e| format!("DjVu rendering error: {}", e))?;

        let src_w = pixmap.width as u32;
        let src_h = pixmap.height as u32;
        let raw_rgba = RgbaImage::from_raw(src_w, src_h, pixmap.data)
            .ok_or_else(|| "Failed to create RGBA buffer for DjVu page".to_string())?;

        let filter = match request.quality {
            RenderQuality::Fuzzy | RenderQuality::Draft => FilterType::Nearest,
            RenderQuality::High => FilterType::Triangle,
        };

        let mut final_rgba = if src_w > (target_w as f32 * 1.1) as u32 || src_h > (target_h as f32 * 1.1) as u32 {
            image::imageops::resize(&raw_rgba, target_w, target_h, filter)
        } else {
            raw_rgba
        };

        if request.is_night_mode && !request.is_image_based {
            apply_smart_night_mode_filter(&mut final_rgba);
        }

        Ok(final_rgba)
    }

    fn table_of_contents(&self) -> Vec<TocItem> {
        Vec::new()
    }
}

impl Drop for DjVuBackend {
    fn drop(&mut self) {
        crate::models::workspace::trim_memory();
    }
}

impl std::fmt::Debug for DjVuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjVuBackend")
            .field("file_name", &self.file_name)
            .field("total_pages", &self.total_pages)
            .finish()
    }
}