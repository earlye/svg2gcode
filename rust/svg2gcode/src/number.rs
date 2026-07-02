//! Port of svg/Number.go: parses the numeric tokens used throughout SVG
//! attributes (lengths, path data, transform arguments).

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NumberError {
    #[error("input '{0}' is not a valid number")]
    NotANumber(String),
    #[error("input '{0}' doesn't start with whitespace and a number")]
    NoLeadingNumber(String),
    #[error("input '{0}' starts with a number but continues with characters that require additional digits")]
    Incomplete(String),
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// Scans a leading `[+-]?(digits(.digits)?|.digits)([eE][+-]?digits)?` token
/// from `s`, mirroring svg.NumberPattern. Returns the byte length consumed,
/// or `None` if `s` doesn't start with a valid number.
fn scan_number(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0;

    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let int_start = i;
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let mut has_mantissa = i > int_start;

    if i < n && bytes[i] == b'.' {
        let mut j = i + 1;
        let frac_start = j;
        while j < n && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > frac_start {
            i = j;
            has_mantissa = true;
        }
        // else: leave the '.' unconsumed, same as the Go regex not matching it.
    }

    if !has_mantissa {
        return None;
    }

    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < n && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_digits_start = j;
        while j < n && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_digits_start {
            i = j;
        }
        // else: leave the 'e'/'E' unconsumed.
    }

    Some(i)
}

/// Parses `input` as a number, requiring the entire string to match.
pub fn parse_number(input: &str) -> Result<f64, NumberError> {
    match scan_number(input) {
        Some(len) if len == input.len() && len > 0 => input
            .parse::<f64>()
            .map_err(|_| NumberError::NotANumber(input.to_string())),
        _ => Err(NumberError::NotANumber(input.to_string())),
    }
}

/// Parses `input` as a number, panicking on failure. Mirrors Go's
/// `MustParseNumber` for straight-port call sites; new code should prefer
/// `parse_number`.
pub fn must_parse_number(input: &str) -> f64 {
    parse_number(input).unwrap_or_else(|err| panic!("{err}"))
}

/// Parses `input` as a number, returning `default_result` on failure.
pub fn parse_number_default(input: &str, default_result: f64) -> f64 {
    parse_number(input).unwrap_or(default_result)
}

/// Consumes leading whitespace and a number from `input`, returning the
/// parsed value and the remaining (unconsumed) slice.
pub fn pop_number(input: &str) -> Result<(f64, &str), NumberError> {
    let bytes = input.as_bytes();
    let mut ws_end = 0;
    while ws_end < bytes.len() && is_space(bytes[ws_end]) {
        ws_end += 1;
    }
    let after_ws = &input[ws_end..];

    match scan_number(after_ws) {
        Some(len) if len > 0 => {
            let number_part = &after_ws[..len];
            let remaining = &after_ws[len..];
            if remaining.starts_with('.') || remaining.starts_with('e') || remaining.starts_with('E') {
                return Err(NumberError::Incomplete(input.to_string()));
            }
            let value = number_part
                .parse::<f64>()
                .map_err(|_| NumberError::NotANumber(input.to_string()))?;
            Ok((value, remaining))
        }
        _ => Err(NumberError::NoLeadingNumber(input.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let cases: &[(&str, f64, bool)] = &[
            ("+", 0.0, true),
            ("-", 0.0, true),
            ("", 0.0, true),
            ("32.", 0.0, true),
            ("1e", 0.0, true),
            ("+32", 32.0, false),
            ("-32", -32.0, false),
            ("32", 32.0, false),
            ("32.5", 32.5, false),
            (".5", 0.5, false),
            ("-.5", -0.5, false),
            ("1e2", 100.0, false),
        ];

        for &(input, expect_val, expect_err) in cases {
            let result = parse_number(input);
            if expect_err {
                assert!(result.is_err(), "expected error for input {input:?}");
            } else {
                assert_eq!(result, Ok(expect_val), "input {input:?}");
            }
        }
    }

    #[test]
    fn test_pop_number() {
        let cases: &[(&str, f64, &str, bool)] = &[
            ("+", 0.0, "", true),
            ("-", 0.0, "", true),
            ("", 0.0, "", true),
            ("32.", 0.0, "", true),
            ("32).", 32.0, ").", false),
            ("1e", 0.0, "", true),
            ("+32", 32.0, "", false),
            ("-32", -32.0, "", false),
            ("32", 32.0, "", false),
            ("32.5", 32.5, "", false),
            (".5", 0.5, "", false),
            ("-.5", -0.5, "", false),
            ("1e2", 100.0, "", false),
            ("32 64", 32.0, " 64", false),
            (" 64", 64.0, "", false),
            ("32,64", 32.0, ",64", false),
            ("64", 64.0, "", false),
        ];

        for &(input, expect_val, expect_remain, expect_err) in cases {
            let result = pop_number(input);
            if expect_err {
                assert!(result.is_err(), "expected error for input {input:?}");
            } else {
                let (val, remaining) = result.unwrap_or_else(|e| panic!("input {input:?}: {e}"));
                assert_eq!(val, expect_val, "input {input:?}");
                assert_eq!(remaining, expect_remain, "input {input:?}");
            }
        }
    }
}
