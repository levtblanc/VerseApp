use crate::engine::traits::{DocumentBackend, PageRenderRequest, RenderQuality, TocItem};
use image::{Rgba, RgbaImage};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DocxBackend {
    file_name: String,
    paragraphs: Vec<String>,
    total_pages: usize,
    toc: Vec<TocItem>,
}

impl DocxBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mut paragraphs = Vec::new();

        // Extract DOCX text from word/document.xml inside ZIP archive
        if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                if let Ok(mut doc_file) = archive.by_name("word/document.xml") {
                    let mut bytes = Vec::new();
                    if doc_file.read_to_end(&mut bytes).is_ok() {
                        let xml = String::from_utf8_lossy(&bytes);
                        for p_chunk in xml.split("<w:p") {
                            let mut para_text = String::new();
                            for t_chunk in p_chunk.split("<w:t") {
                                if let Some(end_tag_pos) = t_chunk.find('>') {
                                    let text_content = &t_chunk[end_tag_pos + 1..];
                                    if let Some(close_pos) = text_content.find("</w:t>") {
                                        para_text.push_str(&text_content[..close_pos]);
                                    }
                                }
                            }
                            let trimmed = para_text.trim();
                            if !trimmed.is_empty() {
                                paragraphs.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
        }

        if paragraphs.is_empty() {
            paragraphs.push(format!("Word Document: {}", file_name));
            paragraphs.push("This document contains formatted body text paragraphs.".to_string());
        }

        let total_pages = ((paragraphs.len() + 19) / 20).max(1);

        let mut toc = Vec::new();
        toc.push(TocItem { title: format!("Document: {}", file_name), page_index: 0, children: vec![] });

        Ok(Self { file_name, paragraphs, total_pages, toc })
    }
}

impl DocumentBackend for DocxBackend {
    fn page_count(&self) -> usize {
        self.total_pages
    }

    fn page_dimensions(&self, _page_index: usize) -> (f32, f32) {
        (612.0, 792.0)
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        let quality_scale = match request.quality {
            RenderQuality::Fuzzy => 0.25,
            RenderQuality::Draft => 0.8,
            RenderQuality::High => 1.5,
        };

        let width = (450.0 * request.zoom * quality_scale) as u32;
        let height = (600.0 * request.zoom * quality_scale) as u32;
        let mut img = RgbaImage::new(width.max(20), height.max(20));

        let bg_color = Rgba([252, 252, 254, 255]);    // Crisp Word paper tone
        let text_color = Rgba([40, 44, 52, 255]);     // Dark charcoal text color
        let heading_color = Rgba([40, 90, 180, 255]);  // Blue Word heading accent

        for pixel in img.pixels_mut() {
            *pixel = bg_color;
        }

        // Render Word Document Header Accent Line
        let header_y = 15u32;
        for x in 15..(width.saturating_sub(15)) {
            for y in header_y..(header_y + 3) {
                if x < width && y < height {
                    img.put_pixel(x, y, heading_color);
                }
            }
        }

        // Render Document Paragraph Blocks & Headings
        let start_para = request.page_index * 20;
        let end_para = (start_para + 20).min(self.paragraphs.len());

        let mut current_y = 35u32;

        for i in start_para..end_para {
            if let Some(p) = self.paragraphs.get(i) {
                let is_heading = p.len() < 35;
                let color = if is_heading { heading_color } else { text_color };
                let block_height = if is_heading { 14u32 } else { 8u32 };
                let line_len = (p.len() * 6).min((width.saturating_sub(30)) as usize) as u32;

                for lx in 15..(15 + line_len) {
                    for ly in current_y..(current_y + block_height) {
                        if lx < width && ly < height {
                            img.put_pixel(lx, ly, color);
                        }
                    }
                }

                current_y += block_height + 12;
                if current_y + 30 >= height { break; }
            }
        }

        // Footer line
        let footer_y = height.saturating_sub(20);
        let footer_len = (width / 3).min(100);
        let start_x = (width / 2).saturating_sub(footer_len / 2);

        for fx in start_x..(start_x + footer_len) {
            for fy in footer_y..(footer_y + 2) {
                if fx < width && fy < height {
                    img.put_pixel(fx, fy, heading_color);
                }
            }
        }

        Ok(img)
    }

    fn table_of_contents(&self) -> Vec<TocItem> {
        self.toc.clone()
    }
}

impl std::fmt::Debug for DocxBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocxBackend").field("file_name", &self.file_name).finish()
    }
}