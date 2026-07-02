//! Port of svgx/PathCursor.go.

use crate::transform::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PathCursor {
    pub x: f64,
    pub y: f64,
    pub start_x: f64,
    pub start_y: f64,
}

/// Go embeds PathCursor in CarveCtx so callers write `ctx.X`/`ctx.Y`
/// directly; Deref/DerefMut below reproduce that field-promotion ergonomic
/// in Rust so the (much larger) path-handler port in svgx_element.rs can
/// stay a faithful, low-diff translation of the Go source.
#[derive(Debug, Clone, Default)]
pub struct CarveCtx {
    pub cursor: PathCursor,
    pub z: f64,
    pub depth: f64,
    pub safe_height: f64,
    pub mm_per_unit: f64,
    pub transforms: Vec<Transform>,
    pub using_absolute: bool,
}

impl std::ops::Deref for CarveCtx {
    type Target = PathCursor;
    fn deref(&self) -> &PathCursor {
        &self.cursor
    }
}

impl std::ops::DerefMut for CarveCtx {
    fn deref_mut(&mut self) -> &mut PathCursor {
        &mut self.cursor
    }
}

pub type PathHandler = fn(&[f64], CarveCtx) -> (Vec<String>, CarveCtx);
