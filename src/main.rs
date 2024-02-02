use std::{env, fs, path::Path};
use void_probe::void_probe;
fn main() {
    let dir = env::args().nth(1).unwrap();
    let dir = Path::new(dir.as_str());
    let mut rtfs = vec![];
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".rtf") {
            rtfs.push(dir.join(name));
        }
    }
    void_probe(&rtfs).unwrap().iter().for_each(|r| {
        println!("{:?}", r);
    });
}
