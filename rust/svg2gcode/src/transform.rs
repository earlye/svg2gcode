//! Port of svg/Transform.go: parses SVG `transform="..."` attribute lists
//! and applies them to points.
//!
//! Per issue-019f1a9b-fa38-7267-baae-57c1c192015a-rust-port-plan.md, this is
//! hand-rolled (no `nalgebra`). Go's `Transform` isn't a matrix struct — it's
//! a named op + parameter list evaluated by direct point-transform functions
//! (matrix/translate/scale/rotate/skewX/skewY), so this is a faithful,
//! minimal port of that shape rather than an invented Affine2 matrix type.

use crate::number::scan_number;

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub name: String,
    pub parameters: Vec<f64>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransformError {
    #[error("failed to consume all of transform list: '{0}'")]
    IncompleteList(String),
    #[error("failed to consume all of transform parameters list: '{0}'")]
    IncompleteParameters(String),
}

fn matrix(x: f64, y: f64, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> (f64, f64) {
    (a * x + c * y + e, b * x + d * y + f)
}

fn translate(x: f64, y: f64, dx: f64, dy: f64) -> (f64, f64) {
    (x + dx, y + dy)
}

fn scale(x: f64, y: f64, sx: f64, sy: f64) -> (f64, f64) {
    (x * sx, y * sy)
}

fn radians(deg: f64) -> f64 {
    deg * (std::f64::consts::PI / 180.0)
}

fn rotate(x: f64, y: f64, cx: f64, cy: f64, deg: f64) -> (f64, f64) {
    let a = radians(deg);
    matrix(
        x,
        y,
        a.cos(),
        a.sin(),
        -a.sin(),
        a.cos(),
        cx * (1.0 - a.cos()) + cy * a.sin(),
        cy * (1.0 - a.cos()) - cx * a.sin(),
    )
}

// NOTE: this mirrors Go's existing skewX/skewY formulas, which use cos(a)
// rather than the SVG-spec tan(a). That's locked in by Transform_test.go's
// TestTransformApply (skewX(30) on (4,4) expects ~7.46 = 4 + cos(30°)*4).
// Out of scope to "fix" per the straight-port decision in the plan doc.
fn skew_x(x: f64, y: f64, deg: f64) -> (f64, f64) {
    let a = radians(deg);
    (x + a.cos() * y, y)
}

fn skew_y(x: f64, y: f64, deg: f64) -> (f64, f64) {
    let a = radians(deg);
    (x, y + a.cos() * x)
}

impl Transform {
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let p = &self.parameters;
        match self.name.as_str() {
            "matrix" => {
                if p.len() < 6 {
                    return (x, y);
                }
                matrix(x, y, p[0], p[1], p[2], p[3], p[4], p[5])
            }
            "translate" => {
                if p.is_empty() {
                    return (x, y);
                }
                let dy = p.get(1).copied().unwrap_or(0.0);
                translate(x, y, p[0], dy)
            }
            "scale" => {
                if p.is_empty() {
                    return (x, y);
                }
                let sy = p.get(1).copied().unwrap_or(p[0]);
                scale(x, y, p[0], sy)
            }
            "rotate" => {
                if p.is_empty() {
                    return (x, y);
                }
                let (cx, cy) = if p.len() >= 3 { (p[1], p[2]) } else { (0.0, 0.0) };
                rotate(x, y, cx, cy, p[0])
            }
            "skewX" => {
                if p.is_empty() {
                    return (x, y);
                }
                skew_x(x, y, p[0])
            }
            "skewY" => {
                if p.is_empty() {
                    return (x, y);
                }
                skew_y(x, y, p[0])
            }
            _ => (x, y), // "" (identity) and unsupported names both pass through.
        }
    }
}

pub fn apply_transform_list(x: f64, y: f64, transforms: &[Transform]) -> (f64, f64) {
    transforms.iter().fold((x, y), |(tx, ty), t| t.apply(tx, ty))
}

fn remove_whitespace_leading_comma(s: &str) -> &str {
    let s = s.trim();
    let s = s.strip_prefix(',').unwrap_or(s);
    s.trim()
}

/// Parses a `,`-or-whitespace separated numeric parameter list, e.g. the
/// inside of `translate(15, 16)`.
fn parse_transform_parameters(input: &str) -> Result<Vec<f64>, TransformError> {
    let mut remaining = remove_whitespace_leading_comma(input);
    let mut result = Vec::new();
    while !remaining.is_empty() {
        match scan_number(remaining).filter(|&len| len > 0) {
            Some(len) => {
                // scan_number already validated this slice; parse() cannot fail here.
                result.push(remaining[..len].parse::<f64>().unwrap());
                remaining = remove_whitespace_leading_comma(&remaining[len..]);
            }
            None => {
                return Err(TransformError::IncompleteParameters(input.to_string()));
            }
        }
    }
    Ok(result)
}

/// Finds a leading `name(params)` call at the start of `s` (no nested
/// parens). Returns (name, params, bytes consumed) on success.
fn find_transform_call(s: &str) -> Option<(&str, &str, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return None;
    }
    i += 1;
    while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
        i += 1;
    }
    let name_end = i;
    if i >= bytes.len() || bytes[i] != b'(' {
        return None;
    }
    let params_start = i + 1;
    let close_offset = s[params_start..].find(')')?;
    let params_end = params_start + close_offset;
    Some((&s[..name_end], &s[params_start..params_end], params_end + 1))
}

/// Parses an SVG `transform` attribute value into a list of `Transform`s
/// ordered so that `apply_transform_list` folding left-to-right reproduces
/// correct SVG semantics (the rightmost transform in the source string is
/// applied to the point first).
///
/// This anchors each call at the start of the (trimmed) remaining input,
/// rather than Go's unanchored regex search that silently skips
/// non-matching characters wherever they occur. Every case exercised by
/// Transform_test.go behaves identically either way; this port simply fails
/// loudly instead of silently ignoring stray text between transforms.
pub fn parse_transform_list(input: &str) -> Result<Vec<Transform>, TransformError> {
    let mut result = Vec::new();
    let mut remaining = remove_whitespace_leading_comma(input);
    while !remaining.is_empty() {
        match find_transform_call(remaining) {
            Some((name, params_str, consumed)) => {
                let parameters = parse_transform_parameters(params_str)?;
                result.push(Transform { name: name.to_string(), parameters });
                remaining = remove_whitespace_leading_comma(&remaining[consumed..]);
            }
            None => {
                return Err(TransformError::IncompleteList(remaining.to_string()));
            }
        }
    }
    result.reverse();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transform_list() {
        assert_eq!(
            parse_transform_list("scale(0.25,0.26), translate(15,16)").unwrap(),
            vec![
                Transform { name: "translate".into(), parameters: vec![15.0, 16.0] },
                Transform { name: "scale".into(), parameters: vec![0.25, 0.26] },
            ]
        );
        assert!(parse_transform_list("scale(0.25,0.26), translate(15,16), --").is_err());
        assert!(parse_transform_list("scale(0.25,---)").is_err());
        assert_eq!(parse_transform_list("").unwrap(), vec![]);
    }

    #[test]
    fn test_transform_apply() {
        let cases: &[(&str, &[f64], f64, f64, f64, f64, f64)] = &[
            ("scale", &[2.0, 3.0], 4.0, 5.0, 8.0, 15.0, 0.0),
            ("scale", &[2.0], 4.0, 5.0, 8.0, 10.0, 0.0),
            ("scale", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("translate", &[2.0, 3.0], 4.0, 5.0, 6.0, 8.0, 0.0),
            ("translate", &[2.0], 4.0, 5.0, 6.0, 5.0, 0.0),
            ("translate", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("rotate", &[90.0], 4.0, 0.0, 0.0, 4.0, 1e-10),
            ("rotate", &[90.0, 2.0, 0.0], 4.0, 0.0, 2.0, 2.0, 1e-9),
            ("skewX", &[30.0], 4.0, 4.0, 7.46, 4.0, 0.1),
            ("skewY", &[30.0], 4.0, 0.0, 4.0, 3.46, 0.1),
            ("matrix", &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("matrix", &[2.0, 0.0, 0.0, 3.0, 0.0, 0.0], 4.0, 5.0, 8.0, 15.0, 0.0),
            ("matrix", &[1.0, 0.0, 0.0, 1.0, 1.0, 2.0], 4.0, 5.0, 5.0, 7.0, 0.0),
            ("matrix", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("rotate", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("skewX", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("skewY", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
            ("unknown", &[], 4.0, 5.0, 4.0, 5.0, 0.0),
        ];

        for &(name, params, x, y, expect_x, expect_y, delta) in cases {
            let t = Transform { name: name.into(), parameters: params.to_vec() };
            let (rx, ry) = t.apply(x, y);
            assert!(
                (rx - expect_x).abs() <= delta,
                "{name}({params:?}) x: got {rx}, want {expect_x}"
            );
            assert!(
                (ry - expect_y).abs() <= delta,
                "{name}({params:?}) y: got {ry}, want {expect_y}"
            );
        }
    }
}
