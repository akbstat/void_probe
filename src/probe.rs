use std::path::Path;

use anyhow::Ok;

use crate::{pdf::reader::PDFReader, report::Report};

pub fn probe(pdf_path: &Path) -> anyhow::Result<Report> {
    let reader = PDFReader::new(&pdf_path)?;
    let r = Report::new(pdf_path.to_string_lossy().to_string().as_str());
    for (page_number, page) in reader.content().iter().enumerate() {
        if page.is_empty() {
            r.append_void(page_number + 1);
            continue;
        }
        for row in page {
            if row.is_empty() {
                continue;
            }
            for word in row {
                if word.is_empty() {
                    continue;
                }
                if !(word.starts_with("康方") || word.starts_with("Akeso")) {
                    r.append_void(page_number + 1);
                }
                break;
            }
            break;
        }
    }
    Ok(r)
}
