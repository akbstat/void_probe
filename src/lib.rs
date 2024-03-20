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
const PROCESS: &str = "process";
const RESULT: &str = "result";

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

    let process_dir = temp.join(PROCESS);
    let result_dir = temp.join(RESULT);

    make_sure_dir_existed(&process_dir)?;
    fs::remove_dir_all(&process_dir)?;
    make_sure_dir_existed(&result_dir)?;

    for rtf in rtfs.iter() {
        if !rtf.exists() || rtf.is_dir() {
            todo!()
        }
        if let Some(divider) = RTFDivider::new(&rtf)? {
            divider.set_pagesize(PAGE_SIZE).divide(&process_dir)?;
        }
    }

    let converter = PDFConverter::new(&process_dir)?;
    converter.convert()?;

    let combiner = PDFCombiner::new(&process_dir)?;
    combiner.combine_output(&result_dir)?;

    let pdfs = find_pdf_in_dir(&result_dir)?;

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

fn make_sure_dir_existed(p: &Path) -> Result<()> {
    if !p.exists() {
        fs::create_dir_all(p)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::Path};

    use super::*;
    #[test]
    fn probe_test() {
        const WORKER_NUMBER_ENV: &str = "MK_WORD_WORKER";
        const SCRIPT_PATH: &str = "MK_TEMP_SCRIPT";
        env::set_var(WORKER_NUMBER_ENV, "5");
        env::set_var(
            SCRIPT_PATH,
            r"D:\Users\yuqi01.chen\.temp\app\mobiuskit\void_probe",
        );
        let dir = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\测试");
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
