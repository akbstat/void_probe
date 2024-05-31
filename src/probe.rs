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
        for (row_number, row) in page.iter().enumerate() {
            if row.is_empty() {
                continue;
            }
            let row = row.trim();
            if row.is_empty() {
                if row_number.eq(&(page.len() - 1)) {
                    r.append_void(page_number + 1);
                }
                continue;
            }
            if !(row.contains("康方") || row.to_uppercase().starts_with("AKESO")) {
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
        let p =
            Path::new(r"D:\Studies\ak105\302\stats\adhoc\product\output\l-16-02-04-03-mh-fas.pdf");
        let report = probe(p).unwrap();
        println!("{:?}", report);
    }
}
