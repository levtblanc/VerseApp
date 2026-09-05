pub mod traits;
pub mod mupdf;
pub mod djvu;
pub mod docx;

use std::path::Path;
use std::sync::Arc;
use crate::engine::traits::DocumentBackend;
use crate::engine::mupdf::MuPdfBackend;
use crate::engine::djvu::DjVuBackend;
use crate::engine::docx::DocxBackend;

pub fn load_document(path: &Path) -> Result<Arc<dyn DocumentBackend>, String> {
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "djvu" => {
            let backend = DjVuBackend::open(path)?;
            Ok(Arc::new(backend))
        }
        "docx" => {
            let backend = DocxBackend::open(path)?;
            Ok(Arc::new(backend))
        }
        _ => {
            let backend = MuPdfBackend::open(path)?;
            Ok(Arc::new(backend))
        }
    }
}
