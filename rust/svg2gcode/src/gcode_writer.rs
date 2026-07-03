//! Port of svgx/GCodeWriter.go.

use std::io::Write as _;

use crate::path_cursor::CarveCtx;
use crate::transform::{apply_transform_list, Transform};

pub struct GCodeWriter {
    pub ctx: CarveCtx,
    pub writer: Box<dyn std::io::Write>,
}

impl GCodeWriter {
    pub fn new(writer: Box<dyn std::io::Write>) -> Self {
        Self {
            ctx: CarveCtx::default(),
            writer,
        }
    }

    /// Mirrors Go's GCodeWriter.Write, which also discards the underlying
    /// io.Writer's (n, err) return.
    pub fn write(&mut self, input: &str) {
        let _ = self.writer.write_all(input.as_bytes());
    }

    pub fn comment(&mut self, input: &str) {
        self.write(&format!("; {}\n", input.replace('\n', "\\n")));
    }

    pub fn set_transforms(&mut self, transforms: Vec<Transform>) {
        self.write(&format!("; Using transforms: {transforms:?}\n"));
        self.ctx.transforms = transforms;
    }

    pub fn comment_current_xy(&mut self, label: &str) {
        let (mut tx, mut ty) = apply_transform_list(self.ctx.x, self.ctx.y, &self.ctx.transforms);
        tx *= self.ctx.mm_per_unit;
        ty *= self.ctx.mm_per_unit;
        // The trailing "\n" here is intentional (matches Go), even though
        // comment() below also appends one -- it shows up as a literal
        // "\n" followed by a real newline in the emitted gcode comment.
        self.comment(&format!("{label} {tx:.6} {ty:.6}\n"));
    }

    pub fn lift_to_safe_height(&mut self) {
        let (lines, new_ctx) = lift_to_safe_height_lines(self.ctx.clone());
        self.ctx = new_ctx;
        for line in lines {
            self.write(&line);
        }
    }
}

/// Port of SvgxElement.go's liftToSafeHeightLines. It's a pure CarveCtx ->
/// (lines, CarveCtx) helper used only by GCodeWriter, so it lives here
/// rather than mirroring Go's (somewhat arbitrary) file boundary.
pub fn lift_to_safe_height_lines(ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    if out.z == out.safe_height {
        return (Vec::new(), out);
    }
    let (mut tx, mut ty) = apply_transform_list(out.x, out.y, &out.transforms);
    tx *= out.mm_per_unit;
    ty *= out.mm_per_unit;
    let line = format!(
        "G0 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (lift tosafe-height)\n",
        out.safe_height
    );
    out.z = out.safe_height;
    (vec![line], out)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_comment_escapes_newlines() {
        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        writer.comment("line one\nline two");
        assert_eq!(
            String::from_utf8(buf.borrow().clone()).unwrap(),
            "; line one\\nline two\n"
        );
    }

    #[test]
    fn test_lift_to_safe_height_lines_noop_when_already_at_safe_height() {
        let ctx = CarveCtx {
            z: 5.0,
            safe_height: 5.0,
            ..Default::default()
        };
        let (lines, out) = lift_to_safe_height_lines(ctx);
        assert!(lines.is_empty());
        assert_eq!(out.z, 5.0);
    }

    #[test]
    fn test_lift_to_safe_height_lines_emits_move() {
        let ctx = CarveCtx {
            z: 0.0,
            safe_height: 10.0,
            mm_per_unit: 1.0,
            ..Default::default()
        };
        let (lines, out) = lift_to_safe_height_lines(ctx);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("G0 F1000 X0.000000 Y0.000000 Z10.000000"));
        assert_eq!(out.z, 10.0);
    }
}
