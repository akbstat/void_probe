// 1. determine if rtf need to be divided - (read metadata to get file size)
// 2. convert rtf to pdf, and merge then with original sequence
// 3. read content of pdf
// 4. check if first row of each page is a title, if false, means break happends
// 5. remove the pdf and divided rtf files

use anyhow::Result;
use pdf::{combine::PDFCombiner, convert::PDFConverter};
use probe::probe;
use report::Report;
use rtf_divider::RTFDivider;
use std::{
    fs,
    path::{Path, PathBuf},
};

const PAGE_SIZE: usize = 50;

mod pdf;
mod probe;
mod report;

pub fn void_probe(rtfs: &[PathBuf]) -> Result<Vec<Report>> {
    let mut reports = Vec::with_capacity(rtfs.len());
    if rtfs.len() == 0 {
        return Ok(reports);
    }

    let temp = (if let Some(parent) = rtfs.get(0).unwrap().parent() {
        PathBuf::from(parent)
    } else {
        PathBuf::from(".")
    })
    .join(r".temp");

    for rtf in rtfs.iter() {
        if !rtf.exists() || rtf.is_dir() {
            todo!()
        }
        if let Some(divider) = RTFDivider::new(&rtf)? {
            divider.set_pagesize(PAGE_SIZE).divide(temp.as_path())?;
        }
    }

    let converter = PDFConverter::new(&temp)?;
    converter.convert()?;

    let combiner = PDFCombiner::new(&temp)?;
    combiner.combine_output(&temp)?;

    let pdfs = find_pdf_in_dir(temp.as_path())?;

    for (_, pdf) in pdfs.iter().enumerate() {
        let report = probe(pdf)?;
        reports.push(report);
    }

    Ok(reports)
}

fn find_pdf_in_dir(p: &Path) -> Result<Vec<PathBuf>> {
    let mut pdfs = vec![];
    let root = PathBuf::from(p);
    for entry in fs::read_dir(p)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".pdf") {
            pdfs.push(root.clone().join(name));
        }
    }
    Ok(pdfs)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    #[test]
    fn probe_test() {
        let dir = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output");
        let mut rtfs = vec![];
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".rtf") {
                rtfs.push(dir.join(name));
            }
        }

        let r = void_probe(&rtfs).unwrap();
        println!("{:?}", r);
    }
}
