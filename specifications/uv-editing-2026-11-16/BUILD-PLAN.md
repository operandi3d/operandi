# UV build plan, staged (2026-11-16)

Goal for stage 1 to 3: a beginner can follow the Cartoon Kid course,
video 5 (iris, clothes, belt, laces, shoes), and Whiskey Glass video 8
(rotate the table's UVs 90°) start to finish. Those steps are K71 to K80
and G43 in `tutorial-gaps-2026-11-13-GAP-LIST.md`. Each stage is one build
thread, merged on Jason's "merge it".

Specs: **UVS-1** = `uv-unwrap-pack-relax-2026-11-11` v1.1, **UVE-1** =
`tutorial-gaps-2026-11-13/SPEC-UV-EDITING.md`, **UVT** =
`BEHAVIOR-UV-TOOLS.md` (this folder). Engines E1..E15 are from
`UV-TOOL-FAMILIES.md`.

Already built: the UV engine crate, Unwrap and Pack as dialogs, the UV
view (stage 1).

---

## Stage 1: UV maps and moving UVs (unblocks most of Kid 5)

| Work | Spec | Engine | Tutorial step |
|---|---|---|---|
| UV map list: new, select, rename, delete several, copy and paste a whole map | UVE-1 §1 | E1 | K71, K73, K77 |
| Move, Rotate, Scale, Transform in the UV view through the existing pipe (action centre, falloff, snapping), Tear Off | UVE-1 §2, UVT §6.1 | E6 | K74, K80, G43 |
| Fit UVs (Entire, per Island, per Polygon; Shift and Alt clicks) | UVE-1 §3, UVT §7.1 | E7 (first rows) | K71, K80 |
| Mirror and Flip | UVE-1 §5, UVT §7.2–7.3 | E7 | K80 |
| Axis Align Up, Down, Left, Right (with Alt per edge row) | UVE-1 §6, UVT §7.4 | E7 | K79 |
| Paint and lasso select in the UV view | UVS-1 §4 | E14 | K73 |

Build E7 as the engine with its grouping and rule table here, not as five
separate commands. Later stages only add rows.

## Stage 2: Project, and Unwrap as a live tool

| Work | Spec | Engine | Tutorial step |
|---|---|---|---|
| Project from View | UVE-1 §4, UVT §2.6 | E2 (view frame) | K71 |
| Project: Planar, Cylindrical, Spherical, Cubic, Barycentric | UVT §2 | E2 | (common in other tutorials) |
| UV Unwrap as the interactive tool (live fields, drag for Iterations), replacing the dialog | UVS-1 §1 | E3 | K74 |
| UV Relax tool, with pins and falloff | UVS-1 §3, UVT §4 | E4 | (tidy-up after Unwrap) |
| Unwrap & Relax as Unwrap then Relax | UVT §3 | E3 + E4 | |
| Checker toggle (needs Jason's OK, UVT-Q08) | UVT §14 | E14 | (checking stretch) |

After stage 2, Kid 5's UV steps all work except texture painting (out of
scope).

## Stage 3: the rest of the everyday kit

| Work | Spec | Engine |
|---|---|---|
| Split, Sew, Move and Sew | UVT §9.1–9.3 | E9 |
| Quick Align, Island Align, Orient Pieces, To Origin | UVT §7.5–7.8 | E7 rows |
| Select Border, Select Error UVs, Select Unmapped | UVT §11 | E11 |
| Cut, Copy, Paste, Delete, Set Value, Rotate Polygons, Merge UV Maps | UVT §12 | E12 |
| Pack: Nearest, Polygon Tag, By Item, Background Constraint, Pack to New Map | UVS-1 §2 | E8 |
| Coverage readout, Flipped tint, Show Inactive UVs, backdrop image | UVS-1 §4, UVT §14 | E14 |

## Stage 4: seams, symmetry, density

| Work | Spec | Engine |
|---|---|---|
| Seam maps and the Cut Map popover | UVS-1 §1.6, UVT §13.1 | E13 |
| Conditional Loop selection tool, with Seam Marker as its preset | UVT §13.2 | selection engine |
| UV Symmetry layer and Symmetrize; Unwrap's Symmetrize option | UVT §10, UVS-1 §1 | E10 |
| Texel Density panel and Real World Size | UVT §7.9 | E7 rows |
| UDIM Indicator tool | UVT §14 | E14 + E7 |

## Stage 5: new maths

| Work | Spec | Engine |
|---|---|---|
| Atlas and Atlas 2 projections: port xatlas's chart splitting to Rust (MIT notice kept), then flatten and pack with the existing crate | UVT §2.4, ENGINES.md | E2 |
| Peeler and Rectangle | UVT §5 | E5 |
| Radial Align and Linear Align (build once; works in 3D and UV) | UVT §6.3 | E6 |
| Split and Sew brush, Box Transform | UVT §9.4, §6.2 | E9, E6 |

## Stage 6: low value

Export EPS and SVG, Convert UVs to Mesh, Transfer UV positions, procedural
(mesh operation) versions of UV tools.

---

## Before the builds

- One short bridge pass (when the bridge is free, with Jason's in-thread
  "go") answers UVT-Q01, Q02, Q04, Q06, Q07, Q09, Q10 and UVE-1 Q1, Q4,
  Q5, Q6, plus the old UVS-1 questions Q3 and Q5. Stage 1 can start
  without it: all its guesses are marked and easy to change.
- Jason: UVT-Q08 (checker overlay as an Operandi extra).
