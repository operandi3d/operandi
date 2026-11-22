# Milestone 1.6 bounded primitive implementation

2026-10-22 follow-up: approved equal-radius Ctrl Sphere drawing, temporary Ctrl
side movement and PM-UP01..06 canonical unit insertion supersede the earlier
unavailable-modifier/unit statements at their bounded scope. See
[PRIMITIVE_MODIFIERS.md](archive/PRIMITIVE_MODIFIERS.md); other unresolved contexts,
reference questions and native/physical limitations remain.

Subsequent alignment delivery: the user explicitly approved provisional explicit
Sphere/Cylinder side handles and combined Transform in the parity-audit session.
The earlier numeric-only and continuation-refusal limits below are historical
at that affected scope. See [PARITY_AUDIT_FIXES.md](archive/PARITY_AUDIT_FIXES.md) for the
current authority, implementation, native mouse results and still-open reference
calibration. Y-only Face/UV-off and other unamended limits remain in force.

Started 2026-10-21 by direct user instruction. In progress; no complete milestone
or reference parity claim. Reviewed behavioral specification with documented
investigator/implementer separation. Directory-name boundary check found no
private archive; no prohibited material, reference runtime or agents accessed.

## Acceptance defined before implementation

- Face Globe and two-Single-Polygon-cap Cylinder, independent Position/Radius,
  Sides/Segments; Cylinder axial radius is half height. Replace one preview
  against the activation scene; preserve all unrelated geometry, corner maps,
  labels, transforms, identities and selections.
- Compare R16-SP-01/R17-SP-01 and clean R16-CY-01 numerical summaries in
  reference-oracles.json: counts/arities, closed connected genus-zero topology,
  opposing edge directions, outward winding, layers, bounds and ellipsoid/ring
  equations. R17-CY-01 is a post-continuation sample, never the normalized baseline.
- Atomic invalid input/resource refusal; Drop one creation checkpoint; Cancel
  restores; inactive history; undo/redo and committed native save/reopen; failed
  or canceled save preserves live work. Existing corner attributes must survive.
- Shared numeric focus/expression/unit controls, command/menu/button routing,
  ordered shape-specific panels and exact admitted icon resources; unavailable
  options honestly unavailable. Test actual GUI separately from simulations.
- No Quadball/Tesselated, alternative caps, subdivision, Bulge, UV compensation,
  advanced contexts, transformed/multiple/hidden targets or later milestones.

## Specification questions and concrete proposals

These are missing reference behaviors, not inferred Modo observations. The user
explicitly approved the first four bounded policies below in this chat. Axis
switching and exact handles remain unresolved and unavailable. This approval
does not approve all M16-D0–D4 or turn sample values into reference defaults.

| IDs | Initial state/actions and missing behavior | Concrete proposed resolution |
|---|---|---|
| SP-Q01/Q04; CYL-Q01/Q03 | New live tool on aligned plane; asymmetric first drag, later off-handle click/haul. Exact anchor/extents, flat topology, depth/side selector unresolved. | Opposite-endpoint in-plane bounds, zero normal radius, temporary filled ellipse display without committed flat topology; positive numeric radii produce solid. Refuse off-handle continuation with feedback until specified. Do not reuse BOX-Q02. |
| SP-Q08; CYL-Q11 | Activation before any geometry; conflict between prose/sample/retained settings. | Clearly labeled initial Operandi sample settings: Sphere .5/.5/.5,24/12; Cylinder .5/.75/.5,24/1. No factory-default claim. |
| SP-Q13; CYL-Q16/Q17 | UV-on oracle versus no map ownership/atlas specification; existing named seam map. | Keep generation unavailable pending clarification; preserve existing corners and append absent corners in existing maps. Compare only admitted geometry summaries; UV-on fixture parity remains blocked. |
| SP-Q06/Q12; CYL-Q10/Q21 | Drop, Cancel, active Apply, history and save during live numeric preview. | Preserve existing approved shared Drop/Cancel/save/history policies, no repeated active Apply. Exact reference lifecycle unresolved. |
| SP-Q09/Q10; CYL-Q02/Q12 | Axis-Y explicit fixture, different counts and Axis switching. Phase, count minima and asymmetric axis reassignment not observed. | Implement independent analytic Face mesh construction; validate measured invariants, do not claim exact phase/order/general reference algorithm. Axis switching remains unavailable pending observable mapping. Technical allocation/degeneracy refusals are integrity guards, not reference limits. |

Detailed results, traceability and development bundle will be appended after
implementation and actual validation. M1.5 and LXO acceptance remain separate.

Native persistence regression found one-bit coordinate changes for trigonometric
f64 values with the existing JSON reader. Enabled serde_json's `float_roundtrip`
feature on the existing pinned dependency; no new crate/version, schema or
license choice. Exact save/reopen is required, including generated coordinates.
Importer and existing persistence regressions must rerun after this change.

## Implemented scope and traceability

Delivered bounded numeric creation, **not complete M1.6/reference acceptance**.
Both solids use Axis Y; independent world-XYZ radii do not rotate/reassign on
Axis changes because that operation is unavailable. Aligned XY/XZ/YZ draft
planes are supported independently of this solid-orientation limit. No exact
reference angular phase, record order or general UV algorithm is claimed.

| Requirements | Original implementation | Actual checks |
|---|---|---|
| SP-P02/M01; CYL-P02/P03/G01/G02 | core `radial.rs`: explicit semiaxes, shared rings/poles, outward Face loops, two single-polygon Cylinder caps | `primitives_m16.rs::m16_reference_geometry_oracles_connectivity_winding_layers_and_surface` reads all four numerical summaries; directed edge closure, connectedness, Euler, winding, layers, bounds and ellipsoid/radial equations pass |
| PRIM-CTX01/DATA01; SP-I01..03; CYL-I01..03 | core `editor.rs`: existing activation/base replacement, append service, Drop/Cancel/history/staged atomic save; `Parameters.radial` is live-only | Replacement, preservation of seam-bearing original geometry/labels/sparse maps/ordered selections/transforms, invalid values/counts/budgets, failed save, cancel, one checkpoint, exact undo/redo/native roundtrip pass |
| SP-A01/A02/L01/L06; CYL-A01/A02/L01/L05; PRIM-IN01..03 | app `primitive_input.rs`, `input.rs`, `main.rs`, `toolbox.rs`, menu in `panels.rs` | Buttons/command activation, no default shortcut, captured all-three-plane drawing and stable release, continuation refusal, field focus, Drop/history pass in simulation; native ordinary drawing/Q/button routes exercised |
| SP-P01/P03; CYL-P01/P04; PRIM-UI01/02/05 | app `radial_panel.rs` and existing shared number/triple/confirmation controls | Independent unit/expression edits, fixed-opposite numeric bounds, integer validation, field errors/Escape, ordered rows, same-frame dependent-buffer refresh and scrolling verified |
| Original artwork / PRIM-UI04/DPI01 | exact dictionary `prim.sphere`/`prim.cylinder` SVG resources via `icons.rs` and `sync-icon-dictionary.py` | All ten integrated drawings match dictionary exactly; rasterization sizes/scales pass; native 24-logical-unit primitive icons inspected |
| PRIM-DPI01; SP-T30/CYL-T30 bounded | app simulations and `qa.rs`; `scripts/qa-primitives.py` | Logical layout/input sizes through 5K profiles and twelve actual offscreen Metal framebuffers, separate from native/physical checks below |

The numerical-oracle test treats R17-CY-01 with its **derived post-click**
center/radius; only R16-CY-01 is the clean Cylinder baseline. UV bounds, UV
record counts and discontinuity counts from these UV-on samples are **not
passed**: generation remains unavailable by explicit user approval. Existing
corner values are preserved exactly and newly appended corners in existing
named maps are absent; generated faces are ordinary Face, with no fabricated
subdivision/material/part assignments. Unknown map ownership is not guessed.

Technical solid guards require finite positive radii greater than 10 nm,
radius <=1e8 m and resulting coordinates <=1e9 m; at least 3 Sides,
2 Globe Segments / 1 Cylinder Segment; product of Sides*Segments <=249998;
Cylinder caps <=4096 corners. These are current computation/integrity limits,
not Modo restrictions. One zero radius is an approved display-only flat draft;
all-zero/two-zero, negative, crossed and over-budget inputs refuse atomically.
Drop discards a flat draft without a geometry checkpoint and says so. Saving
a flat draft refuses explicitly, preserving it until positive radii or Cancel.

## Validation results

| Exact command/check | Result |
|---|---|
| `cargo fmt --all --check` | Pass after final source changes |
| `cargo test --locked --workspace` | Pass: **101 tests**: 42 app,20 core unit,5 LXO integration,3 M1.6 portable,12 M1.5 portable,19 workflow; zero failures. `development/milestone-1.6/workspace-test.log` |
| `cargo clippy --locked -p operandi-app --all-targets --all-features --no-deps -- -D warnings` | Pass, `app-clippy.log` |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | Fails on the same two preexisting LXO `match_like_matches_macro` diagnostics (`lxo/metadata.rs:20,47`) documented by M1.5. Importer source not edited for this task; strict whole-workspace lint is not claimed |
| Same workspace clippy with `-D warnings -A clippy::match_like_matches_macro` | Pass; this is an explicitly qualified diagnostic check, not strict-lint acceptance |
| `python3 scripts/sync-icon-dictionary.py --check` | Pass: ten exact shared drawings |
| `scripts/bundle-dev.sh` with M16 environment below | Pass: normal runnable binary, ad-hoc signing and strict signature verification; separate output, QA feature excluded |
| `cargo build --locked -p operandi-app --features qa` then `python3 scripts/qa-primitives.py --binary ~/Documents/Operandi-Rust/target/debug/operandi-app` | Pass: twelve offscreen Metal framebuffers in `renders/matrix.json`, with PNGs/metadata/logs |

The current upstream `block 0.1.6` future-compatibility warning remains; no
dependencies were installed or versions upgraded. Final normal bundle and QA
binary have different hashes; QA was not packaged as the normal app.

Native final-build checks used actual coordinate-focused text input (AX-only
field actions were ineffective and are not credited):

- Sphere initial Top/XZ draw showed a filled ellipse and RadiusY=0 with no
  committed vertices. Entered Position(1,2,-3), Radius(1,1.5,2),8/4 using
  units/expression `3 m/2`: live result26/56/32, nominal bounds
  (0,.5,-5)..(2,3.5,-1). Negative RadiusX reported an error and retained geometry;
  local Escape restored confirmed text. Q Drop, Undo to empty, Redo all observed.
- Saved `Sphere_Native.opscene`. Cylinder flat drawing beside it preserved the
  Sphere. Position(-2,2,-3), Radius(1,1.5,2),8/3 produced combined58/112/58.
  Failed Save As into a nonexistent parent preserved live Cylinder, filename
  and source; canceling the dialog retained it. Scrolling reached Min/Max,
  disabled Apply, Drop and Cancel. Cancel restored the saved26/56/32 Sphere.
- A fresh numeric Cylinder was dropped as one transaction. Undo returned to
  Sphere26/56/32; Redo restored58/112/58. Saved separately as
  `Sphere_Cylinder_Native.opscene` and reopened through command entry. Exact
  JSON data checks confirmed preceding Sphere vertices/faces and item transform
  unchanged. Tool state/history are intentionally inactive after reopening.
- Native screenshots inspected original icons, active state, ordered controls,
  disabled options, filled drafts, solid geometry and the reopened scene.
  Exact panel optical/measurement parity is not claimed.

Simulated/offscreen profiles:1100x720@1x,1920x1080@1x,
2560x1440@1.5x,3840x2160@2x,5120x2880@2x for both solids, plus
1920x1080@1x drafts for both tools. A representative minimum Sphere and2x
Cylinder image were visually inspected. These are offscreen or egui simulations,
not actual physical-resolution tests. Native reported Metal/Apple M1 Max,
1.0 logical/native scale,4x MSAA,896x778 viewport/framebuffer. Physical
1080p/1440p/4K/5K,2x/mixed-DPI, Windows and Linux remain unvalidated.
Native border resizing was not credited after a controller external-state guard;
the saved scene was re-read and left untouched. Modifier/right/middle dragging
is not claimed tested natively, and these new-tool actions remain unavailable.

## Build, demo and remaining decisions

Normal app: `~/Library/Caches/Operandi-Rust/Builds/Operandi Rust M16 Dev.app`
(`org.operandi.rust.m16.dev`), linked from `development/Operandi Rust M16 Dev.app`.
It remains open on `development/milestone-1.6/Sphere_Cylinder_Native.opscene`.
Recovery/input-map data are isolated in `development/milestone-1.6/native-data/`.
`~/Applications/Operandi.app`, M1.5/importer builds and user scenes
were not replaced. No commit, push, publication, license selection or agent work.

```sh
OPERANDI_DEV_NAME='Operandi Rust M16 Dev' \
OPERANDI_DEV_ID=org.operandi.rust.m16.dev \
OPERANDI_DEV_DATA='~/Documents/Operandi-Rust/development/milestone-1.6/native-data' \
./scripts/bundle-dev.sh
```

Remaining questions from the table above are still reference gates. Specifically:

- SP-Q04 / CYL-Q05: start a positive asymmetric primitive, pick each visible
  side/corner/center/depth handle including projected overlaps and Ctrl changes.
  Need actual handle layout, hit precedence and dimensional laws. Concrete
  **pending** product proposal: explicitly hit side handles only, use documented
  fixed-center/opposite-bound formulas, refuse ambiguous overlaps and keep
  off-handle continuation unavailable. This is not implemented or approved.
- SP-Q09/Q10 / CYL-Q02/Q12: switch Y/X/Z on unequal radii at odd/even counts.
  Need semiaxis ownership/reassignment, pole/cap orientation and angular phase.
  Concrete pending resolution: admit explicit world-XYZ semiaxes and documented
  orientation rotations without reassigning their numeric values, or supply
  reviewed observable contrary examples. Current numeric solids remain Y-only.
- SP-Q05/Q06 / CYL-Q06..10: modifier drawing/transitions, repeat/clone, unit
  targeting/reset and exact inactive Apply/retention. Keep unavailable until
  reviewed examples or named policies are admitted; do not borrow Cube input.
- SP-Q13 / CYL-Q16/Q17: specify map naming/ownership and exact corner atlas
  values/seams across resolution/axis changes. Proposed next handoff: original
  per-corner oracle summaries plus selection/material/part inheritance examples;
  no private exports or reconstructed algorithms. UV-off difference remains
  explicitly approved for this slice.
- PRIM-UI03/04/05, SP-Q08 / CYL-Q11: exact logical measurements, mini-slider
  detents/hauling/link modes/presets, reset/remembered behavior and shared popover
  scope. Existing independent numeric controls are reused; full conventions,
  K popover/floating layout and reference default fidelity are not claimed.

No placeholder Quadball/Tesselated/Quad Grid/subdivision/Bulge/UV-compensation
algorithm or advanced context was added. The ordered disabled level/weight/UV
rows disclose availability; absent variants cannot mutate geometry. Workspace
layout requirements M16-UI-001..007 were later delivered by the parity pass
(PAR-004..010, see archive/PARITY_AUDIT_FIXES.md); their remaining items wait on
reviewed specs (numeric mini-sliders/links/presets, context popup contents,
multi-item selection) or physical display checks. M1.5 and importer release gates remain their own ledgers.
