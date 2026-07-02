# Port svg2gcode to Rust as a shared library + standalone CLI

## Rationale

`xcarve-controller` (sibling project, `../xcarve-controller`) is a Rust/wxdragon
GUI app that drives a GRBL CNC machine. It needs to speak SVG natively instead
of shelling out to the existing Go `svg2gcode` binary. Reimplementing the
SVG/GCode logic twice (once in Go, once in Rust inside the controller) would
mean two copies of path/transform/arc math to keep in sync, and the controller
already has machine-state concepts (probing, work offsets) that the depth
metadata in SVGs should be able to use.

So: port `svg2gcode` to Rust as a **library crate**, with a thin **binary**
on top so people who just want gcode output (no controller, no GUI, no
hardware) can still use it standalone. `xcarve-controller` depends on the
library crate directly.

Dependency direction is `xcarve-controller -> svg2gcode`, never the reverse.
svg2gcode must not know about GRBL, probes, or wxdragon. **This is absolute,
no exceptions** — if some future consumer needs probe-like behavior, it
implements its own `DepthResolver`, same as `xcarve-controller`; svg2gcode
never grows consumer-specific awareness, which is exactly the
two-copies-to-keep-in-sync problem this port exists to avoid.

### The sentinel-depth problem

Today (Go, `svgx/GCodeDesc.go` + `svgx/SvgxElement.go:559-569`), carve depth
comes from YAML embedded in an SVG `<desc>` tag:

```go
type GCodeDesc struct {
    CarveDepth string `yaml:"carve-depth,omitempty"`
    SafeHeight string `yaml:"safe-height,omitempty"`
}
```

`CarveDepth` is parsed with `svg.MustParseNumber` — it's already a string in
the struct, but it's required to be numeric today. There's no symbolic value.

The user to write `carve-depth: full` in the SVG and have the
controller provide an input that svg2gcode can use to resolve "full"
to "all the way through the stock" using measured (probably by probe)
material thickness. svg2gcode itself has no concept of a probe or
measuring material thicknes and shouldn't grow one — it just needs to
stop assuming every depth spec is a literal number, and let the caller
(the controller, or the standalone CLI) supply the resolution.

## Plan

1. **Scaffold the Rust workspace** — **decided: two-crate workspace.**
   `Cargo.toml` workspace with two members: `svg2gcode` (lib) and
   `svg2gcode-cli` (bin). Rejected the single-crate-with-feature-flag
   alternative: a feature flag only keeps `clap` etc. out of
   `xcarve-controller`'s build if every consumer remembers
   `default-features = false`; a workspace makes the separation structural
   instead of a discipline problem, for the same amount of `Cargo.toml`
   ceremony.

2. **Port the SVG parsing layer** (`svg/` in Go: `ParseSvgDocument.go`,
   `ParseSvgPathData.go`, `XmlElement.go`, `Number.go`, `Transform.go`,
   `ViewBox.go`)
   - XML parsing → **decided: `roxmltree`**. It parses into an immutable DOM
     tree up front with `.parent()`/`.children()`/`.attribute()` already
     available — matching how `svgx/LoadDocument.go`'s `svgToSvgxElement`
     already builds a parent/child tree right after parsing (and relies on
     `.Parent` for things like origin-marker lookup). `SvgxElement` can wrap
     `roxmltree::Node`s instead of re-implementing tree construction on top
     of a stream of open/close events (which is what `quick-xml` would
     require). Rejected `quick-xml` for this reason, despite it already
     being transitively present in `xcarve-controller`'s `Cargo.lock` (via
     `wxdragon-macros`, not the controller's own code — not a real signal).
   - Path data tokenizer/parser → hand-rolled, mirroring existing arc/curve
     handling (per `bd9d825` — elliptical arcs already work reasonably well
     in Go; preserve that behavior and bring over edge-case awareness even
     though Go-side automated tests for arcs aren't written yet).
   - Transform stack → **decided: hand-rolled `Affine2 { a, b, c, d, e, f: f64 }`**,
     matching SVG's `matrix()` grammar 1:1. Rejected `nalgebra` (even though
     `xcarve-controller` already depends on it) — it's built for general
     N-dimensional linear algebra, which is dependency weight for unused API
     surface here, and SVG-space (2D) and machine-space (3D) transforms are
     different coordinate systems with no real math to share anyway. Can
     revisit if a future need for more general matrix ops emerges.
   - Number parsing (units, percentages) → straight port of `Number.go`.

3. **Port the SVGX layer** (`svgx/`: `SvgxElement.go`, `SvgxDocument.go`,
   `GCodeWriter.go`, `GCodeDesc.go`)
   - This is where the depth-sentinel redesign happens. Replace the
     Go `CarveDepth string` + `MustParseNumber` pattern with something like:
     ```rust
     pub enum CutDepth {
         Fixed(f64),
         Sentinel(String), // e.g. "full" — unresolved
     }
     ```
   - svg2gcode resolves `CutDepth::Fixed` directly. For `CutDepth::Sentinel`,
     it calls back into a caller-supplied resolver — a trait, e.g.:
     ```rust
     pub trait DepthResolver {
         fn resolve(&self, sentinel: &str, context: &CutContext) -> Result<f64, ResolveError>;
     }
     ```
     The standalone CLI ships a default resolver that errors out (or only
     knows fixed values); `xcarve-controller` supplies its own resolver
     backed by probe data. This keeps the dependency direction correct.
   - **Resolution granularity: per-move, not per-element.** A sentinel must
     be resolved at *every* move to a depth other than safe-height, not once
     per path/element and cached. Rationale: a hypothetical ultrasound
     height-map resolver needs the actual `(x, y)` of each cut waypoint —
     "how thick is the stock at (32, 45)?" — not just the depth at the
     path's first point. So `resolve()` is called per-waypoint during gcode
     emission, with a fresh `CutContext` each time (cheap to construct —
     it's just position + the effective desc in scope).
   - **Units, settled:** `CutContext.x`/`y` and the resolved depth are
     **millimeters**, post-origin-transform, post-`MmPerUnit` scale.
     `xcarve-controller` is assumed to operate internally in mm regardless
     of what it displays to the user (in/mm toggle is a UI-layer concern),
     so no unit negotiation is needed across the trait boundary.
   - **Fail loud, confirmed:** if `resolve()` returns an error, svg2gcode
     must abort gcode generation for that job rather than substituting a
     default — this replaces the current Go behavior (see below) where an
     unparseable depth silently falls back to 0.
   - `GCodeWriter` (safe-height lift logic, move/cut emission) ports fairly
     mechanically.
   - **Origin marker: decided.** Currently (Go, `SvgxDocument.Carve`) the
     `origin-marker: true` element can be *any* element with `cx`/`cy`
     attributes, and reads them via `svg.MustParseNumber` — panics on a
     missing/malformed attribute. The Rust port tightens this: the marked
     element **must be `<circle>`**; anything else (wrong tag, or a
     `<circle>` missing `cx`/`cy`) is a proper `Result::Err` at load time,
     never a panic.

4. **Port the CLI** (`cli/`, `main.go`, `consts/Version.go`)
   - Re-implement file/stdin input handling, `-o`/`--output`, `-v` verbosity
     flag using `clap`.
   - This binary uses the default (non-probe) `DepthResolver`.

5. **Scope: straight port of the current Go feature set only — decided.**
   This port is *not* gated on the README TODO list. v0 covers exactly what
   today's Go code does (paths, transforms, arcs, origin marker, safe-height/
   carve-depth incl. the new sentinel support) so `xcarve-controller` isn't
   blocked on features nobody's asked for yet, and a smaller port is easier
   to verify against the Go original before relying on it for real cuts.
   Everything on the README TODO list (`<circle>`, `<clipPath>`,
   `<defs>`/`<use>`, `<ellipse>`, `<line>`, `<pattern>`, `<polygon>`,
   `<polyline>`, `<rect>`, `<symbol>`, `<text>`/`<textPath>`/`<tspan>`,
   `<view>`, tabs, ramping carve, hatch-mode pocket carves, arbitrary-angle
   hatch pocket carves) is tracked as separate follow-on issues, not part of
   this plan.

6. **Tests**: bring over/expand on existing Go test coverage
   (`*_test.go` files in `svg/`, `util/`) as Rust unit tests, and add the
   arc edge-case tests called out as missing in `bd9d825` while porting that
   logic, rather than deferring them again.

7. **Wire up `xcarve-controller`**: add `svg2gcode` as a path/git dependency,
   implement its `DepthResolver` against probe-measured stock thickness, and
   replace any planned shell-out integration with direct library calls.

## Upstream refactor merged (`origin/master` @ 038cb2b, "Architecture refactor + unit conversion + arc deduplication")

Merged into this branch on 2026-07-01, fast-forward, no conflicts. It changes
several assumptions this plan was written against — noted here rather than
rewriting history above:

- **`GCodeDesc` now inherits down the tree.** `svgx/LoadDocument.go`'s new
  `computeEffectiveDescs` propagates a parent's `GCodeDesc` to children that
  don't specify their own, exposed as `EffectiveDesc`. `SvgxElement.Carve`
  now reads `EffectiveDesc`, not a parent-chain walk. **Implication for the
  Rust port:** `CutDepth`/sentinel resolution must happen against the
  *effective* (inherited) desc per leaf cuttable element, not per raw
  `<desc>` tag — a `"full"` sentinel set on a `<g>` should apply to every
  descendant path that doesn't override it.

- **Real unit normalization now exists**, answering the CutContext x/y-units
  question from the interview: `svg/Length.go`'s `ParseLengthMm` normalizes
  `carve-depth`/`safe-height` (and presumably geometry, via
  `SvgxDocument.MmPerUnit`, computed in `LoadDocument.go` from
  `viewBox`/physical `width`) into millimeters. `GCodeWriter.CommentCurrentXY`
  already does `tx *= MmPerUnit`. **Decision, now backed by precedent:**
  `CutContext.x/y` should be **millimeters**, post-origin-transform,
  post-`MmPerUnit` scale — matching what the rest of the pipeline already
  normalizes to, not raw SVG user-space units.

- **Current Go behavior silently swallows a would-be sentinel.**
  `GCodeDesc.GetCarveDepth`/`GetSafeHeight` do
  `mm, err := ParseLengthMm(...); if err != nil { return defaultResult }`.
  So on current `master`, writing `carve-depth: full` today doesn't error —
  it silently falls back to the default depth (0, per the call site in
  `SvgxElement.go:622`) with no warning. This is a live footgun and a
  concrete argument for the Rust port's `CutDepth::Sentinel` + fail-loud
  `DepthResolver` design already agreed on above: never silently substitute
  a default when a depth spec doesn't parse as a number.

- **Document loading moved out of `cli/` and into `svgx/LoadDocument.go`.**
  `loadSvgxDocument`/`svgToSvgxElement`/`attachSvgDesc` used to live in
  `cli/rootCmd.go`; they're now `svgx.LoadDocument` and friends, with zero
  cobra/flag dependencies. This *simplifies* the two-crate split (plan item 1
  and 4): `LoadDocument` maps cleanly onto the `svg2gcode` lib crate, and
  `cli/rootCmd.go` is now just flag parsing + wiring, mapping onto
  `svg2gcode-cli`. No CLI-only concerns need to be untangled from the parse
  path anymore.

- **New files to account for in the port:** `svg/Length.go` (→ plan item 2,
  unit parsing), `svgx/PathCursor.go` (`PathCursor`/`CarveCtx`/`PathHandler`
  — cursor+context threading for path emission, → plan item 3, replaces
  scattered writer state).

- Arc deduplication logic (also part of this refactor, in `SvgxElement.go`)
  not yet reviewed in detail — worth a pass before/during plan item 2 to
  make sure the Rust arc-handling port starts from the deduplicated version,
  not the one described in `bd9d825`.

## Open questions

None remaining — all branches resolved in the 2026-07-01 grill session below.

## Grill Log

### 2026-07-01

- Q: Single-crate-with-feature-flag vs. two-crate workspace? — A: Two-crate
  workspace, for sure.
- Q: Does `CutDepth`/`DepthResolver` resolution need X/Y position (for
  non-flat stock / height-map probing), and at what granularity — once per
  element or per move? — A: Yes, needs X/Y, in `CutContext`, and resolution
  must happen per-move (not cached per-element), so a future ultrasound
  height-map resolver can answer "how thick at (32,45)?" for every waypoint.
- Q: What units for `CutContext.x/y`? — A: Millimeters — confirmed against
  the just-merged `MmPerUnit`/`ParseLengthMm` precedent; `xcarve-controller`
  is assumed to work internally in mm regardless of display units.
- Q: Should the default CLI resolver hard-error or soft-fallback on an
  unresolved sentinel? — A: Fail loud — svg2gcode aborts generation rather
  than silently substituting a default (Go's current `GetCarveDepth`/
  `GetSafeHeight` silently fall back to 0 on parse failure, which is the
  behavior being explicitly rejected).
- Q: Should the origin-marker element be enforced as `<circle>`, and should
  bad/missing `cx`/`cy` be a panic or an error? — A: Enforce `<circle>`;
  malformed/missing attributes are a load-time `Result::Err`, never a panic.
- Q: (mid-interview) Merge `origin/master` and check for concerns — A: Merged
  cleanly (fast-forward, no conflicts); folded five concerns from the
  upstream refactor into the plan (see "Upstream refactor merged" section
  above) rather than filing separately.
- Q: XML crate — `quick-xml` or `roxmltree`? — A: `roxmltree` — matches the
  parent/child tree `SvgxElement` already builds; `quick-xml`'s presence in
  `xcarve-controller`'s lockfile is only transitive, not a real signal.
- Q: Transform stack — reuse `xcarve-controller`'s `nalgebra`, or hand-roll?
  — A: Hand-roll a small `Affine2` type matching SVG's `matrix()` grammar;
  `nalgebra` is unnecessary weight for 2D affine math, can revisit later if
  a real need for general matrix ops shows up.
- Q: Is the "svg2gcode knows nothing about GRBL/probes/wxdragon" rule
  absolute, or could future exceptions apply? — A: Absolute, no exceptions.
- Q: Does this port need to cover the full README TODO list / Go test
  parity before `xcarve-controller` can consume it? — A: No — straight port
  of the current Go feature set only; TODO list tracked as separate
  follow-on issues.
