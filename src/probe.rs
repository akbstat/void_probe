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
            let row = row.trim();
            if row.is_empty() {
                continue;
            }
            if !(row.starts_with("康方") || row.starts_with("Akeso")) {
                r.append_void(page_number + 1);
            }
            break;
        }
    }
    Ok(r)
}

#[cfg(test)]
mod test_probe {
    use super::*;
    #[test]
    fn probe_test() {
        let p = Path::new(
            r"D:\Studies\ak112\303\stats\CSR\product\output\.temp\l-16-02-04-08-01-antu-ex-ss.pdf",
        );
        let report = probe(p).unwrap();
        println!("{:?}", report);
        let p = Path::new(
            r"D:\Studies\ak112\303\stats\CSR\product\output\bk\f-14-02-01-04-inve-pfs-for-fas.pdf",
        );
        let report = probe(p).unwrap();
        println!("{:?}", report);
    }
}
