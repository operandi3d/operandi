# Dictionary icon integration — 2026-10-21

ICON-DICT-001 implements the user's request to replace current application
icons with the admitted artwork dictionary. This is implementation from a
reviewed behavioral specification with documented separation. Only admitted
handoffs, current Rust sources, and public dependency documentation were used.
No reference application, old implementation, or private evidence was inspected.

## Mapping and acceptance

| Current drawing / controls | Dictionary lookup → shared drawing | Code |
|---|---|---|
| Cube tool and existing scene mesh glyph | Box → prim.cube | toolbox.rs, panels.rs |
| Move | TransformMove → TransformMove | toolbox.rs |
| Rotate | TransformRotate → TransformRotate (corrected dimensional rings) | toolbox.rs |
| Scale | TransformScale → TransformScale | toolbox.rs |
| Bevel | poly.bevel → poly.bevel | toolbox.rs |
| Extrude | poly.extrude → poly.extrude | toolbox.rs |
| Visible/hidden mesh eye | {uiicon} eyeGrey → same drawing | theme.rs, panels.rs |
| Toolbox, Scene, Item Transform, Work Plane, Supported Scope disclosures | tab.collapse → vp.collapse | theme.rs, panels.rs |

Eight exact SVG resources live in `crates/operandi-app/assets/icons/`, with one
copy per drawing. `manifest.json` retains lookup names, source names/aliases,
fidelity differences, production review status and original provenance verbatim.
The sync script reads only the admitted HTML's `icon-dictionary-data` JSON.
The dictionary's preview palette and 35% disabled-opacity approximation are not
substituted for approved application control states.

`icons.rs` uses resvg 0.48.1, without default features, to preserve paths,
front/back overlaps, gradients, shading, colors and alpha. Embedded resources
need no runtime files or server. Drawable pixel size is logical art size times
current egui display scale, rounded up; the context-owned cache has at most 64
textures, each at most 256 pixels per axis. Textures regenerate when scale changes.
The six existing tool buttons retain 24-unit/20-unit artwork, original hit areas,
active/hover/pressed/focus chrome, accessible names, shared operations and the
same editable shortcut labels/tooltips. Scene glyph and eye stay 20 units.
Disclosures use the existing allocated icon areas (12 units for custom headers);
the same down triangle rotates to point right when closed. The Scene header's
old +/- text indicator now uses this shared SVG. Hidden eyes keep their existing
slash as a separate control-state overlay, rather than treating another eye's
name as a hidden-state specification.

Viewport handles, construction-plane badge, grid and geometry feedback remain
functional projected content. Checkbox checks remain shared boolean state
marks required by M11-016. Text-only selection/history/navigation/actions and OS
window buttons receive no new artwork or changed behavior. No unavailable
commands or combined Transform tool were added.

## Validation

- `python3 scripts/sync-icon-dictionary.py --check`: all eight SVG resources and
  metadata match the dictionary exactly.
- `cargo fmt --all --check`: pass.
- `cargo clippy --locked --workspace --all-targets --features qa -- -D warnings`:
  pass. Existing transitive `block 0.1.6` future-compatibility warning remains.
- `cargo test --locked -p operandi-app`: 25 passed, including all eight drawings
  rasterized at logical 12/16/20/24/32 and 1×/1.5×/2×; nonempty and transparent
  pixels verified. Existing input, shared-control, layout/pick, focus/history,
  palette and save protection tests pass.
- `cargo test --locked --workspace`: app 25 and core unit 3 pass; LXO 3 pass,
  **2 fail** (`modo16_corpus`, `modo17_corpus`), the previously documented
  unimplemented LXO-Q001 decoding profiles. No importer support claim/change.
  The workflow target is run separately because Cargo stops at that failure.
- `cargo test --locked -p operandi-core --test workflow`: 19 pass, including
  original vertical slice, topology/corner attributes, cancel/history/save/reopen.
- `cargo build --locked -p operandi-app --features qa` and normal bundle build:
  pass. `scripts/qa-icons.py` runs 16 offscreen egui/wgpu profiles: 1080p/1440p/
  4K/5K at 1×/1.5×/2×, additional Polygon category scales and 1100×720 minimum.
  Render/layout/picking checks pass; below-minimum logical areas are explicitly
  stress profiles. Controlled captures and JSON remain in ignored
  `development/icon-dictionary/qa/`. Reviewed 1440×900 Polygon at 1× and
  3840×2160 Create at 1.5×: SVGs present, transparent, distinct, unclipped.
- Actual macOS native 1440×900 window on Apple M1 Max/Metal at observed 1×:
  Cube/Rotate/Scale/Bevel/Extrude activate via existing controls; cancel returns
  to idle without geometry changes. Create/Polygon switching preserves scene.
  Native screenshot confirms integrated tool, eye, mesh and disclosure artwork.
  Further native checks stopped when external user input changed the live
  Extrude preview; that work was preserved. Native hidden-eye/disclosure checks
  are therefore not claimed as rerun. Existing automated control tests pass.

No physical 1.5×/2×/mixed-monitor or Windows/Linux hardware pass is claimed.
Dictionary `visual-review-required` and approximation labels are unchanged;
this verifies local integration, not final visual parity or legal clearance.

## Development build

Built and signature-verified separately with:

```sh
OPERANDI_DEV_NAME='Operandi Rust M11 Icons' \
OPERANDI_DEV_ID=org.operandi.rust.m11.icons scripts/bundle-dev.sh
```

Bundle: `~/Library/Caches/Operandi-Rust/Builds/Operandi Rust M11 Icons.app`
(repository link: `development/Operandi Rust M11 Icons.app`). Native QA used
`OPERANDI_DEV_DATA=$PWD/development/icon-dictionary/native-data` and the original
`fixtures/VS_Box_Run01.opscene` as startup input (unchanged SHA-256 verified). User scenes and prior
installation/builds were not overwritten. The new app remains open; earlier
running builds retain earlier artwork. No commit, push or publication performed.


## Parity-audit alignment update — 2026-10-21

Alignment delivery extends the integration to14 exact shared drawings: Transform and selType.vertex/edge/polygon join the existing ten. Four equal transform cells preserve primitive sizing. Dictionary manifest/raster tests include these assets; approximation/visual-review-required labels remain. See [PARITY_AUDIT_FIXES.md](archive/PARITY_AUDIT_FIXES.md).
