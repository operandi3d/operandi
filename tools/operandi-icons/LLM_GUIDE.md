# Operandi icon generator: instructions for LLMs

> **Deprecated (Jason, 2026-11-20).** New icons are rendered with the icon
> studio in [`tools/icon-studio`](../icon-studio/README.md). Don't add icons
> here; this tool only keeps the existing SVGs, which the app uses as
> fallbacks and for icons not yet rendered.

Use this guide to regenerate, extend, or revise Operandi's programmatic SVG
icons. Read the actual source before editing; this guide describes the current
implementation, not features that are planned or assumed to exist.

## Start here

The tool is a Python drawing library and generator. An LLM translates an icon
concept into a small drawing function; the generator executes those functions
and exports SVGs. There is no natural-language input, set-selection option, or
automatic symbol-design engine in the command-line interface.

The tool lives at `tools/operandi-icons` in the Operandi repository. Work on
your own feature branch, and never make a second copy of the tool.

Before working:

1. Read the repository's `AGENTS.md` and any instructions applying to the tool
   directory. Check the current branch and working changes; preserve others' work.
2. Read `README.md`, this guide, and the relevant Python modules below.
3. Establish the requested icon names, meaning, and scope from the user's request
   and admitted Operandi specifications. Propose reasonable original concepts
   when visual details are unspecified.
4. Keep this work inside the standalone tooling and its outputs. Application
   integration is a separate task requiring the user's instruction.

Jason approved the appearance of the generated six-icon core set in the chat
that created this guide. That approval does not cover future icons, redesigns,
application integration, commits, or pushes; get Jason's visual approval for
new artwork before integrating it. Follow the repository's clean-room rules: use
Operandi's own specifications and approved original artwork; do not inspect or
copy another application's icons, screenshots, source, or restricted research.
Do not use image generation for the final SVG artwork.

## Files to understand

All paths in this table are relative to the tool directory.

| File | Responsibility |
|---|---|
| `generate.py` | CLI; builds every entry in `ICONS`, validates, writes files |
| `operandi_icons/conventions.py` | Palette, stroke constants, projection and scale |
| `operandi_icons/primitives.py` | Shared drawing and SVG serialization |
| `operandi_icons/icons.py` | Unit-cube geometry, six operation recipes, `ICONS` registry |
| `operandi_icons/previews.py` | Transparent PNGs, vector contact sheet, light/dark comparisons |
| `operandi_icons/validation.py` | Checks the serialized SVG and rendered bounds |
| `tests/test_icons.py` | Generation, convention, failure-case and preview tests |
| `reference/svg/` | Unchanged approved original SVGs, for comparisons only |
| `requirements.lock.txt` | Complete tested Python dependency versions |

Production geometry does not read the reference artwork. Comparison generation
does read it. Never extract paths from a reference SVG into an icon recipe or
edit a generated SVG as the permanent source of a change.

## Set up and run the existing six icons

Use Python 3.10 or later. On macOS/Linux, from the tool directory:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.lock.txt
.venv/bin/python generate.py
.venv/bin/python -m unittest discover -s tests -v
```

Reuse a working `.venv` instead of recreating it on every run. CairoSVG needs
the system Cairo library for raster rendering. If Cairo cannot load, check
whether it is installed and discoverable. On a Mac using Homebrew, its install
command is `brew install cairo`; follow the host's installation permissions.
On Windows, use the equivalent `.venv\Scripts\python.exe` commands and configure
Cairo for that platform; Windows setup has not been validated in this handoff.

Optional commands, also from the tool directory:

```sh
# Change the destination, not which icons are generated.
.venv/bin/python generate.py --output /absolute/path/to/review-folder

# Generate SVGs without loading references or the raster-rendering stack.
.venv/bin/python generate.py --svg-only
```

`--svg-only` still checks XML, conventions and geometric control bounds, but
skips rendered stroke-bounds checks, PNGs, and review sheets. It is useful while
developing new icons; it does not complete visual QA. That mode requires only
svgwrite. A normal full run requires the complete dependency stack.

Generation writes into its destination without clearing it first. After
removing or renaming an icon, old output files may remain. Use a fresh review
destination or remove only confirmed stale files belonging to your task.

## Drawing conventions to preserve

Every production icon must be independent, transparent, and have
`viewBox="0 0 32 32"`, default width/height 32, a title, and a description.
Labels and background rectangles belong only in review sheets.

Use `Projection` for mesh geometry. It maps model coordinates `(x, y, z)` to
canvas pixels using:

```text
screen_x = origin_x + unit * sqrt(3)/2 * (x - y)
screen_y = origin_y + unit * (0.5 * (x + y) - z)
```

The default origin is `(16, 15.75)` and the default unit length is `12.9` px.
Positive X projects down/right, positive Y down/left, and positive Z up.
The shared cube spans 0 to 1 on each model axis. Reuse `TOP`, `LEFT`, `RIGHT`,
and `cube()` from `icons.py` when appropriate. Existing optical sizes are
intentional; for a different silhouette, adjust `Projection(unit, origin)`
while preserving the same projection basis. Stroke widths remain in canvas
pixels and must not scale with the model.

| Palette key | Color | Meaning |
|---|---|---|
| `ink` | `#263442` | Outlines and vertices |
| `top` | `#DCE3E9` | Top-facing surface |
| `left` | `#AAB4C0` | Left-facing surface; Rotate guide |
| `right` | `#8795A5` | Right-facing surface |
| `selected` | `#FFD470` | Selected (same yellow-orange as the viewport selection; Jason 2026-11-01) |
| `new` | `#56B4D3` | New geometry or operation edge |
| `x` | `#D85C58` | X motion |
| `y` | `#4EAA73` | Y motion |
| `z` | `#4C8CD6` | Z motion |

Use `EDGE = 0.96`, `ACCENT = 1.12`, and `DETAIL = 0.72` from
`conventions.py`; round caps and joins are required. Preserve at least a
half-pixel canvas margin including strokes. Curved paths also undergo a
conservative control-point bounds check.

Use palette keys with the shared primitives:

| Method | Coordinate space and behavior |
|---|---|
| `mesh(projection, faces, width=EDGE)` | Model-space face/color pairs; fills in supplied order, then deduplicates identical edges |
| `highlight_face(projection, points, color="selected")` | Model-space polygon; colored fill and outline |
| `highlight_edge(projection, points)` | Model-space polyline; cyan accent stroke |
| `face(points, color)` | Canvas-space polygon fill only |
| `edge(points, color="ink", width=EDGE, closed=False)` | Canvas-space polyline or closed boundary |
| `vertex(point, radius=0.7, color="ink")` | Canvas-space filled circular vertex |
| `arrow(start, end, color)` | Canvas-space shaft with a shared open arrowhead |
| `cubic_arrow(points, color)` | Four canvas-space cubic control points with a tangent-based arrowhead |
| `guide(center, radius, width)` | Canvas-space circular guide |

`mesh()` does not perform hidden-surface removal, clipping, triangulation, or
automatic face sorting. Supply only the visible faces in the correct drawing
order. Its edge deduplication matches identical model endpoints; it does not
merge partially overlapping segments. Avoid passing an already projected point
to a method that expects model coordinates.

The production validator accepts polygons, polylines, circles, and paths
containing a single explicit `M`/`C` cubic. It rejects groups/transforms, CSS
styles, opacity attributes, external content and other element types. Build
geometry directly through primitives. If a requested design truly requires a
new primitive, implement it centrally and update meaningful validation/tests
for the new capability; do not bypass the checks to make an icon pass.

## Add an icon or create another set

1. Define one main visual cue for each requested tool. Keep detail readable at
   24 px; use the existing six icons as the style baseline.
2. Add a function accepting `Canvas` in `icons.py`. Describe model geometry
   through shared primitives rather than duplicating SVG path strings.
3. Register a unique lowercase, hyphenated slug in `ICONS`, with the function,
   display name, and accessible description. Preserve the existing six unless
   the user asked to replace them.
4. Adapt reference handling and preview layout as described below before
   expecting a full generation run to succeed.
5. Update the expected icon set and add tests for the new operation's cue.
   Preserve the existing regression checks and verify existing icons still
   generate. Run full generation, tests, and visual review.

For example, the following recipe could be placed in `icons.py` to demonstrate
a selected right face. It uses names already available in that module. This
is an example, not an instruction to add an unsolicited icon. The registry
assignment must appear after the existing `ICONS` dictionary is defined:

```python
def select_face(canvas: Canvas):
    projection = Projection()
    cube(canvas, projection)
    canvas.highlight_face(projection, RIGHT)


ICONS["select-face"] = (
    select_face,
    "Select Face",
    "Orange selected face on the right side of an isometric cube.",
)
```

### Sets, references and previews

- `generate.py --only a,b,c` generates a subset; the default is every icon.
- Approved/generated comparison sheets are made only for icons that have an
  approved original in `reference/svg/` (the first six). New icons get the
  contact sheet and `review-light.png` / `review-dark.png`, which grow with
  the set. Never invent a reference or copy a generated icon into that role.
- Tests require the original six and the primitives batch to exist; every
  icon is checked individually against the palette and strokes. The original
  six together still must exercise every color and stroke width.

### Fill and gizmos

`drawing()` builds each icon through `fit_to_canvas`: it draws once to
measure, then again scaled and centered so the ink's longer side is `FILL`
(30 px of 32; primitives 26, see `FILLS` in icons.py). Stroke widths and
dot radii are not scaled. Transform gizmo
axes (Move, Rotate, Scale, Transform) use the bolder `GIZMO` width.

### Curved solids

`solids.py` draws convex solids in the cube's style: outline = convex hull of
the projected surface, a lit top region (cylinder cap) and the body split at
the front into the left and right tones (no ink line down the split). Spheres and
ellipsoids instead use nested tones lit from the upper left, with no inked
inner edges: an inked boundary on a round body reads as a flat lid. Use `solids.fitted(points)` to center a solid's
screen bounds on the canvas. Curves are sampled polygons (no SVG arcs).

## Validation and visual review

After code or artwork changes, run full generation and:

```sh
.venv/bin/python -m unittest discover -s tests -v
```

The initial six-icon implementation passed 16 tests. Treat that as the recorded
baseline, not proof that later changes passed. Report the actual current result.
For tooling-only work, these Python checks exercise the changed functionality;
follow repository instructions for any additional checks applicable to scope.

Open and inspect the actual rendered images. Verify:

- Every requested icon appears with its correct label and operation cue.
- Native 24 px and 32 px icons are legible on both light and dark backgrounds.
- Faces, arrows and vertices have enough space; no strokes, labels or columns
  are clipped or overlapping.
- Enlarged details use nearest-neighbor scaling, exposing real source pixels.
- Approved/generated labels are accurate; the reference art stays unchanged.
- Existing approved icons have no unintended visual regressions.

Passing structure/bounds tests does not establish visual recognizability.
Show previews to the user and obtain visual approval for new or revised artwork
before any separately authorized integration step.

## Deliver and stop at the requested boundary

Default output structure:

```text
generated/
  svg/<slug>.svg
  png/24/<slug>.png
  png/32/<slug>.png
  previews/contact-sheet.svg
  previews/contact-sheet.png
  previews/comparison-light.png
  previews/comparison-dark.png
```

Deliver the generator source, SVGs, contact sheet and size previews. Include
the generation/test result and a short explanation of intentional visual
changes or remaining limitations. Share runnable source with its requirements
and reference provenance, excluding `.venv`, caches and temporary files.

Do not claim an icon is integrated, committed, or published merely because
artwork was approved. Make those changes only when the user requests them.

## Suggested instruction to start another LLM

> Read the repository's AGENTS.md and tools/operandi-icons/LLM_GUIDE.md. Use the
> existing generator to create the icons I name in Operandi's approved visual
> style. Reuse shared projection, geometry, palette and stroke primitives.
> Handle missing references and preview layout honestly when adding icons.
> Generate SVGs and light/dark 24/32 px review sheets, run the relevant tests,
> and show the results. Do not change application code, integrate, commit or
> push. Stop for my visual approval.
