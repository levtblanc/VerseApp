use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality, TocItem};
use image::RgbaImage;
use mupdf::{Colorspace, Document, Matrix, Outline};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct ThreadSafeDocument(pub Mutex<Document>);

unsafe impl Send for ThreadSafeDocument {}
unsafe impl Sync for ThreadSafeDocument {}

pub struct MuPdfBackend {
    file_name: String,
    total_pages: usize,
    dimensions_cache: Vec<(f32, f32)>, // Pre-computed lock-free dimensions cache
    doc: Arc<ThreadSafeDocument>,
}

impl MuPdfBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let path_str = path.to_str().ok_or("Invalid Unicode file path")?;

        let doc = Document::open(path_str)
            .map_err(|e| format!("MuPDF failed to open '{}': {}", path.display(), e))?;

        let total_pages = doc.page_count().unwrap_or(1) as usize;

        // Pre-compute page dimensions during document open so UI thread never locks Mutex
        let mut dimensions_cache = Vec::with_capacity(total_pages);
        for i in 0..total_pages {
            if let Ok(page) = doc.load_page(i as i32) {
                if let Ok(bounds) = page.bounds() {
                    dimensions_cache.push((bounds.width(), bounds.height()));
                } else {
                    dimensions_cache.push((595.0, 842.0));
                }
            } else {
                dimensions_cache.push((595.0, 842.0));
            }
        }

        Ok(Self {
            file_name,
            total_pages,
            dimensions_cache,
            doc: Arc::new(ThreadSafeDocument(Mutex::new(doc))),
        })
    }
}

impl DocumentBackend for MuPdfBackend {
    fn page_count(&self) -> usize {
        self.total_pages
    }

    /// Lock-Free: Reads cached dimensions directly from RAM to prevent UI thread freezes
    fn page_dimensions(&self, page_index: usize) -> (f32, f32) {
        self.dimensions_cache.get(page_index).copied().unwrap_or((595.0, 842.0))
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        // Scope the Mutex lock so it releases immediately after rasterization
        let pixmap = {
            let guard = self.doc.0.lock().map_err(|_| "Failed to acquire MuPDF lock".to_string())?;

            let page = guard
                .load_page(request.page_index as i32)
                .map_err(|e| format!("MuPDF failed to load page {}: {}", request.page_index, e))?;

            let bounds = page.bounds().map_err(|e| e.to_string())?;
            let doc_w = bounds.width().max(1.0);
            let doc_h = bounds.height().max(1.0);

            let quality_multiplier = match request.quality {
                RenderQuality::Fuzzy => 0.2,
                RenderQuality::Draft => 0.55,
                RenderQuality::High => 1.25,
            };

            let desired_scale = request.zoom * quality_multiplier;

            let (max_w, max_h) = request.max_dimensions.unwrap_or((3840, 3840));
            let max_scale_w = max_w as f32 / doc_w;
            let max_scale_h = max_h as f32 / doc_h;

            let total_scale = desired_scale.min(max_scale_w).min(max_scale_h).max(0.05);
            let matrix = Matrix::new(total_scale, 0.0, 0.0, total_scale, 0.0, 0.0);

            page.to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
                .map_err(|e| format!("MuPDF rasterization error on page {}: {}", request.page_index, e))?
        }; // Mutex lock released here!

        let width = pixmap.width();
        let height = pixmap.height();
        let samples = pixmap.samples();
        let n_components = pixmap.n();

        // SIMD-Vectorized RGBA construction without holding Mutex
        let mut rgba_bytes = vec![255u8; (width * height * 4) as usize];

        if n_components == 3 {
            for (src, dst) in samples.chunks_exact(3).zip(rgba_bytes.chunks_exact_mut(4)) {
                dst[0..3].copy_from_slice(src);
            }
        } else if n_components == 4 {
            rgba_bytes.copy_from_slice(samples);
        } else {
            for (i, &b) in samples.iter().enumerate() {
                let offset = i * 4;
                if offset + 3 < rgba_bytes.len() {
                    rgba_bytes[offset] = b;
                    rgba_bytes[offset + 1] = b;
                    rgba_bytes[offset + 2] = b;
                }
            }
        }

        RgbaImage::from_raw(width, height, rgba_bytes)
            .ok_or_else(|| format!("Failed to create RGBA image buffer for page {}", request.page_index))
    }

    fn table_of_contents(&self) -> Vec<TocItem> {
        if let Ok(guard) = self.doc.0.lock() {
            if let Ok(outlines) = guard.outlines() {
                return outlines.iter().map(convert_mupdf_outline).collect();
            }
        }
        Vec::new()
    }
}

impl std::fmt::Debug for MuPdfBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MuPdfBackend")
            .field("file_name", &self.file_name)
            .field("total_pages", &self.total_pages)
            .finish()
    }
}

fn convert_mupdf_outline(node: &Outline) -> TocItem {
    let title = node.title.clone();
    let page_index = node.dest.as_ref().map(|d| d.loc.page_number as usize).unwrap_or(0);
    let children = node.down.iter().map(convert_mupdf_outline).collect();

    TocItem { title, page_index, children }
}