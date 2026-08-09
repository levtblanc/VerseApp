use crate::engine::mupdf::MuPdfBackend;
use crate::engine::traits::{DocumentBackend, PageRenderRequest, TocItem};
use image::RgbaImage;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct DocxBackend {
    file_name: String,
    mupdf_backend: MuPdfBackend,
    temp_dir: PathBuf,
}

impl DocxBackend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        let file = File::open(path).map_err(|e| format!("Failed to open DOCX file '{}': {}", path.display(), e))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid DOCX archive '{}': {}", path.display(), e))?;

        let temp_dir = std::env::temp_dir().join(format!("verse_docx_{}", uuid_simple()));
        fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let mut rels_map: HashMap<String, String> = HashMap::new();
        if let Ok(mut rels_file) = archive.by_name("word/_rels/document.xml.rels") {
            let mut rels_bytes = Vec::new();
            if rels_file.read_to_end(&mut rels_bytes).is_ok() {
                let rels_xml = String::from_utf8_lossy(&rels_bytes);
                for Relationship in rels_xml.split("<Relationship ") {
                    let id = extract_attr(Relationship, "Id");
                    let target = extract_attr(Relationship, "Target");
                    if !id.is_empty() && !target.is_empty() {
                        rels_map.insert(id, target);
                    }
                }
            }
        }

        for i in 0..archive.len() {
            if let Ok(mut zip_file) = archive.by_index(i) {
                let name = zip_file.name().to_string();
                if name.starts_with("word/media/") {
                    if let Some(file_name) = Path::new(&name).file_name() {
                        let dest_path = temp_dir.join(file_name);
                        if let Ok(mut out_file) = File::create(&dest_path) {
                            let _ = std::io::copy(&mut zip_file, &mut out_file);
                        }
                    }
                }
            }
        }

        let mut body_html = String::new();
        if let Ok(mut doc_file) = archive.by_name("word/document.xml") {
            let mut doc_bytes = Vec::new();
            if doc_file.read_to_end(&mut doc_bytes).is_ok() {
                let xml = String::from_utf8_lossy(&doc_bytes);
                body_html = parse_docx_xml_to_html(&xml, &rels_map);
            }
        }

        if body_html.trim().is_empty() {
            body_html.push_str("<p><em>Empty Word Document</em></p>");
        }

        let full_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    body {{
      font-family: 'Noto Sans', 'Arial', sans-serif;
      font-size: 13pt;
      line-height: 1.6;
      color: #1a1a1e;
      background-color: #ffffff;
      padding: 40px;
      max-width: 750px;
      margin: 0 auto;
    }}
    h1 {{ font-size: 22pt; color: #1e2b48; margin-top: 1.2em; margin-bottom: 0.5em; border-bottom: 2px solid #e0e4ec; padding-bottom: 6px; }}
    h2 {{ font-size: 17pt; color: #2a3c66; margin-top: 1.1em; margin-bottom: 0.4em; }}
    h3 {{ font-size: 14pt; color: #384e7a; margin-top: 1.0em; margin-bottom: 0.3em; }}
    p {{ margin-top: 0; margin-bottom: 1.0em; text-align: justify; }}
    b, strong {{ font-weight: bold; color: #000000; }}
    i, em {{ font-style: italic; }}
    u {{ text-decoration: underline; }}
    ul, ol {{ margin-top: 0; margin-bottom: 1em; padding-left: 24px; }}
    li {{ margin-bottom: 0.3em; }}
    img {{ max-width: 100%; height: auto; display: block; margin: 1.2em auto; border-radius: 4px; filter: none !important; }}
    table {{ border-collapse: collapse; width: 100%; margin: 1.2em 0; }}
    td, th {{ border: 1px solid #d0d4dc; padding: 8px 12px; text-align: left; }}
    th {{ background-color: #f0f3f8; font-weight: bold; }}
  </style>
</head>
<body>
  {body}
</body>
</html>"#,
            body = body_html
        );

        let html_path = temp_dir.join("index.html");
        fs::write(&html_path, full_html).map_err(|e| format!("Failed to write temp HTML: {}", e))?;

        let mupdf_backend = MuPdfBackend::open(&html_path)?;

        Ok(Self {
            file_name,
            mupdf_backend,
            temp_dir,
        })
    }
}

impl DocumentBackend for DocxBackend {
    fn page_count(&self) -> usize {
        self.mupdf_backend.page_count()
    }

    fn page_dimensions(&self, page_index: usize) -> (f32, f32) {
        self.mupdf_backend.page_dimensions(page_index)
    }

    fn render_page(&self, request: &PageRenderRequest) -> Result<RgbaImage, String> {
        self.mupdf_backend.render_page(request)
    }

    fn table_of_contents(&self) -> Vec<TocItem> {
        self.mupdf_backend.table_of_contents()
    }
}

impl Drop for DocxBackend {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
        crate::models::workspace::trim_memory();
    }
}

impl std::fmt::Debug for DocxBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocxBackend")
            .field("file_name", &self.file_name)
            .field("temp_dir", &self.temp_dir)
            .finish()
    }
}

fn uuid_simple() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(12345678)
}

fn extract_attr(chunk: &str, attr_name: &str) -> String {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = chunk.find(&pattern) {
        let val_part = &chunk[start + pattern.len()..];
        if let Some(end) = val_part.find('"') {
            return val_part[..end].to_string();
        }
    }
    String::new()
}

fn escape_html(input: &str) -> String {
    input.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_docx_xml_to_html(xml: &str, rels: &HashMap<String, String>) -> String {
    let mut html = String::new();

    for p_chunk in xml.split("<w:p") {
        if p_chunk.trim().is_empty() { continue; }

        let is_h1 = p_chunk.contains("Heading1") || p_chunk.contains("heading 1");
        let is_h2 = p_chunk.contains("Heading2") || p_chunk.contains("heading 2");
        let is_h3 = p_chunk.contains("Heading3") || p_chunk.contains("heading 3");

        let tag = if is_h1 { "h1" } else if is_h2 { "h2" } else if is_h3 { "h3" } else { "p" };

        let mut p_content = String::new();

        if p_chunk.contains("<w:drawing") || p_chunk.contains("r:embed=") {
            for id in rels.keys() {
                if p_chunk.contains(&format!("r:embed=\"{}\"", id)) || p_chunk.contains(&format!("r:id=\"{}\"", id)) {
                    if let Some(target) = rels.get(id) {
                        if let Some(img_filename) = Path::new(target).file_name() {
                            p_content.push_str(&format!("<img src=\"{}\" />", img_filename.to_string_lossy()));
                        }
                    }
                }
            }
        }

        for r_chunk in p_chunk.split("<w:r") {
            if r_chunk.trim().is_empty() { continue; }

            let is_bold = r_chunk.contains("<w:b/>") || r_chunk.contains("<w:b ");
            let is_italic = r_chunk.contains("<w:i/>") || r_chunk.contains("<w:i ");
            let is_underline = r_chunk.contains("<w:u ") || r_chunk.contains("<w:u/>");

            let mut run_text = String::new();
            for t_chunk in r_chunk.split("<w:t") {
                if let Some(end_tag) = t_chunk.find('>') {
                    let content = &t_chunk[end_tag + 1..];
                    if let Some(close) = content.find("</w:t>") {
                        run_text.push_str(&escape_html(&content[..close]));
                    }
                }
            }

            if !run_text.is_empty() {
                let mut formatted = run_text;
                if is_bold { formatted = format!("<b>{}</b>", formatted); }
                if is_italic { formatted = format!("<i>{}</i>", formatted); }
                if is_underline { formatted = format!("<u>{}</u>", formatted); }
                p_content.push_str(&formatted);
            }
        }

        if !p_content.trim().is_empty() {
            html.push_str(&format!("<{tag}>{content}</{tag}>\n", tag = tag, content = p_content));
        }
    }

    html
}