//! Port of svg/Length.go: converts an SVG length string to millimeters.

use crate::number::{pop_number, NumberError};

/// Converts an SVG length string to millimeters. Unitless values and "mm"
/// are returned as-is; other units (cm, in, pt, pc, px) are converted.
pub fn parse_length_mm(s: &str) -> Result<f64, NumberError> {
    let (value, remaining) = pop_number(s)?;
    let mm = match remaining.trim() {
        "cm" => value * 10.0,
        "in" => value * 25.4,
        "pt" => value * 25.4 / 72.0,
        "pc" => value * 25.4 / 6.0,
        "px" => value * 25.4 / 96.0,
        _ => value, // "mm" or unitless -- treat as mm
    };
    Ok(mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_length_mm() {
        let cases: &[(&str, f64)] = &[
            ("10", 10.0),
            ("10mm", 10.0),
            ("1cm", 10.0),
            ("1in", 25.4),
            ("72pt", 25.4),
            ("6pc", 25.4),
            ("96px", 25.4),
        ];
        for &(input, expect) in cases {
            let got = parse_length_mm(input).unwrap_or_else(|e| panic!("input {input:?}: {e}"));
            assert!(
                (got - expect).abs() < 1e-9,
                "input {input:?}: got {got}, want {expect}"
            );
        }
    }

    #[test]
    fn test_parse_length_mm_invalid() {
        assert!(parse_length_mm("not-a-number").is_err());
    }
}
