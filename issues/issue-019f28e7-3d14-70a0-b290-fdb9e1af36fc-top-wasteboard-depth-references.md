# Depth specs need independent Top/Wasteboard references (future work, deferred)

## Context

Discovered while designing the `CutDepth`/`DepthResolver` API for depth
sentinels (e.g. `"full"`) during the svg2gcode Go-to-Rust port (see
`issue-019f1a9b-fa38-7267-baae-57c1c192015a-rust-port-plan.md`). The port
ships with a simpler single-value model — `CutDepth::Fixed(f64) |
Sentinel(String)`, already implemented in `rust/svg2gcode/src/gcode_desc.rs`
— and this richer model is explicitly **deferred**, not part of that work.

The user only owns a simple (non-height-mapping) probe today, and all real
stock they cut is flat enough that this is "purely academic" right now —
there's no real height-map/probing implementation in `xcarve-controller` to
design against yet (its `GrblMachine` is currently just a stub: every method
returns `NotConnected`). Revisit once that probing system is real and
requirements are concrete, rather than speculatively building this now.

## The idea

Depth specifications should eventually support two independently-resolvable
references, using CNC domain terms:

- **Top**: the local top-of-stock surface height at a given `(x, y)` —
  conceptually backed by a future height-map/probe system in
  `xcarve-controller`.
- **Wasteboard**: the machine/global Z reference (the sacrificial spoilboard
  the stock sits on) — i.e. absolute machine-relative, not surface-relative.

This lets a cut spec express things like:

- "0 thickness after the cut" — cut all the way through material down to
  the wasteboard (a through-cut).
- "0.5in thickness after the cut" — leave 0.5in of material remaining,
  i.e. bottom target = Wasteboard + 0.5in, referenced from the wasteboard
  upward.
- "0.5in depth of cut" — cut 0.5in deep from wherever the top surface is,
  referenced from Top downward, regardless of total material thickness.

The eventual design needs a reference enum (`Top | Wasteboard`) attached
independently to both a top-of-cut spec and a bottom-of-cut (depth) spec,
each of which can be `Fixed(value)` or `Sentinel(name)`, resolved via a
future `DepthResolver`-like mechanism that also receives which reference is
being asked about (not just `(x, y)`).

## Motivating scenarios

1. **Planing/flattening a convex workpiece.** The top surface varies by
   `(x, y)` per a height map, so you'd want to start cutting at the highest
   point (smallest circle) and progressively widen as you flatten toward a
   target plane. Here the TOP reference for toolpath sequencing is
   Top/surface-relative (follow the height map to know where material
   currently exists), but the BOTTOM target (the flat plane being planed to)
   is Wasteboard-relative (a fixed machine Z).

2. **Cutting a pocket for an inlay in a curved object.** Both the top
   (start) and bottom (depth) of the pocket should track the local surface
   height — i.e. both references are Top-relative, so the pocket has
   uniform depth through the material regardless of the surface's
   curvature.

## Relevant files

- `rust/svg2gcode/src/gcode_desc.rs` — current (simpler) `CutDepth`/
  `DepthResolver`/`CutContext` implementation this idea would eventually
  extend or replace.
- `../xcarve-controller/src/machine/grbl.rs` — `GrblMachine`, currently a
  stub with no probing/height-map capability.
- `issue-019f1a9b-fa38-7267-baae-57c1c192015a-rust-port-plan.md` — the
  parent port plan and grill log where the single-value model was decided
  for now.
