//! Port of svgx/SvgxDocument.go + svgx/LoadDocument.go.

use crate::document::{name_to_key, view_box as doc_view_box, width as doc_width, DocumentError};
use crate::gcode_desc::{DepthResolver, GCodeDesc};
use crate::gcode_writer::GCodeWriter;
use crate::length::parse_length_mm;
use crate::number::{parse_number, NumberError};
use crate::path_cursor::CarveCtx;
use crate::svgx_element::{CarveError, SvgxElement};
use crate::transform::Transform;
use crate::view_box::parse_view_box;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("origin-marker must be set on a <circle> element, found <{0}>")]
    OriginMarkerNotCircle(String),
    #[error("origin-marker <circle> is missing a '{0}' attribute")]
    OriginMarkerMissingAttribute(&'static str),
    #[error("origin-marker <circle> has an invalid '{attribute}' attribute: {source}")]
    OriginMarkerInvalidAttribute {
        attribute: &'static str,
        source: NumberError,
    },
}

/// Reads the YAML content of an element's direct `<desc>` child, if any.
/// Simplification vs. Go: concatenates all of the `<desc>`'s direct text
/// children into one string and parses once, rather than unmarshaling each
/// CharData chunk separately and merging the results -- behaviorally
/// identical for the overwhelming common case (a `<desc>` with one text
/// node), and Go's multi-chunk-merge behavior only differs for the rare
/// case of a `<desc>` split across multiple text nodes (e.g. by embedded
/// entities), which no example or test in this repo exercises.
fn own_gcode_desc(node: roxmltree::Node) -> Option<GCodeDesc> {
    let desc = node.children().find(|c| {
        c.is_element()
            && name_to_key(c.tag_name().namespace(), c.tag_name().name())
                == "http://www.w3.org/2000/svg:desc"
    })?;
    let text: String = desc.children().filter_map(|c| c.text()).collect();
    GCodeDesc::parse(&text)
}

/// Builds the SvgxElement tree, tracking the most recent (in document
/// order) element whose own `<desc>` sets `origin-marker: true` -- mirrors
/// Go's last-write-wins `Document.OriginMarker` assignment in
/// attachSvgDesc, since svgToSvgxElement visits in document order too.
fn build_svgx_tree<'a>(
    node: roxmltree::Node<'a, 'a>,
    origin_marker: &mut Option<roxmltree::Node<'a, 'a>>,
) -> SvgxElement<'a> {
    let gcode_desc = own_gcode_desc(node);
    if let Some(desc) = &gcode_desc {
        if desc.origin_marker {
            *origin_marker = Some(node);
        }
    }
    let children = node
        .children()
        .filter(|c| c.is_element())
        .map(|c| build_svgx_tree(c, origin_marker))
        .collect();
    SvgxElement {
        node,
        gcode_desc,
        effective_desc: None,
        children,
    }
}

fn compute_effective_descs(element: &mut SvgxElement, inherited: Option<GCodeDesc>) {
    let effective = element.gcode_desc.clone().or(inherited);
    element.effective_desc = effective.clone();
    for child in &mut element.children {
        compute_effective_descs(child, effective.clone());
    }
}

/// Derives the physical-to-user-unit scale from the SVG root element.
/// Falls back to 1.0 (1 user unit = 1mm) when dimensions or viewBox are
/// absent or degenerate.
fn compute_mm_per_unit(doc: &roxmltree::Document) -> f64 {
    let root = doc.root_element();
    let Some(view_box_str) = doc_view_box(&root) else {
        return 1.0;
    };
    let Ok(vb) = parse_view_box(view_box_str) else {
        return 1.0;
    };
    if vb.width == 0.0 {
        return 1.0;
    }
    let Some(width_str) = doc_width(&root) else {
        return 1.0;
    };
    let Ok(width_mm) = parse_length_mm(width_str) else {
        return 1.0;
    };
    if width_mm == 0.0 {
        return 1.0;
    }
    width_mm / vb.width
}

/// Validates and resolves the origin-marker node (if any) into its
/// (cx, cy), in raw SVG user units. Decided in
/// issue-019f1a9b-fa38-7267-baae-57c1c192015a-rust-port-plan.md: the
/// marked element must be a `<circle>`, and a missing/malformed cx/cy is a
/// load-time error, never a panic -- tightening Go's current behavior,
/// which accepts any element with cx/cy and panics via MustParseNumber on
/// a bad attribute.
fn resolve_origin_marker(node: Option<roxmltree::Node>) -> Result<Option<(f64, f64)>, LoadError> {
    let Some(node) = node else { return Ok(None) };
    if node.tag_name().name() != "circle" {
        return Err(LoadError::OriginMarkerNotCircle(
            node.tag_name().name().to_string(),
        ));
    }
    let cx = node
        .attribute("cx")
        .ok_or(LoadError::OriginMarkerMissingAttribute("cx"))?;
    let cy = node
        .attribute("cy")
        .ok_or(LoadError::OriginMarkerMissingAttribute("cy"))?;
    let cx = parse_number(cx).map_err(|source| LoadError::OriginMarkerInvalidAttribute {
        attribute: "cx",
        source,
    })?;
    let cy = parse_number(cy).map_err(|source| LoadError::OriginMarkerInvalidAttribute {
        attribute: "cy",
        source,
    })?;
    Ok(Some((cx, cy)))
}

#[derive(Debug)]
pub struct SvgxDocument<'a> {
    pub filename: String,
    pub root: SvgxElement<'a>,
    pub origin_marker: Option<(f64, f64)>,
    pub mm_per_unit: f64,
}

/// Builds an SvgxDocument from an already-parsed roxmltree::Document (see
/// document::parse_svg_document). Split from parsing so callers control
/// how the source text is kept alive alongside the borrowed tree.
pub fn load_document<'a>(
    filename: impl Into<String>,
    doc: &'a roxmltree::Document<'a>,
) -> Result<SvgxDocument<'a>, LoadError> {
    let mm_per_unit = compute_mm_per_unit(doc);
    let mut origin_marker_node = None;
    let mut root = build_svgx_tree(doc.root_element(), &mut origin_marker_node);
    compute_effective_descs(&mut root, None);
    let origin_marker = resolve_origin_marker(origin_marker_node)?;
    Ok(SvgxDocument {
        filename: filename.into(),
        root,
        origin_marker,
        mm_per_unit,
    })
}

impl<'a> SvgxDocument<'a> {
    /// Takes an existing GCodeWriter rather than owning the output sink
    /// itself, so a caller (the CLI) can carve several SvgxDocuments into
    /// one shared output stream in sequence -- matching Go's CLI, which
    /// opens one io.Writer and calls svgxDoc.Carve(output) once per input
    /// file. Resets `writer.ctx` to a fresh CarveCtx (keeping only
    /// mm_per_unit) at the start, mirroring Go's `Ctx: CarveCtx{MmPerUnit:
    /// this.MmPerUnit}` on every Carve() call.
    pub fn carve(
        &self,
        writer: &mut GCodeWriter,
        resolver: &dyn DepthResolver,
    ) -> Result<(), CarveError> {
        writer.ctx = CarveCtx {
            mm_per_unit: self.mm_per_unit,
            ..Default::default()
        };
        writer.comment(&format!("Source: {}\n", self.filename));

        let mut transforms = vec![Transform {
            name: "scale".to_string(),
            parameters: vec![1.0, -1.0],
        }];
        if let Some((cx, cy)) = self.origin_marker {
            writer.comment(&format!("Origin: ({cx},{cy})\n"));
            transforms.push(Transform {
                name: "translate".to_string(),
                parameters: vec![-cx, -cy],
            });
        }

        self.root.carve(writer, transforms, resolver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse_svg_document;
    use crate::gcode_desc::{CutContext, ResolveError};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct SharedBuf(Rc<RefCell<Vec<u8>>>);
    impl std::io::Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct NoopResolver;
    impl DepthResolver for NoopResolver {
        fn resolve(&self, sentinel: &str, _ctx: &CutContext) -> Result<f64, ResolveError> {
            Err(ResolveError(format!("unresolved sentinel '{sentinel}'")))
        }
    }

    #[test]
    fn test_compute_mm_per_unit_defaults_to_one() {
        let doc = parse_svg_document("<svg></svg>").unwrap();
        assert_eq!(compute_mm_per_unit(&doc), 1.0);
    }

    #[test]
    fn test_compute_mm_per_unit_from_viewbox_and_width() {
        let doc = parse_svg_document(r#"<svg width="12cm" viewBox="0 0 1200 525"></svg>"#).unwrap();
        assert_eq!(compute_mm_per_unit(&doc), 120.0 / 1200.0);
    }

    #[test]
    fn test_effective_desc_inheritance() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <g><desc>carve-depth: 5mm</desc>
                <path d="M 0,0 L 1,1"/>
            </g>
        </svg>"#;
        let doc = parse_svg_document(xml).unwrap();
        let loaded = load_document("test.svg", &doc).unwrap();
        // root -> g -> path
        let g = &loaded.root.children[0];
        let path = &g.children[0];
        assert!(g.gcode_desc.is_some());
        assert!(path.gcode_desc.is_none());
        assert_eq!(
            path.effective_desc.as_ref().unwrap().carve_depth,
            Some("5mm".to_string())
        );
    }

    #[test]
    fn test_origin_marker_requires_circle() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <rect x="0" y="0" width="1" height="1"><desc>origin-marker: true</desc></rect>
        </svg>"#;
        let doc = parse_svg_document(xml).unwrap();
        let err = load_document("test.svg", &doc).unwrap_err();
        assert!(matches!(err, LoadError::OriginMarkerNotCircle(_)));
    }

    #[test]
    fn test_origin_marker_requires_cx_cy() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <circle><desc>origin-marker: true</desc></circle>
        </svg>"#;
        let doc = parse_svg_document(xml).unwrap();
        let err = load_document("test.svg", &doc).unwrap_err();
        assert!(matches!(err, LoadError::OriginMarkerMissingAttribute("cx")));
    }

    #[test]
    fn test_origin_marker_rejects_malformed_cx() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <circle cx="not-a-number" cy="20"><desc>origin-marker: true</desc></circle>
        </svg>"#;
        let doc = parse_svg_document(xml).unwrap();
        let err = load_document("test.svg", &doc).unwrap_err();
        assert!(matches!(
            err,
            LoadError::OriginMarkerInvalidAttribute {
                attribute: "cx",
                ..
            }
        ));
    }

    #[test]
    fn test_origin_marker_valid_circle_shifts_carve_output() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <circle cx="10" cy="20" r="1"><desc>origin-marker: true</desc></circle>
        </svg>"#;
        let doc = parse_svg_document(xml).unwrap();
        let loaded = load_document("test.svg", &doc).unwrap();
        assert_eq!(loaded.origin_marker, Some((10.0, 20.0)));

        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        loaded.carve(&mut writer, &NoopResolver).unwrap();
        let output = String::from_utf8(buf.borrow().clone()).unwrap();
        assert!(output.contains("Origin: (10,20)"));
    }
}
