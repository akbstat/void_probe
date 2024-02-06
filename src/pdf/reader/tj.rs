#[derive(Debug, PartialEq)]
pub enum Text {
    ASCII(String),
    UNICODE(String),
}

const OPEN_BRACKET: u8 = b'[';
const CLOSE_BRACKET: u8 = b']';
const OPEN_PARENTHESIS: u8 = b'(';
const CLOSE_PARENTHESIS: u8 = b')';
const GREATER: u8 = b'>';
const LESS: u8 = b'<';

pub fn handle_tj(source: &[u8]) -> Text {
    let mut s = String::new();
    let mut content_start = 0;
    let mut content_end = source.len() - 1;

    // if pdf convert by word, text content will be wrap in to bracket like [( )], so we need to extract the text content first
    while content_start.lt(&source.len()) && content_end.ge(&0) && content_end.gt(&content_start) {
        let head = source.get(content_start).unwrap();
        let tail = source.get(content_end).unwrap();
        if OPEN_BRACKET.eq(head) && CLOSE_BRACKET.eq(tail) {
            content_start += 1;
            break;
        }
        if OPEN_BRACKET.ne(head) {
            content_start += 1;
        }
        if CLOSE_BRACKET.ne(tail) {
            content_end -= 1;
        }
    }

    // content_start < content_end means perhaps pdf convert by word, else means convert by wps
    let source = if content_start < content_end {
        source.get(content_start..content_end).unwrap()
    } else {
        source
    };

    let mut content_indexes = vec![];
    let mut content_index = (0, 0);
    // a mark if content is present in unicode pattern
    let mut unicode_content = false;
    let mut content_parsing = false;
    source.iter().enumerate().for_each(|(i, c)| {
        if OPEN_PARENTHESIS.eq(c) && !content_parsing {
            unicode_content = false;
            content_index.0 = i + 1;
            content_parsing = true;
        }
        if CLOSE_PARENTHESIS.eq(c) && !unicode_content {
            content_index.1 = i;
            content_indexes.push(content_index);
            content_parsing = false;
        }
        if LESS.eq(c) && !content_parsing {
            unicode_content = true;
            content_index.0 = i + 1;
            content_parsing = true;
        }
        if GREATER.eq(c) && unicode_content {
            content_index.1 = i;
            content_indexes.push(content_index);
            content_parsing = false;
        }
    });
    content_indexes.iter().for_each(|(start, end)| {
        if let Some(data) = source.get(*start..*end) {
            s.push_str(String::from_utf8_lossy(data).to_string().as_str());
        }
    });
    if unicode_content {
        Text::UNICODE(s)
    } else {
        Text::ASCII(s)
    }
}

#[cfg(test)]
mod tj_test {
    use super::*;
    #[test]
    fn handle_tj_test() {
        let content = "[(p)-6(r)5(o)7(g)7(r)5(a)-3(m)]".as_bytes();
        let content = handle_tj(content);
        assert_eq!(content, Text::ASCII("program".into()));
        let content = "[( )]".as_bytes();
        let content = handle_tj(content);
        assert_eq!(content, Text::ASCII(" ".into()));
        let content = "[<22EB2BC71C151D4F02C42E4430A6>]".as_bytes();
        let content = handle_tj(content);
        assert_eq!(
            content,
            Text::UNICODE("22EB2BC71C151D4F02C42E4430A6".into())
        );
        let content = "[<1BE91E783546>11<0A2702D6>]".as_bytes();
        let content = handle_tj(content);
        assert_eq!(content, Text::UNICODE("1BE91E7835460A2702D6".into()));
        let content = r"[(\()] ".as_bytes();
        let content = handle_tj(content);
        assert_eq!(content, Text::ASCII(r"\(".into()));

        // content in wps
        let content = "<0026>Tj 139.188 -0 TD<0036>Tj 117.594 -0 TD<0035>".as_bytes();
        let content = handle_tj(content);
        assert_eq!(content, Text::UNICODE("002600360035".into()));
    }
}
