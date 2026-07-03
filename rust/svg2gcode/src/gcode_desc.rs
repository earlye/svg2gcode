//! Port of svgx/GCodeDesc.go, plus the CutDepth/DepthResolver/CutContext
//! design from issue-019f1a9b-fa38-7267-baae-57c1c192015a-rust-port-plan.md.
//!
//! Go's `CarveDepth` is a plain numeric string; a non-numeric value like
//! "full" silently falls back to a default depth (see `GetCarveDepth`'s Go
//! source: `if err != nil { return defaultResult }`). This port replaces
//! that with an explicit `CutDepth::Sentinel` variant and a `DepthResolver`
//! trait the caller supplies (e.g. xcarve-controller resolving "full"
//! against probe-measured stock thickness) -- and fails loudly instead of
//! ever substituting a default when a sentinel can't be resolved.

use serde::Deserialize;

use crate::length::parse_length_mm;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct GCodeDesc {
    #[serde(rename = "origin-marker", default)]
    pub origin_marker: bool,
    #[serde(rename = "carve-depth", default)]
    pub carve_depth: Option<String>,
    #[serde(rename = "safe-height", default)]
    pub safe_height: Option<String>,
}

impl GCodeDesc {
    /// Parses the YAML content of an SVG `<desc>` tag. Returns `None` if the
    /// content has no GCodeDesc mapping at all (e.g. comment-only prose, as
    /// in examples/arcs.svg's `<desc>`) -- mirroring Go leaving the
    /// `*GCodeDesc` pointer nil rather than allocating an all-default
    /// struct that would incorrectly override an inherited GCodeDesc from
    /// higher up the element tree.
    pub fn parse(text: &str) -> Option<GCodeDesc> {
        match serde_yaml::from_str::<serde_yaml::Value>(text) {
            Ok(serde_yaml::Value::Null) => None,
            Ok(value) => serde_yaml::from_value(value).ok(),
            Err(_) => None,
        }
    }

    /// Mirrors Go's GetSafeHeight: silently falls back to `default_result`
    /// on a missing/unparseable value. Safe-height isn't part of the
    /// sentinel design -- it's always a concrete retraction height.
    pub fn get_safe_height(&self, default_result: f64) -> f64 {
        self.safe_height
            .as_deref()
            .and_then(|s| parse_length_mm(s).ok())
            .unwrap_or(default_result)
    }

    /// Returns the element's carve depth, distinguishing a fixed numeric
    /// depth from an unresolved sentinel. `None` means no carve-depth was
    /// specified at all (the element isn't carved).
    pub fn carve_depth(&self) -> Option<CutDepth> {
        self.carve_depth.as_deref().map(CutDepth::parse)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CutDepth {
    Fixed(f64),
    Sentinel(String),
}

impl CutDepth {
    pub fn parse(raw: &str) -> Self {
        match parse_length_mm(raw) {
            Ok(mm) => CutDepth::Fixed(mm),
            Err(_) => CutDepth::Sentinel(raw.to_string()),
        }
    }
}

/// Where in the drawing a depth sentinel needs resolving: millimeters, post
/// origin-transform, post-MmPerUnit scale (matching every other coordinate
/// in the pipeline). Currently built once per element, at that element's
/// first cut waypoint (see `SvgxElement::carve_depth_ramp`) -- not once per
/// point or per ramp pass. A per-point-varying resolution (e.g. a future
/// height-map-backed resolver following an uneven surface within a single
/// path) is a deliberately deferred follow-up, not what this type promises
/// today -- see issue-019f28e7-3d14-70a0-b290-fdb9e1af36fc-top-wasteboard-
/// depth-references.md.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutContext {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ResolveError(pub String);

/// Resolves a depth sentinel (e.g. "full") to a concrete depth in
/// millimeters. svg2gcode has no concept of what a sentinel means or how
/// it's measured (probes, fixed stock thickness, a height map, ...) --
/// that's entirely up to the caller. An `Err` here must abort gcode
/// generation for the job; svg2gcode never silently substitutes a default.
pub trait DepthResolver {
    fn resolve(&self, sentinel: &str, context: &CutContext) -> Result<f64, ResolveError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_safe_height() {
        let desc = GCodeDesc {
            safe_height: Some("10mm".to_string()),
            ..Default::default()
        };
        assert_eq!(desc.get_safe_height(0.0), 10.0);

        let missing = GCodeDesc::default();
        assert_eq!(missing.get_safe_height(7.0), 7.0);

        let invalid = GCodeDesc {
            safe_height: Some("not-a-length".to_string()),
            ..Default::default()
        };
        assert_eq!(invalid.get_safe_height(7.0), 7.0);
    }

    #[test]
    fn test_carve_depth() {
        let fixed = GCodeDesc {
            carve_depth: Some("10mm".to_string()),
            ..Default::default()
        };
        assert_eq!(fixed.carve_depth(), Some(CutDepth::Fixed(10.0)));

        let sentinel = GCodeDesc {
            carve_depth: Some("full".to_string()),
            ..Default::default()
        };
        assert_eq!(
            sentinel.carve_depth(),
            Some(CutDepth::Sentinel("full".to_string()))
        );

        let absent = GCodeDesc::default();
        assert_eq!(absent.carve_depth(), None);
    }

    #[test]
    fn test_parse_desc_yaml() {
        // Comment-only content (as in examples/arcs.svg's <desc>) must not
        // allocate an all-default GCodeDesc that would shadow an inherited one.
        assert_eq!(GCodeDesc::parse("# just a comment\n# more comment\n"), None);

        assert_eq!(
            GCodeDesc::parse("carve-depth: full\nsafe-height: 10mm\n"),
            Some(GCodeDesc {
                origin_marker: false,
                carve_depth: Some("full".to_string()),
                safe_height: Some("10mm".to_string()),
            })
        );

        assert_eq!(
            GCodeDesc::parse("origin-marker: true\n"),
            Some(GCodeDesc {
                origin_marker: true,
                carve_depth: None,
                safe_height: None
            })
        );

        // Genuinely malformed YAML also yields None, matching Go's
        // unmarshal-error-leaves-pointer-nil behavior.
        assert_eq!(GCodeDesc::parse(": : :not yaml"), None);
    }

    struct FixedResolver(f64);
    impl DepthResolver for FixedResolver {
        fn resolve(&self, _sentinel: &str, _context: &CutContext) -> Result<f64, ResolveError> {
            Ok(self.0)
        }
    }

    struct FailingResolver;
    impl DepthResolver for FailingResolver {
        fn resolve(&self, sentinel: &str, _context: &CutContext) -> Result<f64, ResolveError> {
            Err(ResolveError(format!("no resolver for '{sentinel}'")))
        }
    }

    #[test]
    fn test_depth_resolver() {
        let ctx = CutContext { x: 32.0, y: 45.0 };
        assert_eq!(FixedResolver(3.2).resolve("full", &ctx), Ok(3.2));
        assert!(FailingResolver.resolve("full", &ctx).is_err());
    }
}
