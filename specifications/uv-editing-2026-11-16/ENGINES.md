# UV engines we could borrow from (2026-11-16)

**Rule (Jason, 11-16):** "use [xatlas] for what it's best at. Blender for
the math behind the rest, and Modo workflow for the workflow." Borrow only
maths, and only from sources that are cleanly GPL-3.0-or-later compatible.
Tool names, options, defaults and feel come from the behaviour specs, never
from a borrowed project's UI.

Licence facts below are from each project's published licence as I know
it. **Check the licence header of every file before copying it**, as the
`operandi-uv` port already does.

## Ranking

| Rank | Engine | Licence | GPL-3 fit | Quality | Drops into Rust | Use it for |
|---|---|---|---|---|---|---|
| 1 | **Blender UV code** (parametrizer: LSCM, ABF++, minimum stretch / SLIM; shape packer; box pack) | GPL-2.0-or-later. Its SLIM solver files are MPL-2.0 | Yes (GPL-2-or-later can be used under GPL-3; MPL-2.0 is GPL-compatible) | Production grade, used daily by millions | **Already done**: `operandi-uv`, about 7,200 lines of Rust, tested | Unwrap, Relax solvers, Pack, Orient's box turn |
| 2 | **xatlas** (charting, its own parametrize and pack) | MIT | Yes | Best open-source automatic chart splitting. Its packer is weaker than Blender's | One C++ file of about 9,000 lines. The only Rust wrapper (`xatlas-rs` 0.1.3) dates from 2019 and is stale | **Atlas and Atlas 2 charting only.** Port the chart-splitting part to Rust (recommended, about a week), or wrap the C++ file through a tiny C layer (a day or two, but adds a C++ build step on three platforms) |
| 3 | **Boundary First Flattening** (Sawhney and Crane) | MIT, as published (check headers) | Yes | Excellent: fast conformal flattening that can also force an island's outline to a chosen shape (rectangle, circle) | Moderate: the maths is a few linear solves, which `faer` (already a dependency) can do. Their code leans on SuiteSparse, which we would not take | Later, optional: a better Relax "Straighten" border and Rectangle on islands that aren't clean quad patches. Port from the paper and the MIT code |
| 4 | **libigl** (LSCM, ARAP, SLIM, harmonic) | MPL-2.0 | Yes | Research grade, solid. Blender's SLIM came from here | Header-only C++ on Eigen, no Rust bindings worth using | ARAP (as-rigid-as-possible) as a Relax mode if Adaptive needs it. It's short enough to write from the paper with `faer` |
| 5 | **OpenNL** (sparse least-squares solver, Bruno Lévy; part of Geogram) | BSD-3-Clause | Yes | Good, and was Blender's old LSCM solver | Easy, but pointless: `faer` already does this job in pure Rust | Nothing. Geogram's other UV code (ABF++, charts, packing) overlaps what we have |
| 6 | **UVAtlas** (Microsoft, isochart) | MIT | Yes | High quality charts and packing, game-industry proven | Hard: large C++ code base built round DirectXMath, Windows-first | Nothing. xatlas covers the same job far more simply |

## Notes for the build

- **Notices.** Each borrowed file keeps its original licence notice and
  gets a row in `docs/DEPENDENCIES.md`, as for the Blender port. An
  xatlas port would be MIT-licensed files inside a GPL-3 program, which is
  fine. Keep the MIT notice in the ported files and in DEPENDENCIES.
- **Inbound rule.** The `operandi-uv` README says contributions to ported
  files come in under that file's licence. An xatlas port follows the same
  rule.
- **No feel from donors.** Blender's UV editor and xatlas's sample viewer
  are never a reference for how a tool behaves. The behaviour specs are.
- **What's not worth borrowing.** A full UV editor from anywhere (Blender's
  is GPL and fine licence-wise, but it's Blender's workflow, not the
  reference's). Rectangle packers like rectpack2D (MIT): the Blender box
  packer already does that.

## What each engine in UV-TOOL-FAMILIES.md uses

| Engine | Maths source |
|---|---|
| E2 Project: Planar, Cylindrical, Spherical, Cubic, Barycentric, View | Our own (simple projections) |
| E2 Project: Atlas, Atlas 2 | xatlas charting (port), then the Blender flatten and pack already in `operandi-uv` |
| E3 Unwrap | Blender (built) |
| E4 Relax | Blender minimum stretch and solvers (built). Spring is our own. Adaptive: SLIM with area weight (built), with ARAP as a later option |
| E5 Peeler, Rectangle | Our own grid walk. BFF later for islands that aren't clean quad patches |
| E8 Pack | Blender (built) |
| E11 Overlap tests | Reuse the packer's triangle tests (built) |
| Everything else | Our own (plain 2D moves, flips and welds) |
