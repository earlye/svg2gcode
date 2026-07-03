//! Port of svg/ParseSvgDocument.go + svg/XmlElement.go, using `roxmltree`
//! instead of Go's streaming `encoding/xml` decoder.
//!
//! Scope trim vs. Go (documented, not silent): Go's `Document` also tracks
//! `FrontMatter`/`BackMatter` (comments/PIs before and after `<svg>`), used
//! only for `-v=SILLY` debug YAML dumps in the CLI, never read by the
//! carve/gcode pipeline itself. That's not ported here. Also, Go decodes
//! character sets declared in the XML prolog via `golang.org/x/net/html/
//! charset`; this port only accepts UTF-8 input (every fixture in
//! examples/*.svg is UTF-8 or unspecified-defaulting-to-UTF-8).

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("input was empty")]
    Empty,
    #[error("failed to parse XML: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("unexpected top-level element '{0}'")]
    UnexpectedRootElement(String),
    #[error("unexpected namespace '{0}'")]
    UnexpectedNamespace(String),
}

pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// Port of util.NameToKey: qualifies a namespaced XML name the same way Go's
/// `encoding/xml` does, e.g. `("http://...sodipodi...", "namedview")` ->
/// `"http://...sodipodi...:namedview"`, or `(None, "desc")` -> `"desc"`.
pub fn name_to_key(namespace: Option<&str>, local: &str) -> String {
    match namespace {
        None | Some("") => local.to_string(),
        Some(ns) => format!("{ns}:{local}"),
    }
}

/// Parses `input` as an SVG document, validating that the root element is
/// `<svg>` in either no namespace or the SVG namespace -- mirroring the
/// checks in Go's `DecodeSvgDocument`.
pub fn parse_svg_document(input: &str) -> Result<roxmltree::Document<'_>, DocumentError> {
    if input.trim().is_empty() {
        return Err(DocumentError::Empty);
    }
    let doc = roxmltree::Document::parse(input)?;
    let root = doc.root_element();
    let tag = root.tag_name();
    if tag.name() != "svg" {
        return Err(DocumentError::UnexpectedRootElement(tag.name().to_string()));
    }
    match tag.namespace() {
        None => {}
        Some(ns) if ns == SVG_NAMESPACE => {}
        Some(ns) => return Err(DocumentError::UnexpectedNamespace(ns.to_string())),
    }
    Ok(doc)
}

pub fn width<'a>(root: &roxmltree::Node<'a, 'a>) -> Option<&'a str> {
    root.attribute("width")
}

pub fn height<'a>(root: &roxmltree::Node<'a, 'a>) -> Option<&'a str> {
    root.attribute("height")
}

pub fn view_box<'a>(root: &roxmltree::Node<'a, 'a>) -> Option<&'a str> {
    root.attribute("viewBox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_document() {
        let cases: &[(&str, bool)] = &[
            ("</svg>", true),
            ("<svg foo='bar'></svg>", false),
            (
                "<?xml version='1.0' encoding='UTF-8'?>\n<!-- comment --><svg foo='bar'></svg>\n\n<?nope?><!-- comment -->",
                false,
            ),
            ("<svg><g>\n</g></svg>", false),
            ("<svg></bar></svg>", true),
            ("<foo:svg></foo:svg>", true),
            ("<foo></foo>", true),
            ("", true),
        ];

        for &(input, expect_err) in cases {
            let result = parse_svg_document(input);
            assert_eq!(result.is_err(), expect_err, "input {input:?}: {result:?}");
        }
    }

    #[test]
    fn test_width_height_view_box() {
        let text = "<svg width=\"12cm\" height=\"5.25cm\" viewBox=\"0 0 1200 525\"></svg>";
        let doc = parse_svg_document(text).unwrap();
        let root = doc.root_element();
        assert_eq!(width(&root), Some("12cm"));
        assert_eq!(height(&root), Some("5.25cm"));
        assert_eq!(view_box(&root), Some("0 0 1200 525"));
    }

    #[test]
    fn test_name_to_key() {
        assert_eq!(name_to_key(None, "desc"), "desc");
        assert_eq!(
            name_to_key(Some("http://www.w3.org/2000/svg"), "desc"),
            "http://www.w3.org/2000/svg:desc"
        );
    }
}
