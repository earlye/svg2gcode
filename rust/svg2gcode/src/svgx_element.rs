//! Port of svgx/SvgxElement.go's path-command handlers and pure gcode-line
//! helpers. The SvgxElement tree type + Carve traversal (which walks the
//! element tree, handles GCodeDesc/CutDepth, and integrates DepthResolver)
//! lives in svgx_document.rs alongside LoadDocument, once the ramping/
//! sentinel-resolution design question is resolved.

use crate::gcode_writer::lift_to_safe_height_lines;
use crate::number::must_parse_number;
use crate::path_cursor::{CarveCtx, PathCursor, PathHandler};
use crate::path_data::parse_svg_path_data;
use crate::transform::apply_transform_list;
use crate::gcode_writer::GCodeWriter;
use crate::transform::Transform;

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
    let sign: f64 = if u_x * v_y - u_y * v_x < 0.0 { -1.0 } else { 1.0 };
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
        let mut coeff = ((rx * rx * ry * ry - rx * rx * y1_prime * y1_prime
            - ry * ry * x1_prime * x1_prime)
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

fn quadratic_bezier_smooth_curve_absolute(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
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

fn quadratic_bezier_smooth_curve_relative(params: &[f64], ctx: CarveCtx) -> (Vec<String>, CarveCtx) {
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

pub fn carve_svg_path(node: &roxmltree::Node, writer: &mut GCodeWriter, transforms: Vec<Transform>) {
    let path_data = node.attribute("d").unwrap_or("");
    carve_svg_path_data(path_data, writer, transforms);
}

/// Port of Go's carveSvgCircle. NOTE (matches Go, not a port regression):
/// this reads `rx`/`ry` attributes, not the standard `<circle>` `r`
/// attribute -- so a real `<circle r="50"/>` with no explicit rx/ry will
/// hit `must_parse_number("auto")` and panic here, exactly as it panics in
/// Go's `svg.MustParseNumber("auto")`. This lines up with `<circle>` still
/// being marked TODO in README.md: this code path is incomplete/
/// experimental in the Go original, not something introduced by the port.
pub fn carve_svg_circle(node: &roxmltree::Node, writer: &mut GCodeWriter, transforms: Vec<Transform>) {
    let cx_str = node.attribute("cx").unwrap_or("0");
    let cy_str = node.attribute("cy").unwrap_or("0");
    let rx_str = node.attribute("rx").unwrap_or("auto");
    let ry_str = node.attribute("ry").unwrap_or("auto");

    if [cx_str, cy_str, rx_str, ry_str].iter().any(|s| s.ends_with('%')) {
        // Go logs an ERROR and returns without carving; percentages aren't supported.
        return;
    }

    let cx = must_parse_number(cx_str);
    let cy = must_parse_number(cy_str);
    let rx = must_parse_number(rx_str);
    let ry = must_parse_number(ry_str);

    let path_data = format!(
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
    );
    carve_svg_path_data(&path_data, writer, transforms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_cursor::CarveCtx;

    fn ctx() -> CarveCtx {
        CarveCtx { mm_per_unit: 1.0, ..Default::default() }
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
}
