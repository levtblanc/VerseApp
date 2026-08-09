use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality, TextQuad, TocItem};
use image::RgbaImage;
use mupdf::{Colorspace, Document, Matrix, Outline, TextExtractOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct ThreadSafeDocument(pub Mutex<Document>);

unsafe impl Send for ThreadSafeDocument {}
unsafe impl Sync for ThreadSafeDocument {}

pub struct MuPdfBackend {
    file_name: String,
    total_pages: usize,
    default_dimensions: (f32, f32),
    dimensions_cache: Mutex<HashMap<usize, (f32, f32)>>,
    doc: Arc<ThreadSafeDocument>,
    is_image_based: bool,
}

impl MuPdfBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let path_str = path.to_str().ok_or("Invalid Unicode file path")?;

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_image_based = matches!(
            ext.as_str(),
            "cbz" | "cbr" | "cb7" | "cbt" | "png" | "jpg" | "jpeg" | "webp"
        );

        let doc = Document::open(path_str)
            .map_err(|e| format!("MuPDF failed to open '{}': {}", path.display(), e))?;

        let total_pages = doc.page_count().unwrap_or(1) as usize;

        let default_dimensions = if let Ok(page) = doc.load_page(0) {
            if let Ok(bounds) = page.bounds() {
                (bounds.width(), bounds.height())
            } else {
                (595.0, 842.0)
            }
        } else {
            (595.0, 842.0)
        };

        Ok(Self {
            file_name,
            total_pages,
            default_dimensions,
            dimensions_cache: Mutex::new(HashMap::new()),
            doc: Arc::new(ThreadSafeDocument(Mutex::new(doc))),
            is_image_based,
        })
    }
}

pub fn apply_smart_night_mode_filter(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let r = pixel[0] as i16;
        let g = pixel[1] as i16;
        let b = pixel[2] as i16;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let saturation = max_c - min_c;

        if saturation < 35 {
            pixel[0] = (255 - r) as u8;
            pixel[1] = (255 - g) as u8;
            pixel[2] = (255 - b) as u8;
        } else {
            pixel[0] = ((r * 9) / 10) as u8;
            pixel[1] = ((g * 9) / 10) as u8;
            pixel[2] = ((b * 9) / 10) as u8;
        }
    }
}

impl DocumentBackend for MuPdfBackend {
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
        self.is_image_based
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        let pixmap = {
            let guard = self.doc.0.lock().map_err(|_| "Failed to acquire MuPDF lock".to_string())?;

            let page = guard
                .load_page(request.page_index as i32)
                .map_err(|e| format!("MuPDF failed to load page {}: {}", request.page_index, e))?;

            let bounds = page.bounds().map_err(|e| e.to_string())?;
            let doc_w = bounds.width().max(1.0);
            let doc_h = bounds.height().max(1.0);

            if let Ok(mut cache_guard) = self.dimensions_cache.lock() {
                cache_guard.insert(request.page_index, (bounds.width(), bounds.height()));
            }

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
        };

        let width = pixmap.width();
        let height = pixmap.height();
        let samples = pixmap.samples();
        let n_components = pixmap.n();

        // SIMD-Vectorized RGBA Buffer Expansion
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

        if request.is_night_mode && !request.is_image_based && !self.is_image_based {
            apply_smart_night_mode_filter(&mut rgba_bytes);
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

    fn extract_text(&self, page_index: usize) -> Vec<TextQuad> {
        let mut quads = Vec::new();
        if let Ok(guard) = self.doc.0.lock() {
            if let Ok(page) = guard.load_page(page_index as i32) {
                if let Ok(words) = page.words(TextExtractOptions::default()) {
                    for word in words {
                        let text = word.text.trim().to_string();
                        if !text.is_empty() {
                            quads.push(TextQuad {
                                text,
                                x0: word.bounds.x0,
                                y0: word.bounds.y0,
                                x1: word.bounds.x1,
                                y1: word.bounds.y1,
                            });
                        }
                    }
                }
            }
        }
        quads
    }
}

impl Drop for MuPdfBackend {
    fn drop(&mut self) {
        crate::models::workspace::trim_memory();
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