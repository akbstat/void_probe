use std::{cell::RefCell, collections::HashMap, io::Read, ops::Sub, path::Path};

use flate2::read::ZlibDecoder;
use lopdf::{Dictionary, Document};

const RESOURCES: &[u8] = "Resources".as_bytes();
const FONT: &[u8] = "Font".as_bytes();
const TO_UNICODE: &[u8] = "ToUnicode".as_bytes();
const BEGINBFCHAR: &[u8] = "beginbfchar".as_bytes();
const SPACE: u8 = b' ';
const CONTENTS: &[u8] = "Contents".as_bytes();
const MAP_ITEM_LEN: usize = "<0000> <0000>".len();
const LEFT_BASKET: u8 = b'<';
const RIGHT_BASKET: u8 = b'>';
const BT: &[u8] = "BT\r".as_bytes();
const TM: &[u8] = "Tm\r".as_bytes();
const FT: &[u8] = r"/FT".as_bytes();
const ET: &[u8] = "ET\r".as_bytes();

pub struct PDFReader {
    doc: Document,
    decode_map: RefCell<HashMap<String, HashMap<String, String>>>,
    pages: RefCell<Vec<Vec<Vec<String>>>>,
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
        for obj_id in reader.doc.page_iter() {
            let obj = reader.doc.get_object(obj_id)?;
            for dict in obj.as_dict().iter() {
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
            let unicode = font.get(TO_UNICODE)?;
            let stream_content = &self
                .doc
                .get_object(unicode.as_reference()?)?
                .as_stream()?
                .content;
            let stream_content = decode(&stream_content)?;
            let code_map = build_unicode_map(&stream_content);
            if let Some(code_map) = code_map {
                self.decode_map.borrow_mut().insert(font_name, code_map);
            }
        }
        Ok(())
    }
    fn build_page_content(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut page = vec![];
        let mut row = vec![];
        let mut string_slice = vec![];
        let mut in_block = false;

        let mut height = 0f64;
        let decode_map = self.decode_map.borrow();
        let mut current_map_name = String::new();
        for (i, c) in data.iter().enumerate() {
            if i < 3 {
                continue;
            }
            if let Some(sign) = data.get(i - 3..i) {
                if sign.eq(BT) {
                    string_slice.clear();
                    in_block = true;
                    continue;
                }
                if sign.eq(TM) {
                    let end = i - 4;
                    let mut start = end - 1;
                    while let Some(char) = data.get(start) {
                        if char.eq(&SPACE) {
                            break;
                        }
                        start -= 1;
                    }
                    start += 1;
                    if let Some(h) = data.get(start..end) {
                        let h = String::from_utf8(h.to_vec())?.trim().parse::<f64>()?;
                        if h.sub(height) > 1f64 || h.sub(height) < -1f64 {
                            page.push(row.clone());
                            row.clear();
                        }
                        height = h;
                    }
                }
                if sign.eq(FT) {
                    current_map_name = format!(
                        "FT{}",
                        String::from_utf8(data[i..i + 2].to_vec())?
                            .trim()
                            .parse::<usize>()?
                    );
                }
                if in_block && c.eq(&RIGHT_BASKET) {
                    if let Some(code) = data.get(i - 4..i) {
                        let s = String::from_utf8(code.to_vec())?;
                        if let Some(unicode) = decode_map.get(&current_map_name) {
                            if let Some(data) = unicode.get(&s) {
                                string_slice.push(data.clone());
                            }
                        }
                    }
                }
                if sign.eq(ET) {
                    let s = unicode_to_u8(&string_slice);
                    row.push(s);
                    string_slice.clear();
                    in_block = false;
                }
            }
        }
        self.pages.borrow_mut().push(page);
        Ok(())
    }
    pub fn content(&self) -> Vec<Vec<Vec<String>>> {
        self.pages.borrow().to_owned()
    }
}

fn build_unicode_map(source: &[u8]) -> Option<HashMap<String, String>> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut i = 0;
    while i < BEGINBFCHAR.len() {
        i += 1;
    }
    // find out char map start index
    while let Some(sign) = source.get(i - BEGINBFCHAR.len()..i) {
        if sign.eq(BEGINBFCHAR) || i == source.len() - 1 {
            break;
        }
        i += 1;
    }
    // find out decode pair like "<01C3> <3001>"
    while let Some(pair) = source.get(i - MAP_ITEM_LEN..i) {
        let first = pair.first();
        let last = pair.last();
        let middle = pair.get(6);
        if first.is_some() && last.is_some() && middle.is_some() {
            let first = first.unwrap();
            let last = last.unwrap();
            let middle = middle.unwrap();
            if first.eq(&LEFT_BASKET) && last.eq(&RIGHT_BASKET) && middle.eq(&SPACE) {
                let key = String::from_utf8(pair.get(1..5).unwrap().to_vec());
                let value = String::from_utf8(pair.get(8..12).unwrap().to_vec());
                if key.is_ok() && value.is_ok() {
                    map.insert(key.unwrap(), value.unwrap());
                }
            }
        }
        i += 1;
    }
    Some(map)
}

fn decode(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut buf = vec![];
    let mut e = ZlibDecoder::new(data);
    e.read_to_end(&mut buf)?;
    Ok(buf)
}

fn unicode_to_u8(source: &[String]) -> String {
    let i = source
        .iter()
        .map(|s| u16::from_str_radix(s, 16).unwrap())
        .collect::<Vec<u16>>();

    let r = char::decode_utf16(i)
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    r.iter().collect::<String>()
}

#[cfg(test)]
mod pdf_reader_test {
    use std::fs;

    use super::*;
    #[test]
    fn read_test() {
        let p = Path::new(
            r"D:\Studies\ak112\303\stats\CSR\product\output\.temp\l-16-02-04-08-01-antu-ex-ss.pdf",
        );
        let r = PDFReader::new(p).unwrap();
        let c = r.content();
        assert_eq!(1, c.len());
    }

    #[test]
    fn build_unicode_map_test() {
        let ft = Path::new(r"D:\misc\utils\rtfs\data\FT9.txt");
        let data = fs::read(ft).unwrap();
        let map = build_unicode_map(&data).unwrap();
        assert_eq!(127, map.len());
    }
}
