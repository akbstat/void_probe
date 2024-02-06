const SPACE: u8 = b' ';
/// handle the position information define in tm row
///
/// ```rust
///     fn handle_tm_test() {
///         let content = "1 0 0 1 435.29 473.14 ".as_bytes();
///         let pos = handle_tm(content);
///         assert_eq!(pos, [1f64, 0f64, 0f64, 1f64, 435.29f64, 473.14f64]);
///     }
/// ```
pub fn handle_tm(source: &[u8]) -> [f64; 6] {
    let mut position: [f64; 6] = [0f64; 6];
    let mut pos = 0;
    let mut start = 0;
    for (i, c) in source.iter().enumerate() {
        if SPACE.eq(c) {
            if let Some(n) = source.get(start..i) {
                if let Ok(n) = String::from_utf8_lossy(n).to_string().parse::<f64>() {
                    position[pos] = n;
                }
                pos += 1;
            }
            start = i + 1;
        }
    }
    position
}

#[cfg(test)]
mod tm_test {
    use super::*;
    #[test]
    fn handle_tm_test() {
        let content = "1 0 0 1 435.29 473.14 ".as_bytes();
        let pos = handle_tm(content);
        assert_eq!(pos, [1f64, 0f64, 0f64, 1f64, 435.29f64, 473.14f64]);
    }
}
