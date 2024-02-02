# Void Probe
> A tool for detecting unexpected page breaking for tfls

# How to Use
```rust
#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;
    #[test]
    fn probe_test() {
        let dir = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output"); // will check all the rtfs in this directory
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
```
Will return a `Vec<Report>`, each Report contains following informations:
* pdf file path
* pages list that break happend
