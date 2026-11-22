# Modeling tool integration — bounded consolidation, 2026-10-23

CONS-001 is an implementation-only consolidation of numeric draft transport.
It preserves the admitted M11-006/013 field contracts and existing Cone and
Ellipsoid numeric behavior. It adds no product behavior or public extension API.
Implementation uses the reviewed behavioral specification with documented
separation; no private investigation material or reference application was used.

## Bottleneck and boundary

The inspected paths are command dispatch (`Editor::dispatch`, app `run` and
`confirm_fields`), panels, numeric session editors, viewport preview snapshots,
handle drawing/hit testing/gesture dispatch, and checkpoint recording.

Before this pass, `sync_cone_fields` and `sync_ellipsoid_fields` each formatted
vector/scalar values into both text maps. `confirm_cone_fields` and
`ellipsoid_candidate` each filtered their namespace, compared confirmed text,
retried errored text, selected distance/scalar parsing, and handled parse results.
A new numeric-only panel had to duplicate those mechanics as well as implement
its actual parameter mapping. Returning to a previously confirmed value after
an invalid preview was an easy recovery rule to miss.

Now [numeric_draft.rs](../crates/operandi-app/src/numeric_draft.rs) owns those
mechanics for both callers. Panels supply a namespace, distance prefixes, value
bindings, and parameter assignment. Dirty eligibility is captured as borrowed
field references in existing BTreeMap order; parsing remains lazy and stops at
the caller's first error. This small allocation is bounded by panel field count.
Synchronization updates both text maps but deliberately does not clear errors.
Callers retain acceptance, error presentation and error-clearing policy.

The improvement removes two copies of draft filtering/unit selection and two
copies of vector/scalar publication. A future matching panel reuses three
helpers instead of writing those loops. No tool registration framework is added.

Other inspected burdens remain: the general Live confirmation path in `main.rs`
has different recovery rules; viewport drawing and input share `viewport.rs`;
commands explicitly route session types. These are deferred, not expanded into
this pass. Cone viewport calibration and Ellipsoid EA-Q01 remain specification
gaps; neither prevents this change. Compile/test duration is validation cost,
not the integration bottleneck addressed here.

## Adding an authorized tool to the current architecture

1. Define supported inputs, context refusals and acceptance oracles first.
   Supply portable parameters and geometry in core, following
   [ellipsoid.rs](../crates/operandi-core/src/ellipsoid.rs) and its
   [domain tests](../crates/operandi-core/tests/ellipsoid_numeric.rs).
   Geometry returns validated mesh/attributes; UI does not implement geometry.
2. Supply session ownership, preview validation and transaction behavior in a
   core editor module, following
   [ellipsoid_editor.rs](../crates/operandi-core/src/ellipsoid_editor.rs).
   Add command variants and explicit routing in
   [editor.rs](../crates/operandi-core/src/editor.rs), and exports in
   [lib.rs](../crates/operandi-core/src/lib.rs). Preserve target identity and
   validate candidate scenes before publication.
3. Supply the approved UI rows and parameter mapping in a local panel, following
   [ellipsoid_panel.rs](../crates/operandi-app/src/ellipsoid_panel.rs) or
   [cone_panel.rs](../crates/operandi-app/src/cone_panel.rs). Shared `number` and
   `triple` controls in [panels.rs](../crates/operandi-app/src/panels.rs) own
   focus/Enter/Tab/Escape. For a matching numeric-only session, use
   `changed_values` with its namespace and distance prefixes, validate and assign
   into a candidate, then dispatch its Configure command. On acceptance publish
   scalar/vector text with `sync_numeric_value`/`sync_numeric_vector`. Integer
   bounds, min/max semantics, choices, row order and error messages stay local.
4. Connect activation/confirmation through
   [main.rs](../crates/operandi-app/src/main.rs), existing toolbox/menu routes and
   shared commands. Confirmation must gate Apply/Drop/Save. Do not bypass focused
   text or create independent button-only geometry operations.
5. Supply only approved interaction through
   [viewport.rs](../crates/operandi-app/src/viewport.rs),
   [input.rs](../crates/operandi-app/src/input.rs), and, where semantics match,
   [primitive_input.rs](../crates/operandi-app/src/primitive_input.rs).
   Drawing consumes preview snapshots; handle hit testing and gestures must use
   the same coordinates. Numeric-only Cone/Ellipsoid deliberately refuse
   unapproved viewport gestures. This pass changes none of these adapters.
6. Supply tool-specific Apply/Drop/Cancel and checkpoint state in core session
   operations and existing history dispatch. Cone Apply appends while Drop ends
   the session; Ellipsoid Drop can commit a draft. Do not infer a universal
   lifecycle from the shared text helpers. Test preview replacement, error
   recovery, cancel, repeated actions, undo/redo, attributes, and native scene
   roundtrip using the existing Cone/Ellipsoid tests as examples.

Scene serialization remains in
[persistence.rs](../crates/operandi-core/src/persistence.rs); drafts/text are
presentation/session state, not a reason to change the file format. The mesh,
scene, geometry and evaluation contracts remain usable without GUI/GPU startup.

Spike's subsequent implementation is a useful distinct lifecycle example:
[spike.rs](../crates/operandi-core/src/spike.rs) returns a validated replacement
and source-to-descendant correspondence; [spike_editor.rs](../crates/operandi-core/src/spike_editor.rs)
holds inactive configuration and performs one cumulative transaction per Apply.
[spike_panel.rs](../crates/operandi-app/src/spike_panel.rs) reuses scalar draft
transport without creating a preview on numeric confirmation. Do not infer the
primitive draft lifecycle from those shared helpers. Its relative morph and
smoothing-label data use a bounded native-v5 extension, with old files defaulting
empty and ordinary documents retaining v4 encoding. Stored labels/types are
separate from evaluation. [SPIKE.md](archive/SPIKE.md) records the actual contracts/tests
and remaining interactive/state/evaluation limits. This adds no public plugin ABI.

## Validation

Pinned Rust 1.99.1, `~/.cargo/bin/cargo`, repository root:

- `cargo fmt --all --check`: pass.
- `cargo test --locked --workspace --all-features`: 248 passed, zero failed;
  two existing opt-in performance tests ignored. Includes both panels' recovery,
  history and failed-save regressions, domain attributes/persistence tests and
  the new `ownership_order_units_and_retry_of_confirmed_error` helper test.
- `cargo clippy --locked -p operandi-app --all-targets --all-features --no-deps -- -D warnings`: pass.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`:
  fails only existing `match_like_matches_macro` diagnostics in
  `operandi-core/src/lxo/metadata.rs:20,47`. Those files were not changed.
- Same workspace clippy with `-A clippy::match_like_matches_macro`: pass.
- `cargo build --locked --workspace --all-features`: pass.
- `python3 scripts/qa-ellipsoid.py --binary ~/Documents/Operandi-Rust/target/debug/operandi-app --output /tmp/operandi-consolidation-renders`:
  all 12 existing egui/Metal capture cases pass; simulated sizes/scales only.
  This checks preview/error counts, unchanged committed source and capture state.

Logs are `/tmp/operandi-consolidation-{tests,clippy,app-clippy,qualified-clippy,build,renders}.log`;
render records are `/tmp/operandi-consolidation-renders/matrix.json`.
The existing transitive `block v0.1.6` future-compatibility warning remains.
No geometry, dependencies, visuals or scene-format changes. No physical monitor,
mixed-DPI, Windows/Linux or other-GPU validation is claimed.

Native smoke check: built and ad-hoc-signature-verified a separate
`Operandi Consolidation QA.app` using `scripts/bundle-dev.sh` with
`OPERANDI_DEV_NAME='Operandi Consolidation QA'`,
`OPERANDI_DEV_ID=org.operandi.rust.consolidation.qa`, and isolated
`OPERANDI_DEV_DATA=/tmp/operandi-consolidation-native-data`.
In a disposable new mesh, native coordinate entry of Cone Sides=8 followed by
Enter displayed a 9-vertex/9-polygon preview; Apply and Drop ended the session.
Ellipsoid Radius X=`250 mm` confirmed as `0.25 m`, with Min/Max X synchronized
to `-0.25 m`/`0.25 m` and a 266-vertex/288-polygon preview. Cancel removed that
draft. Quit/Discard closed the QA application. AX-only numeric click did not
focus the field, so coordinate input was used; it is not counted as a field
edit pass. No physical display-scale/picking/handle audit is implied by this
bounded native check. The existing Cone review application was not changed.

### Numerical Polygon Extrude

`operandi-core::extrude_geometry` owns pure EX-001..014 candidate construction and
returns the mesh, all caps and moved caps separately. `Editor` owns coordinate
conversion, active/armed state, selection policy, revisions and accepted stages;
`ConfigureExtrude` is not a gesture or an Apply. The app's existing numeric draft
transport confirms fields through that command. `extrude_numeric.rs` contains
representative sparse-map, facing, zero, history and persistence checks. See
[POLYGON_EXTRUDE.md](archive/POLYGON_EXTRUDE.md) for contracts and current limits.

Native v6 adds explicit point UV and corner weight/color distinctions, plus exact
exceptional edge incidence records for admitted Thick results. Future topology
operations must preserve/remap these records or refuse before mutation. The
legacy Bevel remapper currently refuses extended data it cannot retain; consolidating
attribute remapping across tools remains material integration work. It must not
silently assume every effective corner value is a continuous point value.

### SST-1 extension example (2026-10-23)

Smooth Shift/Thicken reuse a pure candidate operation in `smooth_shift.rs`, a
focused session/settings module in `smooth_shift_editor.rs`, the existing Editor
transaction boundary, and one `smooth_shift_panel.rs` text/panel adapter. Tool
identity remains separate from geometric parameters. Follow that ownership split
when extending these tools: add a domain/oracle check first, use Preview for
source replacement, and keep scene/history publication in Editor. New typed maps
and sets are validated/persisted/append-remapped centrally in mesh_attributes and
persistence; every editing remapper must preserve or explicitly refuse them.
Current debt: other older remappers refuse those new payloads, single-item scene
selection limits multiple-target editing, field controls lack the full mini-slider
service, and evaluated subdivision is absent. See the SST report for bounded
acceptance and unapproved extensions; these interfaces are not a stable plugin API.

## Bevel BV revision-2 path (incomplete, 2026-10-23)

`bevel.rs::bevel_geometry` accepts immutable source data and optional source-point
weights and returns an owned candidate/selection. `polygon.rs` owns common region
discovery and projected edge validation; its Bevel adapter uses this same kernel.
`Parameters.bevel` holds options and the existing Shift/Inset/Group fields retain
their input/handle identities. Editor owns stage publication and invalidation;
widget frames never enter geometry. The explicit origin/corner donor arrays own
attribute transfer and generated UV assignment identities. Native v8 adds ordered
normal-map ownership without serializing tool sessions or GPU state.

Existing numeric confirmation routes the extra range/count/angle fields; options
replace the existing stage. No separate geometry mutation route or history engine
was added. Remaining integration burdens are native spatial field/provider setup,
profile/shared recipe ownership and persistent state, configured Apply/reset and
retention, and unimplemented planar repair. These remain assigned implementation
work. See BEVEL_EXPANSION.md for actual failing acceptance and scope; this is not
a public API stability or full-tool completion claim.

### Bevel continuation ownership

`bevel_editor.rs` owns `ConfigureBevel`, attribute reset, named recipes and spatial
field changes; Editor's existing Apply/preview/continue/history services publish
geometry. `BevelRecipe` contains parameters only. `LinearFalloff` is a transient
validated source-world-point provider; other tools explicitly guard it.
`bevel_profile.rs` owns the two admitted shape definitions and bounded sample
workflows; shared shape selection lives separately from recipes and scene data.
`bevel_geometry_with_profile` composes those shape samples with intrinsic contour
and normal responses, retaining explicit corner donors and validating intermediate
contours. Arbitrary adaptive sampling remains BV-Q02.

`bevel_panel.rs` owns guarded recipe/profile UI and versioned atomic app storage;
`profile_browser.rs` owns local navigation, filter/view persistence and original
curve previews. Browsing returns a choice rather than mutating scene geometry.
Source-scene replacement preserves the library/shared selection. Numeric inputs
continue through shared confirmation; option clicks cannot discard invalid text.
The older note listing these integrations as missing is superseded. Remaining
material integration debt is planar weld repair/survivors and the failing zero-
angle oracle. There is still no stable plugin API or complete Bevel parity claim.

The zero-angle grid is now corrected with a documented precision-specific cosine
gate; authored geometry and positive-angle response remain f64. Planar weld
repair/survivors remain the outstanding Bevel integration burden.

## Bridge S1 integration (2026-10-27)

The numeric Bridge candidate keeps geometry and sparse remapping in `crates/operandi-core/src/bridge.rs`, atomic candidate publication in `bridge_editor.rs`, and panel/property transport in `crates/operandi-app/src/bridge_panel.rs`. The shared editor, I1 identity invalidation, I2 session ownership, history and mode eligibility explicitly include Bridge. `crates/operandi-core/tests/bridge_s1.rs` contains geometry oracles and integration witnesses. See `docs/bridge-s1-stabilized-integration-20261027/` for actual check results and pending native validation. Adding a tool still requires reviewing category-specific lifecycle, shared dispatch, identity/history and mode routing, property and command metadata, menu/input presentation, selection and persistence; these coordinated edits remain a material integration burden, not a stable plugin API. Stored subdivision type is not evaluated subdivision.
