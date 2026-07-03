//! Port of svg/ViewBox.go: parses the SVG `viewBox` attribute
//! ("minX minY width height", numbers separated by an optional comma and
//! required whitespace).

use crate::number::{is_space, scan_number};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("input '{0}' doesn't match a viewBox (four numbers separated by ',? '+)")]
pub struct ViewBoxError(String);

fn take_number(s: &str) -> Result<(f64, &str), ViewBoxError> {
    let len = scan_number(s).filter(|&len| len > 0);
    let len = match len {
        Some(len) => len,
        None => return Err(ViewBoxError(s.to_string())),
    };
    let value = s[..len]
        .parse::<f64>()
        .map_err(|_| ViewBoxError(s.to_string()))?;
    Ok((value, &s[len..]))
}

fn take_separator(s: &str) -> Result<&str, ViewBoxError> {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && bytes[i] == b',' {
        i += 1;
    }
    let ws_start = i;
    while i < bytes.len() && is_space(bytes[i]) {
        i += 1;
    }
    if i == ws_start {
        return Err(ViewBoxError(s.to_string()));
    }
    Ok(&s[i..])
}

pub fn parse_view_box(input: &str) -> Result<ViewBox, ViewBoxError> {
    let (min_x, rest) = take_number(input)?;
    let rest = take_separator(rest)?;
    let (min_y, rest) = take_number(rest)?;
    let rest = take_separator(rest)?;
    let (width, rest) = take_number(rest)?;
    let rest = take_separator(rest)?;
    let (height, rest) = take_number(rest)?;
    if !rest.is_empty() {
        return Err(ViewBoxError(input.to_string()));
    }
    Ok(ViewBox {
        min_x,
        min_y,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_view_box() {
        assert!(parse_view_box("+").is_err());
        assert!(parse_view_box("-").is_err());
        assert!(parse_view_box("").is_err());
        assert_eq!(
            parse_view_box("0 0 25.4 25.4").unwrap(),
            ViewBox {
                min_x: 0.0,
                min_y: 0.0,
                width: 25.4,
                height: 25.4
            }
        );
        assert_eq!(
            parse_view_box("1 2 3 4").unwrap(),
            ViewBox {
                min_x: 1.0,
                min_y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        // Separator grammar is optional comma + *required* whitespace, so
        // "1,2" (comma with no following space) is invalid, but "1, 2" is.
        assert_eq!(
            parse_view_box("1, 2 3, 4").unwrap(),
            ViewBox {
                min_x: 1.0,
                min_y: 2.0,
                width: 3.0,
                height: 4.0
            }
        );
        assert!(parse_view_box("1,2 3,4").is_err());
    }
}
