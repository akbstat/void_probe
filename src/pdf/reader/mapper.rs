use std::collections::HashMap;

const BEGINBFCHAR: &[u8] = "beginbfchar".as_bytes();
const ENDBFCHAR: &[u8] = "endbfchar".as_bytes();
const BEGINBFRANGE: &[u8] = "beginbfrange".as_bytes();
const ENDBFRANGE: &[u8] = "endbfrange".as_bytes();
const MAP_ITEM_LEN: usize = "<0000> <0000>".len();
const RANGE_MAP_ITEM_LEN: usize = "<0000> <0000> <0000>".len();
const GREATER: u8 = b'>';
const LESS: u8 = b'<';

pub fn build_unicode_map(source: &[u8]) -> Option<HashMap<String, String>> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut range_indexes = vec![];
    let mut char_indexes = vec![];
    let mut index = (0, 0);

    source.iter().enumerate().for_each(|(i, _)| {
        if i > BEGINBFRANGE.len() {
            if BEGINBFRANGE.eq(source.get(i - BEGINBFRANGE.len() + 1..i + 1).unwrap()) {
                index.0 = i + 2;
            }
            if ENDBFRANGE.eq(source.get(i - ENDBFRANGE.len() + 1..i + 1).unwrap()) {
                index.1 = i;
                range_indexes.push(index)
            }
            if BEGINBFCHAR.eq(source.get(i - BEGINBFCHAR.len() + 1..i + 1).unwrap()) {
                index.0 = i + 2;
            }
            if ENDBFCHAR.eq(source.get(i - ENDBFCHAR.len() + 1..i + 1).unwrap()) {
                index.1 = i;
                char_indexes.push(index)
            }
        }
    });

    range_indexes.iter().for_each(|(start, end)| {
        let pairs = source.get(*start..*end).unwrap();
        let mut i = 0;
        while let Some(item) = pairs.get(i..i + RANGE_MAP_ITEM_LEN) {
            unicode_map_pair(item).iter().for_each(|(key, value)| {
                map.insert(key.to_owned(), value.to_owned());
            });
            i += RANGE_MAP_ITEM_LEN + 1;
        }
    });
    char_indexes.iter().for_each(|(start, end)| {
        let pairs = source.get(*start..*end).unwrap();
        let mut i = 0;
        while let Some(item) = pairs.get(i..i + MAP_ITEM_LEN) {
            unicode_map_pair(item).iter().for_each(|(key, value)| {
                map.insert(key.to_owned(), value.to_owned());
            });
            i += MAP_ITEM_LEN + 1;
        }
    });

    Some(map)
}

fn unicode_map_pair(source: &[u8]) -> Vec<(String, String)> {
    let mut pairs = vec![];
    match source.len() {
        13 => {
            if source.first().unwrap().eq(&LESS) && source.last().unwrap().eq(&GREATER) {
                pairs.push((
                    String::from_utf8_lossy(source.get(1..5).unwrap()).to_string(),
                    String::from_utf8_lossy(source.get(8..12).unwrap()).to_string(),
                ));
            }
        }
        20 => {
            if source.first().unwrap().eq(&LESS) && source.last().unwrap().eq(&GREATER) {
                let start = usize::from_str_radix(
                    String::from_utf8_lossy(source.get(1..5).unwrap())
                        .to_string()
                        .as_str(),
                    16,
                );
                let end = usize::from_str_radix(
                    String::from_utf8_lossy(source.get(8..12).unwrap())
                        .to_string()
                        .as_str(),
                    16,
                );
                if start.is_ok() && end.is_ok() {
                    for i in start.unwrap()..=end.unwrap() {
                        pairs.push((
                            format!("{:04X}", i),
                            String::from_utf8_lossy(source.get(15..19).unwrap()).to_string(),
                        ))
                    }
                }
            }
        }
        _ => {}
    }

    pairs
}

#[cfg(test)]
mod mapper_test {
    use std::{fs, path::Path};

    use super::*;
    #[test]
    fn build_unicode_map_test() {
        let ft = Path::new(r"D:\misc\utils\rtfs\data\FT9.txt");
        let data = fs::read(ft).unwrap();
        let map = build_unicode_map(&data).unwrap();
        assert_eq!(127, map.len());
        let ft = Path::new(r"D:\misc\utils\rtfs\data\ftx.txt");
        let data = fs::read(ft).unwrap();
        let map = build_unicode_map(&data).unwrap();
        assert_eq!(75, map.len());
    }

    #[test]
    fn unicode_map_pair_test() {
        let source = r"<02C8> <FF0C>".as_bytes();
        let result = unicode_map_pair(source);
        assert_eq!(result.len(), 1);

        let source = r"<02C4> <02C5> <FF08>".as_bytes();
        let result = unicode_map_pair(source);
        assert_eq!(result.len(), 2);
    }
}
