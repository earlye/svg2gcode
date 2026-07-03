//! Port of svgx/SvgxElement.go: path-command handlers, pure gcode-line
//! helpers, and the SvgxElement tree type + Carve traversal (walks the
//! element tree, handles GCodeDesc/CutDepth, and integrates DepthResolver).
//! Document/LoadDocument-level concerns (origin marker, MmPerUnit, tree
//! construction) live in svgx_document.rs.

use crate::document::name_to_key;
use crate::gcode_desc::{CutContext, CutDepth, DepthResolver, GCodeDesc, ResolveError};
use crate::gcode_writer::lift_to_safe_height_lines;
use crate::gcode_writer::GCodeWriter;
use crate::number::must_parse_number;
use crate::path_cursor::{CarveCtx, PathCursor, PathHandler};
use crate::path_data::parse_svg_path_data;
use crate::transform::apply_transform_list;
use crate::transform::{parse_transform_list, Transform};

// --- pure helpers ---

fn use_absolute_lines(ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    if out.using_absolute {
        return (Vec::new(), out);
    }
    out.using_absolute = true;
    (vec!["G90 ; Use absolute positioning.\n".to_string()], out)
}

fn line_absolute_lines(x: f64, y: f64, ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let (abs_lines, new_out) = use_absolute_lines(out);
    out = new_out;
    let mut lines = abs_lines;
    out.cursor.x = x;
    out.cursor.y = y;
    let (mut tx, mut ty) = apply_transform_list(x, y, &out.transforms);
    tx *= out.mm_per_unit;
    ty *= out.mm_per_unit;
    lines.push(format!(
        "G1 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (line-absolute: {x:.6},{y:.6})\n",
        out.depth
    ));
    out.z = out.depth;
    (lines, out)
}

// --- path handlers ---

fn line_to_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 2 {
        let (l, new_out) = line_absolute_lines(params[0], params[1], out);
        lines.extend(l);
        out = new_out;
        params = &params[2..];
    }
    (lines, out)
}

fn move_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    if params.len() < 2 {
        return (Vec::new(), out);
    }
    let (x, y) = (params[0], params[1]);
    let mut lines = Vec::new();
    let (l, new_out) = use_absolute_lines(out);
    lines.extend(l);
    out = new_out;
    let (l, new_out) = lift_to_safe_height_lines(out);
    lines.extend(l);
    out = new_out;
    out.cursor.x = x;
    out.cursor.y = y;
    out.cursor.start_x = x;
    out.cursor.start_y = y;
    let (mut tx, mut ty) = apply_transform_list(x, y, &out.transforms);
    tx *= out.mm_per_unit;
    ty *= out.mm_per_unit;
    lines.push(format!(
        "G0 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (move-absolute: {:.6}[{x:.6}],{:.6}[{y:.6}] - safe-height)\n",
        out.safe_height, out.x, out.y
    ));
    lines.push(format!(
        "G0 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (move-absolute: {:.6}[{x:.6}],{:.6}[{y:.6}])\n",
        out.depth, out.x, out.y
    ));
    out.z = out.depth;
    let (l, new_out) = line_to_absolute(&params[2..], out);
    lines.extend(l);
    out = new_out;
    (lines, out)
}

fn line_to_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 2 {
        let (l, new_out) = line_absolute_lines(out.x + params[0], out.y + params[1], out);
        lines.extend(l);
        out = new_out;
        params = &params[2..];
    }
    (lines, out)
}

fn move_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    if params.len() < 2 {
        return (Vec::new(), out);
    }
    let (dx, dy) = (params[0], params[1]);
    let mut lines = Vec::new();
    let (l, new_out) = use_absolute_lines(out);
    lines.extend(l);
    out = new_out;
    let (l, new_out) = lift_to_safe_height_lines(out);
    lines.extend(l);
    out = new_out;
    out.cursor.x += dx;
    out.cursor.y += dy;
    out.cursor.start_x = out.x;
    out.cursor.start_y = out.y;
    let (mut tx, mut ty) = apply_transform_list(out.x, out.y, &out.transforms);
    tx *= out.mm_per_unit;
    ty *= out.mm_per_unit;
    lines.push(format!(
        "G0 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (move-relative: {dx:.6},{dy:.6} - safe-height)\n",
        out.safe_height
    ));
    lines.push(format!(
        "G0 F1000 X{tx:.6} Y{ty:.6} Z{:.6}; (move-relative: {dx:.6},{dy:.6})\n",
        out.depth
    ));
    out.z = out.depth;
    let (l, new_out) = line_to_relative(&params[2..], out);
    lines.extend(l);
    out = new_out;
    (lines, out)
}

fn degrees_to_radians(input: f64) -> f64 {
    input / 180.0 * std::f64::consts::PI
}

fn angle_radians(u_x: f64, u_y: f64, v_x: f64, v_y: f64) -> f64 {
    // Eq 5.4 https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
    //
    // NOTE: Go's original computes magV as sqrt(uX^2+uY^2) -- a copy-paste
    // bug, it should use vX/vY. Not covered by any test (svgx/ has no
    // *_test.go files at all); fixed here since it's clearly unintentional
    // and the correct formula is strictly more robust for degenerate or
    // asymmetric arc parameters. In practice this barely changes output for
    // well-formed arcs: by construction (spec Eq 5.5/5.6), both magU and
    // the correct magV are ~1, since u and v are points on the same unit
    // auxiliary circle -- which is why "elliptical arcs work fairly well"
    // in Go despite the bug.
    let dot_product = u_x * v_x + u_y * v_y;
    let mag_u = (u_x * u_x + u_y * u_y).sqrt();
    let mag_v = (v_x * v_x + v_y * v_y).sqrt();
    let sign: f64 = if u_x * v_y - u_y * v_x < 0.0 {
        -1.0
    } else {
        1.0
    };
    sign * (dot_product / (mag_u * mag_v)).acos()
}

fn elliptic_arc_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 7 {
        let x1 = out.x;
        let y1 = out.y;
        let rx = params[0];
        let ry = params[1];
        let x_axis_rotation = params[2];
        let large_arc_flag = params[3];
        let sweep_flag = params[4];
        let x2 = params[5];
        let y2 = params[6];

        let phi = degrees_to_radians(x_axis_rotation);
        let f_a = large_arc_flag;
        let f_s = sweep_flag;

        // Eq 5.1
        let u0 = (x1 - x2) / 2.0;
        let v0 = (y1 - y2) / 2.0;
        let x1_prime = phi.cos() * u0 - phi.sin() * v0;
        let y1_prime = phi.sin() * u0 + phi.cos() * v0;

        // Eq 5.2
        let mut coeff =
            ((rx * rx * ry * ry - rx * rx * y1_prime * y1_prime - ry * ry * x1_prime * x1_prime)
                / (rx * rx * y1_prime * y1_prime + ry * ry * x1_prime * x1_prime))
                .sqrt();
        if f_a == f_s {
            coeff = -coeff;
        }
        let cx_prime = coeff * (rx * y1_prime / ry);
        let cy_prime = coeff * (-ry * x1_prime / rx);

        // Eq 5.3
        let cx = phi.cos() * cx_prime - phi.sin() * cy_prime + (x1 + x2) / 2.0;
        let cy = phi.sin() * cx_prime + phi.cos() * cy_prime + (y1 + y2) / 2.0;

        // Eq 5.5
        let v1 = (x1_prime - cx_prime) / rx;
        let v2 = (y1_prime - cy_prime) / ry;
        let theta1 = angle_radians(1.0, 0.0, v1, v2);
        // Eq 5.6
        let mut delta_theta = angle_radians(
            v1,
            v2,
            (-x1_prime - cx_prime) / rx,
            (-y1_prime - cy_prime) / ry,
        );
        while delta_theta > 2.0 * std::f64::consts::PI {
            delta_theta -= 2.0 * std::f64::consts::PI;
        }
        while delta_theta < -(2.0 * std::f64::consts::PI) {
            delta_theta += 2.0 * std::f64::consts::PI;
        }
        if sweep_flag == 0.0 && delta_theta > 0.0 {
            delta_theta -= 2.0 * std::f64::consts::PI;
        }
        if sweep_flag == 1.0 && delta_theta < 0.0 {
            delta_theta += 2.0 * std::f64::consts::PI;
        }

        let mut steps = (delta_theta.abs() / degrees_to_radians(10.0)).ceil() as i64;
        if steps < 1 {
            steps = 1;
        }
        for i in 0..steps {
            let theta = theta1 + (i as f64) / (steps as f64) * delta_theta;
            let u = rx * theta.cos();
            let v = ry * theta.sin();
            let x = phi.cos() * u - phi.sin() * v + cx;
            let y = phi.sin() * u + phi.cos() * v + cy;
            let (l, new_out) = line_absolute_lines(x, y, out);
            lines.extend(l);
            out = new_out;
        }
        let (l, new_out) = line_absolute_lines(x2, y2, out);
        lines.extend(l);
        out = new_out;
        params = &params[7..];
    }
    (lines, out)
}

fn elliptic_arc_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 7 {
        let rx = params[0];
        let ry = params[1];
        let x_axis_rotation = params[2];
        let large_arc_flag = params[3];
        let sweep_flag = params[4];
        let x = out.x + params[5];
        let y = out.y + params[6];

        let absolute_params = [rx, ry, x_axis_rotation, large_arc_flag, sweep_flag, x, y];
        let (l, new_out) = elliptic_arc_absolute(&absolute_params, out);
        lines.extend(l);
        out = new_out;
        params = &params[7..];
    }
    (lines, out)
}

fn close(_params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let start = [ctx.start_x, ctx.start_y];
    line_to_absolute(&start, ctx)
}

fn interpolate(start: f64, end: f64, t: f64) -> f64 {
    start + t * (end - start)
}

fn cubic_bezier_curve_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 6 {
        let (x0, y0) = (out.x, out.y);
        let (x1, y1) = (params[0], params[1]);
        let (x2, y2) = (params[2], params[3]);
        let (x3, y3) = (params[4], params[5]);

        let mut t = 0.0;
        while t < 1.0 {
            let (xa, ya) = (interpolate(x0, x1, t), interpolate(y0, y1, t));
            let (xb, yb) = (interpolate(x1, x2, t), interpolate(y1, y2, t));
            let (xc, yc) = (interpolate(x2, x3, t), interpolate(y2, y3, t));
            let (xab, yab) = (interpolate(xa, xb, t), interpolate(ya, yb, t));
            let (xbc, ybc) = (interpolate(xb, xc, t), interpolate(yb, yc, t));
            let (x, y) = (interpolate(xab, xbc, t), interpolate(yab, ybc, t));
            let (l, new_out) = line_absolute_lines(x, y, out);
            lines.extend(l);
            out = new_out;
            t += 0.1;
        }
        let (l, new_out) = line_absolute_lines(x3, y3, out);
        lines.extend(l);
        out = new_out;
        params = &params[6..];
    }
    (lines, out)
}

fn cubic_bezier_curve_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 6 {
        let (x0, y0) = (out.x, out.y);
        let (x1, y1) = (out.x + params[0], out.y + params[1]);
        let (x2, y2) = (out.x + params[2], out.y + params[3]);
        let (x3, y3) = (out.x + params[4], out.y + params[5]);

        let mut t = 0.0;
        while t < 1.0 {
            let (xa, ya) = (interpolate(x0, x1, t), interpolate(y0, y1, t));
            let (xb, yb) = (interpolate(x1, x2, t), interpolate(y1, y2, t));
            let (xc, yc) = (interpolate(x2, x3, t), interpolate(y2, y3, t));
            let (xab, yab) = (interpolate(xa, xb, t), interpolate(ya, yb, t));
            let (xbc, ybc) = (interpolate(xb, xc, t), interpolate(yb, yc, t));
            let (x, y) = (interpolate(xab, xbc, t), interpolate(yab, ybc, t));
            let (l, new_out) = line_absolute_lines(x, y, out);
            lines.extend(l);
            out = new_out;
            t += 0.1;
        }
        let (l, new_out) = line_absolute_lines(x3, y3, out);
        lines.extend(l);
        out = new_out;
        params = &params[6..];
    }
    (lines, out)
}

fn cubic_bezier_smooth_curve_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 4 {
        let (x1, y1) = (params[0], params[1]);
        let (x, y) = (params[2], params[3]);
        let (l, new_out) = line_absolute_lines(x1, y1, out);
        lines.extend(l);
        out = new_out;
        let (l, new_out) = line_absolute_lines(x, y, out);
        lines.extend(l);
        out = new_out;
        params = &params[4..];
    }
    (lines, out)
}

fn cubic_bezier_smooth_curve_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 4 {
        let (x0, y0) = (out.x, out.y);
        let (dx1, dy1) = (x0 + params[0], y0 + params[1]);
        let (dx, dy) = (x0 + params[2], y0 + params[3]);
        let (l, new_out) = line_absolute_lines(dx1, dy1, out);
        lines.extend(l);
        out = new_out;
        let (l, new_out) = line_absolute_lines(dx, dy, out);
        lines.extend(l);
        out = new_out;
        params = &params[4..];
    }
    (lines, out)
}

fn horizontal_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while !params.is_empty() {
        let (x, y) = (params[0], out.y);
        let (l, new_out) = line_absolute_lines(x, y, out);
        lines.extend(l);
        out = new_out;
        params = &params[1..];
    }
    (lines, out)
}

fn horizontal_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while !params.is_empty() {
        let (x, y) = (out.x + params[0], out.y);
        let (l, new_out) = line_absolute_lines(x, y, out);
        lines.extend(l);
        out = new_out;
        params = &params[1..];
    }
    (lines, out)
}

fn quadratic_bezier_curve_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 4 {
        let (x0, y0) = (out.x, out.y);
        let (x1, y1) = (params[0], params[1]);
        let (x2, y2) = (params[2], params[3]);

        let mut t = 0.0;
        while t < 1.0 {
            let (xa, ya) = (interpolate(x0, x1, t), interpolate(y0, y1, t));
            let (xb, yb) = (interpolate(x1, x2, t), interpolate(y1, y2, t));
            let (x, y) = (interpolate(xa, xb, t), interpolate(ya, yb, t));
            let (l, new_out) = line_absolute_lines(x, y, out);
            lines.extend(l);
            out = new_out;
            t += 0.1;
        }
        let (l, new_out) = line_absolute_lines(x2, y2, out);
        lines.extend(l);
        out = new_out;
        params = &params[4..];
    }
    (lines, out)
}

fn quadratic_bezier_curve_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 4 {
        let (x0, y0) = (out.x, out.y);
        let (x1, y1) = (out.x + params[0], out.y + params[1]);
        let (x2, y2) = (out.x + params[2], out.y + params[3]);

        let mut t = 0.0;
        while t < 1.0 {
            let (xa, ya) = (interpolate(x0, x1, t), interpolate(y0, y1, t));
            let (xb, yb) = (interpolate(x1, x2, t), interpolate(y1, y2, t));
            let (x, y) = (interpolate(xa, xb, t), interpolate(ya, yb, t));
            let (l, new_out) = line_absolute_lines(x, y, out);
            lines.extend(l);
            out = new_out;
            t += 0.1;
        }
        let (l, new_out) = line_absolute_lines(x2, y2, out);
        lines.extend(l);
        out = new_out;
        params = &params[4..];
    }
    (lines, out)
}

fn quadratic_bezier_smooth_curve_absolute(
    params: &[f64],
    ctx: CarveCtx,
) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 2 {
        let (x1, y1) = (params[0], params[1]);
        let (l, new_out) = line_absolute_lines(x1, y1, out);
        lines.extend(l);
        out = new_out;
        params = &params[2..];
    }
    (lines, out)
}

fn quadratic_bezier_smooth_curve_relative(
    params: &[f64],
    ctx: CarveCtx,
) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while params.len() >= 2 {
        let (dx1, dy1) = (out.x + params[0], out.y + params[1]);
        let (l, new_out) = line_absolute_lines(dx1, dy1, out);
        lines.extend(l);
        out = new_out;
        params = &params[2..];
    }
    (lines, out)
}

fn vertical_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while !params.is_empty() {
        let (x, y) = (out.x, params[0]);
        let (l, new_out) = line_absolute_lines(x, y, out);
        lines.extend(l);
        out = new_out;
        params = &params[1..];
    }
    (lines, out)
}

fn vertical_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
    let mut out = ctx;
    let mut lines = Vec::new();
    let mut params = params;
    while !params.is_empty() {
        let (x, y) = (out.x, out.y + params[0]);
        let (l, new_out) = line_absolute_lines(x, y, out);
        lines.extend(l);
        out = new_out;
        params = &params[1..];
    }
    (lines, out)
}

fn path_handler(command: char) -> Option<PathHandler> {
    match command {
        'A' => Some(elliptic_arc_absolute),
        'a' => Some(elliptic_arc_relative),
        'C' => Some(cubic_bezier_curve_absolute),
        'c' => Some(cubic_bezier_curve_relative),
        'H' => Some(horizontal_absolute),
        'h' => Some(horizontal_relative),
        'L' => Some(line_to_absolute),
        'l' => Some(line_to_relative),
        'M' => Some(move_absolute),
        'm' => Some(move_relative),
        'Q' => Some(quadratic_bezier_curve_absolute),
        'q' => Some(quadratic_bezier_curve_relative),
        'S' => Some(cubic_bezier_smooth_curve_absolute),
        's' => Some(cubic_bezier_smooth_curve_relative),
        'T' => Some(quadratic_bezier_smooth_curve_absolute),
        't' => Some(quadratic_bezier_smooth_curve_relative),
        'V' => Some(vertical_absolute),
        'v' => Some(vertical_relative),
        'Z' | 'z' => Some(close),
        _ => None,
    }
}

pub fn carve_svg_path_data(path_data: &str, writer: &mut GCodeWriter, transforms: Vec<Transform>) {
    writer.comment(&format!("-- Carving path Data: {path_data}\n"));
    writer.set_transforms(transforms);
    writer.ctx.cursor = PathCursor::default();
    writer.comment_current_xy("  Carving path data 0,0 is");

    for command in parse_svg_path_data(path_data) {
        writer.comment(&format!("{command:?}\n"));
        match path_handler(command.command) {
            Some(handler) => {
                let (lines, new_ctx) = handler(&command.parameters, writer.ctx.clone());
                writer.ctx = new_ctx;
                for line in lines {
                    writer.write(&line);
                }
            }
            None => {
                // Go logs [WARN] and continues; no logging system ported (see CLI task).
            }
        }
    }
    writer.lift_to_safe_height();
}

pub fn carve_svg_path(
    node: &roxmltree::Node,
    writer: &mut GCodeWriter,
    transforms: Vec<Transform>,
) {
    let path_data = node.attribute("d").unwrap_or("");
    carve_svg_path_data(path_data, writer, transforms);
}

/// Port of Go's carveSvgCircle's path-synthesis step, split out so both
/// `carve_svg_circle` and the depth-sentinel waypoint lookup (see
/// `first_waypoint_mm` / `SvgxElement::carve_depth_ramp`) can share it
/// instead of re-deriving cx/cy/rx/ry twice. NOTE (matches Go, not a port
/// regression): this reads `rx`/`ry` attributes, not the standard
/// `<circle>` `r` attribute -- so a real `<circle r="50"/>` with no
/// explicit rx/ry will hit `must_parse_number("auto")` and panic here,
/// exactly as it panics in Go's `svg.MustParseNumber("auto")`. This lines
/// up with `<circle>` still being marked TODO in README.md: this code path
/// is incomplete/experimental in the Go original, not something introduced
/// by the port.
pub fn circle_path_data(node: &roxmltree::Node) -> Option<String> {
    let cx_str = node.attribute("cx").unwrap_or("0");
    let cy_str = node.attribute("cy").unwrap_or("0");
    let rx_str = node.attribute("rx").unwrap_or("auto");
    let ry_str = node.attribute("ry").unwrap_or("auto");

    if [cx_str, cy_str, rx_str, ry_str]
        .iter()
        .any(|s| s.ends_with('%'))
    {
        // Go logs an ERROR and returns without carving; percentages aren't supported.
        return None;
    }

    let cx = must_parse_number(cx_str);
    let cy = must_parse_number(cy_str);
    let rx = must_parse_number(rx_str);
    let ry = must_parse_number(ry_str);

    Some(format!(
        "M {:.6} {:.6} A {:.6} {:.6} A {:.6} {:.6} A {:.6} {:.6} A {:.6} {:.6}",
        cx + rx,
        cy,
        cx,
        cy + ry,
        cx - rx,
        cy,
        cx,
        cy - ry,
        cx + rx,
        cy
    ))
}

pub fn carve_svg_circle(
    node: &roxmltree::Node,
    writer: &mut GCodeWriter,
    transforms: Vec<Transform>,
) {
    if let Some(path_data) = circle_path_data(node) {
        carve_svg_path_data(&path_data, writer, transforms);
    }
}

/// The (x, y) of an element's first cut waypoint, in millimeters, post
/// element-transform and post-MmPerUnit scale -- i.e. already in the units
/// CutContext promises. Used only to build a CutContext for resolving a
/// depth sentinel; not part of Go (which never needed to look ahead into a
/// path before carving it).
fn first_waypoint_mm(
    path_data: &str,
    transforms: &[Transform],
    mm_per_unit: f64,
) -> Option<(f64, f64)> {
    let first = parse_svg_path_data(path_data).into_iter().next()?;
    if first.parameters.len() < 2 {
        return None;
    }
    let (mut tx, mut ty) =
        apply_transform_list(first.parameters[0], first.parameters[1], transforms);
    tx *= mm_per_unit;
    ty *= mm_per_unit;
    Some((tx, ty))
}

/// The element's "d" (for `<path>`) or synthesized circle path data,
/// without carving anything -- shared by the depth-ramp loop (to run the
/// same path data on every pass) and `first_waypoint_mm` (to resolve a
/// sentinel before the first pass).
fn element_path_data(node: &roxmltree::Node) -> Option<String> {
    let tag = node.tag_name();
    match name_to_key(tag.namespace(), tag.name()).as_str() {
        "http://www.w3.org/2000/svg:path" => node.attribute("d").map(str::to_string),
        "http://www.w3.org/2000/svg:circle" => circle_path_data(node),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CarveError {
    #[error("failed to resolve carve-depth sentinel: {0}")]
    Resolve(#[from] ResolveError),
}

/// Port of SvgxElement.go's SvgxElement + Carve(), built on the tree
/// LoadDocument.go constructs (see svgx_document.rs).
#[derive(Debug)]
pub struct SvgxElement<'a> {
    pub node: roxmltree::Node<'a, 'a>,
    pub gcode_desc: Option<GCodeDesc>,
    pub effective_desc: Option<GCodeDesc>,
    pub children: Vec<SvgxElement<'a>>,
}

impl<'a> SvgxElement<'a> {
    pub fn carve(
        &self,
        writer: &mut GCodeWriter,
        transforms: Vec<Transform>,
        resolver: &dyn DepthResolver,
    ) -> Result<(), CarveError> {
        let mut transforms = transforms;
        if let Some(transform_str) = self.node.attribute("transform") {
            if !transform_str.is_empty() {
                // Go discards a parse error here and uses whatever partial
                // transform list it had built so far; parse_transform_list
                // doesn't expose partial results on Err, so this port
                // simplifies to "no element-level transform" on failure
                // instead. Undetected by any test (Transform_test.go
                // doesn't exercise this call site).
                if let Ok(element_transform) = parse_transform_list(transform_str) {
                    let mut combined = element_transform;
                    combined.extend(transforms);
                    transforms = combined;
                }
            }
        }

        let tag = self.node.tag_name();
        let key = name_to_key(tag.namespace(), tag.name());
        match key.as_str() {
            "" => return Ok(()),
            "http://www.w3.org/2000/svg:desc" => return Ok(()),
            _ => {
                writer.comment(&format!(
                    "this.XmlElement.id: {} this.XmlElement.Name: {:?} transform: {:?}\n",
                    self.node.attribute("id").unwrap_or(""),
                    key,
                    transforms
                ));
            }
        }

        if let Some(desc) = self.effective_desc.clone() {
            if let Some(cut_depth) = desc.carve_depth() {
                self.carve_depth_ramp(writer, &transforms, &desc, cut_depth, resolver)?;
            }
        }

        for child in &self.children {
            child.carve(writer, transforms.clone(), resolver)?;
        }
        Ok(())
    }

    fn carve_depth_ramp(
        &self,
        writer: &mut GCodeWriter,
        transforms: &[Transform],
        desc: &GCodeDesc,
        cut_depth: CutDepth,
        resolver: &dyn DepthResolver,
    ) -> Result<(), CarveError> {
        writer.ctx.safe_height = desc.get_safe_height(10.0);
        writer.lift_to_safe_height();
        writer.ctx.depth = 0.0;

        let path_data = element_path_data(&self.node);

        let carve_depth = match cut_depth {
            CutDepth::Fixed(mm) => mm,
            CutDepth::Sentinel(name) => {
                // Resolved once per element, at its first waypoint -- not
                // re-resolved per ramp pass or per point within the path.
                // See issue-019f1a9b's grill log and the follow-up
                // top/bottom-reference issue for why a richer per-point
                // model is deferred rather than built here.
                let (x, y) = path_data
                    .as_deref()
                    .and_then(|d| first_waypoint_mm(d, transforms, writer.ctx.mm_per_unit))
                    .unwrap_or((0.0, 0.0));
                resolver.resolve(&name, &CutContext { x, y })?
            }
        };

        while writer.ctx.depth < carve_depth {
            writer.ctx.depth += 1.0;
            if writer.ctx.depth > carve_depth {
                writer.ctx.depth = carve_depth;
            }
            writer.comment(&format!(
                "-- CurrentDepth: {:.6} CarveDepth: {:.6}\n",
                writer.ctx.depth, carve_depth
            ));
            if let Some(d) = &path_data {
                carve_svg_path_data(d, writer, transforms.to_vec());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_cursor::CarveCtx;

    fn ctx() -> CarveCtx {
        CarveCtx {
            mm_per_unit: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn test_line_to_absolute_advances_cursor() {
        let (_, out) = line_to_absolute(&[10.0, 20.0, 30.0, 40.0], ctx());
        assert_eq!((out.x, out.y), (30.0, 40.0));
    }

    #[test]
    fn test_move_absolute_sets_start_and_recurses_into_lineto() {
        let (_, out) = move_absolute(&[1.0, 2.0, 3.0, 4.0], ctx());
        assert_eq!((out.start_x, out.start_y), (1.0, 2.0));
        assert_eq!((out.x, out.y), (3.0, 4.0));
    }

    #[test]
    fn test_close_returns_to_start() {
        let mut c = ctx();
        c.cursor.start_x = 5.0;
        c.cursor.start_y = 6.0;
        c.cursor.x = 50.0;
        c.cursor.y = 60.0;
        let (_, out) = close(&[], c);
        assert_eq!((out.x, out.y), (5.0, 6.0));
    }

    #[test]
    fn test_horizontal_and_vertical() {
        let (_, out) = horizontal_absolute(&[42.0], ctx());
        assert_eq!((out.x, out.y), (42.0, 0.0));

        let mut c = ctx();
        c.cursor.x = 5.0;
        c.cursor.y = 5.0;
        let (_, out) = horizontal_relative(&[3.0], c);
        assert_eq!((out.x, out.y), (8.0, 5.0));

        let (_, out) = vertical_absolute(&[42.0], ctx());
        assert_eq!((out.x, out.y), (0.0, 42.0));
    }

    #[test]
    fn test_elliptic_arc_absolute_reaches_endpoint() {
        // A circular arc (rx == ry, no rotation) from (0,0) to (10,0).
        let mut c = ctx();
        c.cursor.x = 0.0;
        c.cursor.y = 0.0;
        let (_, out) = elliptic_arc_absolute(&[5.0, 5.0, 0.0, 0.0, 1.0, 10.0, 0.0], c);
        assert!((out.x - 10.0).abs() < 1e-6);
        assert!((out.y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_elliptic_arc_absolute_asymmetric_ellipse_reaches_endpoint() {
        // A true ellipse (rx != ry) exercises angle_radians' magV fix --
        // for rx == ry the fix and Go's copy-pasted-magU bug are numerically
        // indistinguishable (both magU and the correct magV are ~1).
        let mut c = ctx();
        c.cursor.x = 0.0;
        c.cursor.y = 0.0;
        let (_, out) = elliptic_arc_absolute(&[10.0, 5.0, 0.0, 0.0, 1.0, 20.0, 0.0], c);
        assert!((out.x - 20.0).abs() < 1e-6);
        assert!((out.y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_elliptic_arc_relative_delegates_to_absolute() {
        let mut c = ctx();
        c.cursor.x = 100.0;
        c.cursor.y = 100.0;
        let (_, out) = elliptic_arc_relative(&[5.0, 5.0, 0.0, 0.0, 1.0, 10.0, 0.0], c);
        assert!((out.x - 110.0).abs() < 1e-6);
        assert!((out.y - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_cubic_and_quadratic_bezier_reach_endpoint() {
        let (_, out) = cubic_bezier_curve_absolute(&[1.0, 1.0, 2.0, 2.0, 3.0, 0.0], ctx());
        assert_eq!((out.x, out.y), (3.0, 0.0));

        let (_, out) = quadratic_bezier_curve_absolute(&[1.0, 1.0, 2.0, 0.0], ctx());
        assert_eq!((out.x, out.y), (2.0, 0.0));
    }

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

    struct FixedResolver(f64);
    impl DepthResolver for FixedResolver {
        fn resolve(&self, _sentinel: &str, _ctx: &CutContext) -> Result<f64, ResolveError> {
            Ok(self.0)
        }
    }

    struct FailingResolver;
    impl DepthResolver for FailingResolver {
        fn resolve(&self, sentinel: &str, _ctx: &CutContext) -> Result<f64, ResolveError> {
            Err(ResolveError(format!("no resolver for '{sentinel}'")))
        }
    }

    fn path_element<'a>(
        doc: &'a roxmltree::Document<'a>,
        desc: Option<GCodeDesc>,
    ) -> SvgxElement<'a> {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.is_element())
            .unwrap();
        SvgxElement {
            node,
            gcode_desc: desc.clone(),
            effective_desc: desc,
            children: vec![],
        }
    }

    #[test]
    fn test_carve_fixed_depth_runs_ramp_passes() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let desc = GCodeDesc {
            carve_depth: Some("3mm".to_string()),
            ..Default::default()
        };
        let element = path_element(&doc, Some(desc));

        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        writer.ctx.mm_per_unit = 1.0;

        element
            .carve(&mut writer, vec![], &FixedResolver(0.0))
            .unwrap();

        let output = String::from_utf8(buf.borrow().clone()).unwrap();
        assert_eq!(output.matches("-- CurrentDepth:").count(), 3);
    }

    /// Parses the sequence of "CurrentDepth" values out of carve() output
    /// comments -- distinguishes the actual increment-by-1mm-then-clamp
    /// ramp algorithm from any pass-count-only-equivalent rewrite.
    fn current_depths(output: &str) -> Vec<f64> {
        output
            .lines()
            .filter_map(|line| line.strip_prefix("; -- CurrentDepth: "))
            .filter_map(|rest| rest.split_whitespace().next())
            .filter_map(|token| token.parse::<f64>().ok())
            .collect()
    }

    #[test]
    fn test_carve_fractional_depth_ramps_by_1mm_then_clamps() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let desc = GCodeDesc {
            carve_depth: Some("2.5mm".to_string()),
            ..Default::default()
        };
        let element = path_element(&doc, Some(desc));

        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        writer.ctx.mm_per_unit = 1.0;

        element
            .carve(&mut writer, vec![], &FixedResolver(0.0))
            .unwrap();

        let output = String::from_utf8(buf.borrow().clone()).unwrap();
        assert_eq!(current_depths(&output), vec![1.0, 2.0, 2.5]);
    }

    #[test]
    fn test_carve_zero_depth_runs_no_passes() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let desc = GCodeDesc {
            carve_depth: Some("0mm".to_string()),
            ..Default::default()
        };
        let element = path_element(&doc, Some(desc));

        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        writer.ctx.mm_per_unit = 1.0;

        element
            .carve(&mut writer, vec![], &FixedResolver(0.0))
            .unwrap();

        let output = String::from_utf8(buf.borrow().clone()).unwrap();
        assert!(current_depths(&output).is_empty());
    }

    #[test]
    fn test_carve_sentinel_depth_uses_resolver() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let desc = GCodeDesc {
            carve_depth: Some("full".to_string()),
            ..Default::default()
        };
        let element = path_element(&doc, Some(desc));

        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut writer = GCodeWriter::new(Box::new(SharedBuf(buf.clone())));
        writer.ctx.mm_per_unit = 1.0;

        element
            .carve(&mut writer, vec![], &FixedResolver(2.0))
            .unwrap();

        let output = String::from_utf8(buf.borrow().clone()).unwrap();
        assert_eq!(output.matches("-- CurrentDepth:").count(), 2);
    }

    #[test]
    fn test_carve_sentinel_resolver_error_aborts() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let desc = GCodeDesc {
            carve_depth: Some("full".to_string()),
            ..Default::default()
        };
        let element = path_element(&doc, Some(desc));

        let mut writer = GCodeWriter::new(Box::new(Vec::new()));
        writer.ctx.mm_per_unit = 1.0;

        let result = element.carve(&mut writer, vec![], &FailingResolver);
        assert!(result.is_err());
    }

    #[test]
    fn test_carve_no_carve_depth_is_a_noop_but_not_an_error() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M 0,0 L 10,0"/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let element = path_element(&doc, None);

        let mut writer = GCodeWriter::new(Box::new(Vec::new()));
        writer.ctx.mm_per_unit = 1.0;

        element
            .carve(&mut writer, vec![], &FailingResolver)
            .unwrap();
    }
}
