use std::{cell::RefCell, ops::Deref};

#[derive(Debug)]
pub struct Report {
    file: String,
    void: RefCell<Vec<usize>>,
}

impl Report {
    pub fn new(file: &str) -> Report {
        Report {
            file: file.into(),
            void: RefCell::new(vec![]),
        }
    }
    pub fn file(&self) -> String {
        self.file.clone()
    }
    pub fn append_void(&self, page: usize) -> &Self {
        self.void.borrow_mut().push(page);
        self
    }
    pub fn void(&self) -> Vec<usize> {
        self.void.borrow().deref().to_vec()
    }
}

#[cfg(test)]
mod report_test {
    use super::*;

    #[test]
    fn test_report() {
        let r = Report::new("test.rtf");
        r.append_void(1).append_void(2).append_void(3);
        assert_eq!(r.void(), vec![1, 2, 3]);
        assert_eq!(r.file(), String::from("test.rtf"));
    }
}
