use std::{cell::RefCell, collections::HashMap, io::Read, ops::Sub, path::Path};

use flate2::read::ZlibDecoder;
use lopdf::{Dictionary, Document};

mod mapper;
mod tj;
mod tm;

const RESOURCES: &[u8] = "Resources".as_bytes();
const FONT: &[u8] = "Font".as_bytes();
const TO_UNICODE: &[u8] = "ToUnicode".as_bytes();
const SPACE: u8 = b' ';
const CONTENTS: &[u8] = "Contents".as_bytes();
const TM: &[u8] = "Tm\r".as_bytes();
const TF: &[u8] = "Tf\r".as_bytes();
const TJ_WORD: &[u8] = "TJ\r".as_bytes();
const TJ_WPS: &[u8] = "Tj\n".as_bytes();
const NEWLINE: u8 = b'\n';
const RETURN: u8 = b'\r';

pub struct PDFReader {
    doc: Document,
    decode_map: RefCell<HashMap<String, HashMap<String, String>>>,
    pages: RefCell<Vec<Vec<String>>>,
}

impl PDFReader {
    pub fn new(file: &Path) -> anyhow::Result<PDFReader> {
        let doc = Document::load(file)?;
        let decode_map = RefCell::new(HashMap::new());
        let pages = RefCell::new(vec![]);
        let reader = PDFReader {
            doc,
            decode_map,
            pages,
        };
        for page_obj_id in reader.doc.page_iter() {
            let page_obj = reader.doc.get_object(page_obj_id)?;
            for dict in page_obj.as_dict().iter() {
                let dict = dict.as_hashmap();
                if let Some(resource) = dict.get(RESOURCES) {
                    let resource = resource.as_dict()?;
                    let font = resource.get(FONT)?.as_dict()?;
                    reader.insert_decode_map(font)?;
                }
                if let Some(content_id) = dict.get(CONTENTS) {
                    let content_stream = &reader
                        .doc
                        .get_object(content_id.as_reference()?)?
                        .as_stream()?
                        .content;
                    let content_stream = decode(&content_stream)?;
                    reader.build_page_content(&content_stream)?;
                }
            }
        }
        Ok(reader)
    }

    fn insert_decode_map(&self, font: &Dictionary) -> anyhow::Result<()> {
        for (font_name, object) in font.iter() {
            let font_name = String::from_utf8(font_name.to_owned())?;
            if self.decode_map.borrow().contains_key(&font_name) {
                continue;
            }
            let font = self.doc.get_object(object.as_reference()?)?.as_dict()?;
            if let Ok(unicode) = font.get(TO_UNICODE) {
                let stream_content = &self
                    .doc
                    .get_object(unicode.as_reference()?)?
                    .as_stream()?
                    .content;
                let stream_content = decode(&stream_content)?;
                let code_map = mapper::build_unicode_map(&stream_content);
                if let Some(code_map) = code_map {
                    self.decode_map.borrow_mut().insert(font_name, code_map);
                }
            }
        }
        Ok(())
    }

    fn build_page_content(&self, data: &[u8]) -> anyhow::Result<()> {
        let page = self.build_content(data)?;
        self.pages.borrow_mut().push(page);
        Ok(())
    }

    pub fn build_content(&self, source: &[u8]) -> anyhow::Result<Vec<String>> {
        let mut rows = vec![];
        let mut row = String::new();
        let mut row_start = 0;
        let mut row_number = 0f64;
        let mut font_type = String::new();
        for (i, c) in source.iter().enumerate() {
            if NEWLINE.eq(c) || RETURN.eq(c) {
                if i > 3 {
                    if let Some(mark) = source.get(i - TJ_WORD.len() + 1..i + 1) {
                        if TJ_WORD.eq(mark) || TJ_WPS.eq(mark) {
                            // handle contents generated by office word
                            let content = source.get(row_start..i - TJ_WORD.len() + 1);
                            let text: tj::Text = tj::handle_tj(content.unwrap());
                            match text {
                                tj::Text::ASCII(text) => {
                                    row.push_str(&text);
                                }
                                tj::Text::UNICODE(text) => {
                                    if let Some(decode_map) =
                                        self.decode_map.borrow().get(&font_type)
                                    {
                                        let mut i = 0;
                                        while let Some(word) = text.get(i..i + 4) {
                                            if let Some(word) = decode_map.get(word) {
                                                row.push_str(&unicode_to_u8(word));
                                            }
                                            i = i + 4;
                                        }
                                    }
                                }
                            }
                        } else if TM.eq(mark) {
                            // handle posistion information
                            let content = source.get(row_start..i - TJ_WORD.len() + 1);
                            let current_row = tm::handle_tm(content.unwrap())[5];
                            let sub = row_number.sub(current_row);
                            if sub > 1f64 || sub < -1f64 {
                                rows.push(row.clone());
                                row.clear();
                                row_number = current_row;
                            }
                        } else if TF.eq(mark) {
                            let content = source.get(row_start..i - TF.len() + 1);
                            let mut i = 0;
                            while let Some(c) = content.unwrap().get(i) {
                                if SPACE.eq(c) {
                                    break;
                                }
                                i += 1;
                            }
                            font_type =
                                String::from_utf8_lossy(content.unwrap().get(1..i).unwrap())
                                    .to_string();
                        }
                    }
                }
                row_start = i + 1;
            }
        }
        Ok(rows)
    }

    pub fn content(&self) -> Vec<Vec<String>> {
        self.pages.borrow().to_owned()
    }
}

fn decode(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut buf = vec![];
    let mut e = ZlibDecoder::new(data);
    e.read_to_end(&mut buf)?;
    Ok(buf)
}

fn unicode_to_u8(source: &String) -> String {
    let hex = u16::from_str_radix(source, 16).unwrap();
    let r = char::decode_utf16(vec![hex])
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    r.iter().collect::<String>()
}

#[cfg(test)]
mod pdf_reader_test {
    use super::*;

    #[test]
    fn read_test() {
        let p = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\.temp\wps-cn.pdf");
        let r = PDFReader::new(p).unwrap();
        let content = r.content();
        assert_eq!(1, content.len());
        let p = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\.temp\word-cn.pdf");
        let r = PDFReader::new(p).unwrap();
        let content = r.content();
        assert_eq!(17, content.len());
    }
}
