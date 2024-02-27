use anyhow::Result;
use std::{
    env, fs,
    path::{Path, PathBuf},
    thread::spawn,
};

const WORKER_NUMBER_ENV: &str = "MK_WORD_WORKER";

pub struct PDFConverter {
    tasks: Vec<(PathBuf, PathBuf)>,
    worker_number: usize,
}

impl PDFConverter {
    pub fn new(dir: &Path) -> Result<PDFConverter> {
        let worker_number: usize = if let Ok(worker) = env::var(WORKER_NUMBER_ENV) {
            if let Ok(n) = worker.parse::<usize>() {
                n
            } else {
                6
            }
        } else {
            6
        };
        let mut rtfs = vec![];
        let mut tasks = vec![];
        if dir.is_file() {
            rtfs.push(dir);
        } else {
            for entry in fs::read_dir(dir)? {
                if let Ok(entry) = entry {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.metadata()?.is_dir() {
                        continue;
                    }
                    if !name.ends_with(".rtf") {
                        continue;
                    }
                    let mut pdf_name = name.get(..name.len() - 4).unwrap().to_owned();
                    pdf_name.push_str(".pdf");
                    tasks.push((
                        PathBuf::from(dir).join(name),
                        PathBuf::from(dir).join(pdf_name),
                    ))
                }
            }
        }
        Ok(PDFConverter {
            tasks,
            worker_number,
        })
    }
    pub fn convert(&self) -> Result<()> {
        let tasks = self.tasks.clone();
        let task_numer_per_group = tasks.len() / self.worker_number + 1;
        let mut task_groups: Vec<Vec<(PathBuf, PathBuf)>> = vec![];
        let mut start = 0;
        while start < tasks.len() {
            if let Some(group) = tasks.get(start..start + task_numer_per_group) {
                task_groups.push(group.to_vec());
                start += task_numer_per_group;
            } else {
                task_groups.push(tasks.get(start..).unwrap().to_vec());
                break;
            }
        }
        let (s, r) = crossbeam_channel::unbounded::<Vec<(PathBuf, PathBuf)>>();
        let mut handles = vec![];
        for _ in 0..self.worker_number {
            // println!("worker {} is running", i);
            let rx = r.clone();
            let h = spawn(move || loop {
                if let Ok(task) = rx.recv() {
                    if let Err(_) = rtf2pdf::rtf2pdf(task) {
                        // println!("ERROR: worker {} is crash", i);
                        return;
                    };
                } else {
                    // println!("worker {} is quitting", i);
                    return;
                }
            });
            handles.push(h);
        }
        for task in task_groups {
            s.send(task).unwrap();
        }
        drop(s);
        for h in handles {
            h.join().unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod converter_test {
    use super::*;
    #[test]
    fn convert_test() {
        let dir = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\rtf_divided");
        let converter = PDFConverter::new(dir).unwrap();
        converter.convert().unwrap();
        assert!(true);
    }
}
