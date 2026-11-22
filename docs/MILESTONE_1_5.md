# Milestone 1.5 progress ledger — 2026-10-21

Implementation from the reviewed behavioral specification with documented
separation. No reference application, manuals, investigation chats, REA,
private evidence, or prior Operandi source was consulted. Initial directory-name
inspection found the admitted handoff structure and implementation folders;
no private archive was indicated. No project license or publication authorized.

## Admission and sequence

Direct user request admits M15-D0/D3, established nonconflicting D1, ordinary
Cube and regular-quad selection under bounded D2. Requirement evidence labels
in the handoff are unchanged; implementation results below are separate.

| Requirements | Admission | Implementation / validation |
|---|---|---|
| KBD-AC-001 | Structured appendix inventory, 259 rows; not a flat keymap | Inventory read; parser acceptance pending |
| KBD-AC-002/003 | Literal modifiers, physical-key distinctions, resolve ambiguity individually | Editable literal chords; no Ctrl/Cmd rewrite. macOS native adapter retains main/keypad digits and decimal. Main 1/2/3/5 modes, keypad view/style defaults implemented; other OS adapters unvalidated |
| KBD-AC-004..006; SEL-012/090 | Context precedence and effective labels | Existing focus/Space handling retained; supported tool/selection chords added; full context matrix pending |
| SEL-017/018/030..034 | Remembered selection, ordered seeds, prehighlight, paint and history | Existing memory/hover retained; continuous sampled paint and approved per-gesture history implemented; focus cancellation/idempotence tests pass |
| SEL-020/021 | Explicit versus implied input | Existing component transform implied-all retained; region implied contour consumer pending |
| SEL-022/050..053/063 | Convert, All, Invert, Connected, Invert Connected, Boundary | Portable operations, Select menu and command entry implemented; original seam/island/conversion/history/reopen tests pass |
| SEL-040..044 | Lasso four shapes/facing/full-partial core; unresolved boundaries/Through/SubD not inferred | Four styles, full/partial enclosure, facing matrix, held-main-8 and persistent partial implemented; pure geometry and simulated facing/no-depth tests pass. Through/SubD/item lasso remain unsupported |
| SEL-054/055/058/060 | Grow/Shrink and regular-quad loop/ring | Vertex/edge/polygon loops, edge ring and Grow/Shrink implemented at named regular-quad scope; cube/grid topology tests pass; extraordinary closed poles, nonquads and ambiguous seeds reject atomically |
| SEL-056/057/061/062/064 | Follow-through where specified | Unequal two-region polygon Close Loop implemented and tested, ties refuse. More/Less, Between and Polygon Selection Fill implemented per user guide (see section below). Previous/Next Loop still unassigned |
| SEL-070..084 | Advanced inventory/panels | Deferred, not represented as completed core |
| BOX-A01/A02/L01/L09; W01..05 | Ordinary functional-plane live drawing | Live quad at zero normal depth; no release reprojection; base and preview transactions shared with Editor |
| BOX-H01..03/P02/I01..04 | Independent boundaries, translation, coherent numeric bounds, integrity | BoxBounds, six sides, arrows, modes and Min/Max added; regressions in progress |
| BOX-L10/Q02 | Off-handle behavior | Explicit user-approved product selector below; not Modo parity |
| BOX-H04/Q04 | Corner handle layout/constraints | Show Corner visibly unavailable pending clarification |
| BOX-A03..06/L02..05 | Modifier drawing/unit insertion/repeat/clone/segments | Right-drag segments implemented under provisional detent/mapping approval. Ctrl initial draw, Shift repeat, middle clone and unit insertion details explicitly deferred by user for focused supplement; gestures report unavailable |
| BOX-P01/P03 | Full property inventory/shared numeric conventions | Segments/Position/Size/Handle Mode/Min/Max operational. Rounding/UV inventory visibly unavailable with reasons. Corner layout and advanced numeric conventions remain gaps; full panel fidelity is not claimed |
| BOX-A07/Q15 | Apply | Active UI disabled. Apply Last Cube applies retained properties once with no live tool; original history test passes |
| BOX-Q11..14/T16/T17/T22 | Rounding/Bulge/exact atlas/advanced contexts | Deferred as requested; no support claim |

Sequence: shared selection operations → selection transaction/paint → shared
primitive bounds/insertion and flat live workflow → contextual mapping → topology
selection/lasso → approved primitive gesture follow-through → full regressions,
GPU and native QA → saved demonstration. This is not a completed milestone.

## Approved product decisions in this implementation chat

- BOX-Q02: after 1 logical pixel of off-handle movement choose the closest
  projected side-handle center at pointer-down. Ties X/Y/Z, minimum before maximum.
  Lock side for the gesture. Change one boundary in its projected axis coordinate;
  stationary click inert. Reject an axis projection shorter than 1 logical pixel
  per world unit. This is a bounded original rule, not verified reference behavior.
- BOX-Q01: one filled quad when exactly one dimension is zero; replace with
  eight vertices/six quads when all dimensions positive. Two zero dimensions
  generate no geometry. No coincident duplicate faces.
- BOX-Q03/Q15: reject crossed/negative bounds with feedback; retain existing
  Drop/Cancel/save/history policy; active Apply disabled; inactive Apply should
  use retained numeric properties once without arming.
- SEL-034: one undo checkpoint per completed click/paint/lasso gesture; ordered
  first encounters; focus loss restores starting selection; no geometry changes.

## Architecture and provenance

`selection.rs` owns pure portable selection commands; Editor owns their history.
`primitive.rs` owns Cube bounds/generation and a shared append service that
extends prior sparse corner maps with absent corners. Editor's existing Live
snapshot/preview/drop/cancel/save transactions remain common infrastructure.
Viewport input arbitrates navigation, handle, creation and selection ownership.
`primitive_input.rs` supplies Cube gesture recognition/interpretation through a
captured Primitive gesture, keeping shared movement/release routing independent
of Cube formulas. Future primitives
must supply parameters, generator and gestures, and reuse insertion/transactions,
not inherit Cube bounds or its mouse behavior. No additional primitive is added.

No artwork was changed. Existing admitted dictionary resources remain in use.
This checkout has no Git metadata; no commit/push was attempted.

## Validation so far

- Six new portable selection/bounds/flat-preview tests pass.
- All 19 existing modeling/persistence regression tests pass after updating the
  old zero-size rejection oracle to negative size (zero size now admitted).
- Prior positive-depth initial-drag GUI oracle updated to approved zero normal
  extent; all three construction planes pass simulated input checks.
- Full workspace run found failures in existing LXO producer corpus and one
  importer UI expectation; these paths are outside the current edits. Strict
  lint initially found two LXO diagnostics; a later rerun passed (workspace LXO
  files are changing independently during this session). Do not infer importer
  acceptance from either run.
- Native app controller initially timed out, then opened the first partial
  selection bundle with the default M11 recovery dialog. No recovery discarded.
  Current workflow/native QA and isolated final bundle/demo remain pending.

Exact final commands/results and remaining decisions will be appended here.

## Current review checkpoint — supersedes initial validation notes

Milestone **incomplete / reference decisions pending**. See the single
[M15_DECISIONS_NEEDED.md](M15_DECISIONS_NEEDED.md) list. No full Cube or selection
parity is claimed. Documentation support, investigator runtime support and Rust
results remain separate; the handoff files and their status labels were not edited.

### Delivered bounded workflows

- Editable literal chords with phase/domain/area eligibility, native main versus
  keypad numeric provenance, existing text/menu/modal/navigation ownership,
  Space drop-before-cycle, Q drop and Cube-specific Escape drop. Main mode keys
  1/2/3/5 come from SEL-011, not invented appendix rows. Main 8 temporarily adds
  partial lasso without changing the persistent toggle. Keypad digits/decimal
  route supported viewport views/styles only over the viewport. Existing native
  Save/Close mappings remain prior approved product behavior.
- Scene N is scoped to the scene area. Supported modeling defaults W/E/R/B/
  Shift+X, Frame A/Shift+A and selection defaults derive their labels/tooltips from
  the effective map. Absent tools, disputed Mac qualifiers and unsupported
  continuation actions remain unassigned. Mapping UI supports literal chords;
  it is not the complete reference Input Editor. New Mesh header sizing is
  measured from its effective label, preventing panel-width growth on each frame.
- Click/continuous paint, idempotent add/remove, clear, remembered component
  selections and approved per-gesture history. Pure Convert and Boundary remain
  distinct. Connected/Invert/Invert Connected/All preserve geometry and corner
  attributes. Region contour tool consumption beyond existing tools is not claimed.
- Grow/Shrink, directional two-polygon/vertex Loop, single-edge Loop, one-edge or
  same-strip two-edge Ring, and unequal two-region polygon Close Loop. Nonquads,
  disconnected vertex fans, extraordinary closed edge-loop poles, ambiguous
  seeds, equal/nonseparating Close Loop and whole-closed-mesh Shrink reject with
  feedback. Current regular-quad algorithms have a 4096-component safety budget;
  this is a development computation limit, not a reference topology limit.
- Freeform/Rectangle/Circle/Ellipse lasso with provisional full/partial enclosure,
  boundary rules and the specified shaded/wireframe right/middle facing matrix.
  A separate wireframe display makes that distinction functional. Facing-only
  inclusion deliberately includes an occluded front-facing face; it performs no
  depth visibility test. Curved outlines use 128 segments; complex enclosure work
  has an explicit 50-million-work budget and 4096-point path budget. Item lasso,
  Through precedence, evaluated SubD, hidden/locked/background/multi-item
  combinations are not represented as supported by the single active-mesh slice.
- Functional major-plane Cube drawing with a live zero-normal-extent quad/grid,
  stable release, six independent sides, translation arrows, Resize/Resize from
  Center/Move All, Ctrl-existing-side translation, numeric Position/full Size/
  Min/Max coherence, replacement previews, retained inactive Apply, and provisional
  right-haul segments. Flat generated live faces draw from either side without
  adding duplicate mesh faces. Existing geometry and sparse named/corner UVs
  survive preview replacement and cancellation.
- Generic captured primitive routing delegates Cube interpretation to
  primitive_input.rs; portable BoxBounds and shape generation remain separate
  from shared Editor snapshots, target checks, append, history and save services.
  No other primitive is implemented. No universal Cube bounds/gesture model is
  imposed on future shapes. A future primitive supplies its provider/generator
  while retaining shared movement/release and transaction ownership.

### Actual validation

| Command / check | Result |
|---|---|
| `cargo fmt --all --check` | Pass after final source changes |
| `cargo test --locked --workspace` | Pass: 94 tests (38 app, 20 core unit, 5 LXO integration, 12 M15 portable, 19 modeling/persistence); zero failures. Log: development/milestone-1.5/final-workspace-test.log |
| `cargo clippy --locked -p operandi-app --all-targets --no-deps -- -D warnings` | Pass; final-app-clippy.log |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Last run failed on two independently changing LXO metadata `match_like_matches_macro` diagnostics at metadata.rs:20 and :47. Those source paths were not edited in this work; final-clippy.log records the result. Earlier session runs passed before that independent metadata addition. Workspace strict-lint acceptance is therefore not claimed. |
| `./scripts/bundle-dev.sh` with M15 Review name/id/data | Pass: normal runnable app compiled, ad-hoc signed and signature verified; QA feature excluded |
| Native Metal ordinary Cube | Pass observed: flat filled quad, numeric depth to 8/12/6, independent +X side with Min X fixed and Y/Z unchanged, one-pixel off-handle change affecting only Max Y, Q drop, save/open |
| Native selection | Pass observed: Boundary gave four contour edges with 8/12/6 geometry unchanged; Grow/Shrink 1→5→1; main 1 entered Vertex; keypad 2 did not enter Edge and Front↔Back toggled; selection Undo restored saved state |
| Simulated input/layout | Pass: per-stroke history/focus cancellation, modifier ownership, four-case lasso facing, occluded front-facing inclusion, provisional segment replacement, zero-depth before release, existing focus/menu/save/history regressions, and Cube side picking at 1100×720 / 1440×900 / 2560×1440 logical sizes with 1×/2× combinations |
| Physical hardware/native modified drags | Not fully run. CUA's documented drag API supplies primary drag; no fabricated right/middle/modifier drag events were injected into native apps. Right/middle/segment/facing variants above use egui simulation, not claims of native execution. |
| Physical display scale | Actual review reports Metal / Apple M1 Max, 1.0 native/logical scale, 4× MSAA; observed resized viewport 1212×1054 logical/framebuffer. 2× is simulation; physical mixed-DPI, 1080p/1440p/4K/5K display matrix, Windows and Linux remain unvalidated. |

The final full suite includes both LXO producer regressions passing. Initial LXO
failures and changing diagnostics were observed while unrelated LXO work was
proceeding in this workspace. No importer implementation or fixture was changed
by this milestone work; passing that suite is not an assertion that all LXO
release/GUI acceptance is complete.

A toolbar-label width regression found during this pass was corrected; the
viewport-stability assertion, construction-plane checks and divider clamp test
now pass. No tests are marked passed merely because the handoff describes them.

### Review artifacts and remaining scope

- App: `~/Library/Caches/Operandi-Rust/Builds/Operandi Rust M15 Review Dev.app`
  (`org.operandi.rust.m15.review.dev`). It is left open on the saved demonstration.
- Scene: `~/Documents/Operandi-Rust/development/milestone-1.5/Ordinary_Box_Demo.opscene`.
  Native-authored asymmetric box, 8/12/6, one remembered polygon selection,
  reopened and verified in the final review build. Tools are inactive so both
  selection and a fresh live Cube can be tested.
- Recovery/config: isolated `development/milestone-1.5/review-qa/` via optional
  bundle LSEnvironment support; installed applications and user scenes were not
  replaced. A separate disposable QA save also exists at development/Untitled.opscene.

The user-directed pending supplement covers Ctrl-initial draw transitions,
Shift repeat/reset, middle clone placement/retention, unit insertion with active
or transformed targets, and corner/depth handle details. Current unsupported
contexts report feedback and are not permanent product restrictions. Show Corner,
rounding/Bulge/UV generation and advanced property conventions remain unavailable.
Scene selection remains the prior single-item model; ordered multi-item ranges,
item lasso and advanced selection panels/sets are not completed here. More/Less,
neighboring loops, Between and Fill remain reference-dependent, not aliases for
simpler operations. Full panel/reference fidelity and the complete milestone gate
remain open.

Rebuild the review bundle with:

```sh
OPERANDI_DEV_NAME='Operandi Rust M15 Review Dev' \
OPERANDI_DEV_ID=org.operandi.rust.m15.review.dev \
OPERANDI_DEV_DATA='~/Documents/Operandi-Rust/development/milestone-1.5/review-qa' \
./scripts/bundle-dev.sh
```

## Selection continuation and fill — 2026-10-30 (per user guide)

Source: public user guide, "Using Selection Modifiers" (Select More/Less,
Select Between, Polygon Selection Fill), applied under the 2026-10-30
guide-precedence rule. These replace the earlier "unavailable" status for
SEL-056/057/061/064. Labeled *per user guide*; not a verified-parity claim.

Acceptance criteria (written before implementation):

- SEL-M1 More: with at least two ordered vertex/edge/polygon picks, the next
  element along the same loop (or edge ring) is appended at the same stride as
  the last two picks, so a skipped element keeps being skipped. Up arrow.
- SEL-M2 Less: removes the most recently picked element; repeated use can empty
  the selection. Down arrow.
- SEL-M3 Between: two polygons or two edges → shortest connection. Seeds on one
  loop select a single row; seeds in different rows/columns select the
  rectangle. Result ordered from the first seed. Shift+G.
- SEL-M4 Fill: in Polygon mode, Ctrl/Cmd+Shift double-click on an unselected
  polygon inside a closed selected perimeter adds everything enclosed and keeps
  the perimeter.
- SEL-M5 Refusals are atomic with feedback: fewer than two picks, no shared
  loop/ring, end of an open loop, next element already selected, unreachable
  Between seeds, Fill seed already selected or area not enclosed.
- SEL-M6 Each command is one undo step; geometry is untouched; result survives
  save/reopen.

Provisional readings (logged in SPECIFICATION_QUESTIONS.md as SEL-Q-M1..M3):
Between seeds with equal-length routes (e.g. opposite sides of a closed ring)
take every shortest route; Fill refuses an area that touches an open mesh
border; on macOS a single Cmd+Shift click in Polygon mode no longer adds to the
selection so the double-click can reach Fill.

Status (2026-10-30, branch `codex/m15-completion`): SEL-M1..M6 pass —
`crates/operandi-core/src/selection_continuation.rs`,
`crates/operandi-core/tests/selection_continuation_m15.rs` (6 tests) and app test
`m15_up_arrow_selects_more_and_ctrl_shift_double_click_fills`. Wired to the Select
menu, the selection panel, command entry (`more`, `less`, `between`) and the
editable input map (Up, Down, Shift+G). Full workspace tests, fmt and clippy pass;
the one clippy warning is the existing one in `bevel.rs`.
GUI-pass checklist: Up/Down arrow More/Less on a cylinder loop with a skipped
polygon; Shift+G rectangle; Ctrl+Shift and Cmd+Shift double-click Fill inside a
polygon ring.
