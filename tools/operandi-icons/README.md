# Operandi programmatic icons

> **Deprecated (Jason, 2026-11-20).** New icons are rendered with the icon
> studio in [`tools/icon-studio`](../icon-studio/README.md). Don't add icons
> here; this tool only keeps the existing SVGs, which the app uses as
> fallbacks and for icons not yet rendered.

Standalone artwork tooling. Jason approved the generated six-icon core set in
the originating chat and, on 2026-11-01, chose to redraw Operandi's icon set
with this tool in reviewed batches, with the viewport's yellow-orange
(`#FFD470`) meaning "selected" everywhere. Each batch needs Jason's visual
approval before the app uses it. See LLM_GUIDE.md for extending the set.

The app uses these icons: after an approved change, copy `generated/svg/*.svg`
into `crates/operandi-app/assets/icons`. The Rust test
`icons::generated_tests::app_icons_match_the_generator` fails if the two
differ, and the icon table in `crates/operandi-app/src/icons.rs` maps each
icon to its file.

For another LLM using or extending this tool, read [LLM_GUIDE.md](LLM_GUIDE.md).

## Acceptance criteria

1. Generate Extrude, Bevel, Inset, Loop Cut, Move, and Rotate from shared Python
   geometry, with no imported SVG paths or runtime dependency on reference art.
2. Each independent SVG has a transparent 32×32 canvas, accessible title and
   description, and only the approved nine artwork colors.
3. All meshes use the same 30° isometric basis and one unit cube. Optical sizes
   are explicit: normal 12.9, Extrude 11.61, Move 10.535, Rotate 6.8585 pixels/unit.
4. Strokes use fixed canvas widths: mesh 0.96, motion/new edge 1.12,
   miniature mesh/guide 0.72 px; round caps and joins throughout.
5. Preserve the approved faces, topology, colored axes and Rotate curves;
   normalize accidental line-weight differences without redesigning the symbols.
6. Produce a vector/PNG contact sheet and native 24/32 px light/dark sheets,
   including approved/generated pairs and nearest-neighbor 4× pixel detail.
7. Automated checks validate SVG structure, palette, strokes, canvas, drawing
   bounds including stroke, transparency, deterministic output, and previews.

## Run

Python 3.10+; CairoSVG also needs the system Cairo library (e.g. `brew install
cairo` on macOS). From this directory:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.lock.txt
.venv/bin/python generate.py
.venv/bin/python -m unittest discover -s tests -v
```

`generated/svg/` contains the six masters; `generated/png/{24,32}/` contains
transparent renders. `generated/previews/` contains the contact sheet and
light/dark comparisons. Use `--output DIR` for a different destination, or
`--svg-only` to generate masters with svgwrite alone. Generation never reads or
modifies application files. Run the two commands above after changes; the Rust
application checks do not exercise this standalone Python tool.

`requirements.txt` lists direct dependencies; `requirements.lock.txt` records
the complete tested dependency closure. Package licenses are recorded in
`docs/DEPENDENCIES.md` at the repository root. Dependencies are tooling only
and are not shipped inside the SVGs or added to the Rust application.

## Structure and extension

`operandi_icons/conventions.py` owns the palette, projection, sizes and widths.
`primitives.py` draws faces, meshes with deduplicated edges, vertices, highlights,
straight arrows and cubic arrows. `icons.py` describes operations in model
coordinates. `previews.py` renders review material; `validation.py` checks the
serialized SVG rather than trusting the generator's geometry.

To add an icon, write a function accepting `Canvas`, construct faces from unit
cube coordinates, and call `mesh`, `highlight_face`, `highlight_edge`, `arrow`,
or `vertex`. Register the function and description in `ICONS`. Add operation
semantics to the tests and update the expected set if extending the approved six.
Avoid literal SVG path strings; use `edge` or `cubic_arrow` to serialize points.
Use `Projection` for all 3D geometry. Only the Rotate guide and motion curves
are screen-space overlays, as in the approved reference.

## Reference and deliberate corrections

`reference/svg/` is an unchanged review-only copy of Jason's approved original
art from the 2026-11-01 `operandi-core-icons` handoff. The original contact sheet
and light/dark size previews were inspected there. `reference/README.md` records
the source; production generation does not read it. Comparisons render both
versions with the same renderer, size and background.

The reference SVGs use a 64×64 viewBox and per-icon scaling that also scales
strokes. Geometry is now emitted directly in 32×32 coordinates, without nested
transforms. The optical fits and original palette remain; mesh, highlight and
detail strokes are centralized in canvas units. Move's axes now follow the
projection exactly (the original direction rounded to six decimals). Rotate
retains its three approved cubic silhouettes and smaller cube, with shared
tangent-based arrowheads. Preserve small-size readability when extending the
set, particularly around miniature geometry such as the Rotate center cube.
