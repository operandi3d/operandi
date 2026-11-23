use crate::{Mode, Result, Scene};
use glam::DVec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Box,
    Sphere,
    Cylinder,
    Bevel,
    /// Twist Bevel (twist-bevel-2026-11-15): options in `twist_bevel`
    /// (`twist_bevel.rs`).
    TwistBevel,
    Chamfer,
    PolyHaul,
    SketchExtrude,
    Extrude,
    SmoothShift,
    Thicken,
    Transform,
    Move,
    Rotate,
    Scale,
    /// ME-T03 Extend: Offset X/Y/Z in `delta`, Segments in `segments[0]`.
    EdgeExtend,
    /// ME-T02 Edge Extrude: Extrude distance in `shift`, Width in `inset`.
    EdgeExtrude,
    /// ME-T05 Slide: amount (−1..1 of the rail) in `shift`.
    EdgeSlide,
    /// ME-T10 / EW-07 Edge Weight: weight (-1..1, shown as %) in `shift`.
    EdgeCrease,
    /// ME-T01 Edge Bevel: Inset in `chamfer.offset`, rows (2·Round Level, EBC-B3-02)
    /// in `chamfer.segments`, Depth in `chamfer.depth`.
    EdgeBevel,
    /// ME-T07 Edge Chamfer: parallel Offset in `chamfer.offset`, Segments in
    /// `chamfer.segments`, Depth in `chamfer.depth`.
    EdgeChamfer,
    /// ME-T06 Edge Relax: Offset in `shift`.
    EdgeRelax,
    /// MV-T01 Create: clicked world points in `sketch_path`.
    VertexCreate,
    /// MV-T02 Vertex Bevel: Inset in `inset`.
    VertexBevel,
    /// MV-T03 Vertex Extrude: Extrude in `shift`, Width in `inset`.
    VertexExtrude,
    /// MV-T04 Merge Tool: Distance in `shift`.
    VertexMerge,
    /// MV-T06 Tear: world drag offset in `delta`.
    VertexTear,
    /// MV-T07 Slide (Vertex mode): amount (−1..1) in `shift`, world rail
    /// direction in `delta`.
    VertexSlide,
    /// Tweak (Element Move, key T): drags the element in `tweak` by the
    /// world offset in `delta` (TWK-1).
    Tweak,
    /// Deform tab Push: every selected point moves along its normal by
    /// Distance (`shift`), scaled by the falloff.
    Push,
    /// Deform tab Smooth: `smooth` options; relaxes the selected points.
    Smooth,
    /// Deform finish (deform_finish.rs): `jitter` options; random offsets.
    Jitter,
    /// `quantize` options; snaps points to a world grid.
    Quantize,
    /// Radial Align (`xfrm.radialAlign`): `radial_align` options.
    RadialAlign,
    /// Linear Align (`xfrm.linearAlign`): `linear_align` options.
    LinearAlign,
    /// Deform tab Bend (`xfrm.bend`): `bend` options; curls the target into
    /// an arc (bend.rs).
    Bend,
    /// Symmetry Tool (symmetry-2026-11-13 section 9, SYM-B3-09): dragging
    /// raises `symmetry_fix.threshold`; points whose mirror partner is
    /// within it snap to match (symmetry_fix.rs).
    SymmetryTool,
    /// Edit tab Slice tools (toolbox-panels-2026-11-06): options and placed
    /// cuts in `slice` (`slice.rs`).
    LoopSlice,
    EdgeSlice,
    PolygonSlice,
    /// Polygon Inset (spec inset-2026-11-05): distance in `inset`.
    Inset,
    /// Polygon Shift (tool audit Tier 1, provisional): distance in `shift`.
    Shift,
    /// Edit tab wave 2 path slicing (WAVE2-PATH-SLICE): paths and the
    /// shared cutter in `slice` (`path_slice.rs`).
    CurveSlice,
    PenSlice,
    PrimitiveSlice,
    /// Edit tab wave 3 Polygon Axis Slice (WAVE3-AXIS-SLICE), `slice.axis_slice`.
    AxisSlice,
    /// Topology tab Contour (missing-tools-g4, contour.rs), `contour`.
    Contour,
    /// Duplicate tab (specifications/duplicate-tools): settings in
    /// `duplicate` (`duplicate.rs`, Lathe in `duplicate_sweep.rs`).
    Mirror,
    Lathe,
    Clone,
    Array,
    RadialArray,
    /// Path tools (CX-xx, path_tools.rs): a path generator feeding a Sweep
    /// (Curve / Bezier / B-Spline Extrude) or a Clone (Curve / Bezier /
    /// B-Spline / Pen Clone); settings in `duplicate.curve_extrude` and
    /// `duplicate.curve_clone`.
    CurveExtrude,
    CurveClone,
    /// Scatter Clone (GO-01..07): settings in `duplicate.scatter`.
    ScatterClone,
    /// Symmetrize, the Duplicate tab tool (DX-40..45): settings in
    /// `duplicate.symmetrize` (symmetrize_tool.rs).
    Symmetrize,
    /// UV tab Project (E2, UVT-10..18): `uv_project`; writes the selected
    /// polygons' UVs from a projection (uv_project.rs).
    UvProject,
    /// Project from View (UVT-19): `uv_view_project`; a click in a 3D view
    /// writes the selected polygons' screen positions as UVs.
    UvViewProject,
    /// UV Unwrap as a live tool (UVS-1 §1): `uv_unwrap`.
    UvUnwrap,
    /// UV Relax (E4, UVS-1 §3): `uv_relax` (uv_relax.rs).
    UvRelax,
    /// Unwrap & Relax (UVS-1 §1.5, UVT-20): Unwrap then Relax, `uv_pelt`.
    UvPelt,
    /// Peeler (E5, UVT-30): `uv_peel`; quad strips laid out as a grid.
    UvPeeler,
    /// Box Transform (UVT-43): `uv_box`; UV-only, through the UV pipe.
    UvBoxTransform,
}
impl Tool {
    fn initial_radial(self) -> Option<crate::RadialParameters> {
        match self {
            Self::Sphere => Some(crate::RadialParameters {
                radius: DVec3::splat(0.5),
                sides: 24,
                segments: 12,
            }),
            Self::Cylinder => Some(crate::RadialParameters {
                radius: DVec3::new(0.5, 0.75, 0.5),
                sides: 24,
                segments: 1,
            }),
            _ => None,
        }
    }
    /// Bevel and Twist Bevel: live from the first edit, Shift+click starts
    /// a fresh one on the caps, one undo step per bevel (TA-01..07).
    pub fn is_bevel(self) -> bool {
        matches!(self, Self::Bevel | Self::TwistBevel)
    }
    /// Smooth and the Deform finish point tools show their result for the
    /// panel's values as soon as they are live; drags and fields then
    /// rework that one result.
    pub fn shows_result_at_once(self) -> bool {
        matches!(
            self,
            Self::Smooth
                | Self::Jitter
                | Self::Quantize
                | Self::RadialAlign
                | Self::LinearAlign
                | Self::UvProject
                | Self::UvUnwrap
                | Self::UvRelax
                | Self::UvPelt
                | Self::UvPeeler
                | Self::Symmetrize
        )
    }
    pub fn is_smooth_shift(self) -> bool {
        matches!(self, Self::SmoothShift | Self::Thicken)
    }
    pub fn is_primitive(self) -> bool {
        self.card().family == crate::tool_card::Family::Primitive
    }
    /// Each drag is its own undo step and Undo/Redo keep the tool active
    /// (Move, Rotate, Scale, Transform and Tweak; `ToolCard::undo_per_drag`).
    pub fn is_transform(self) -> bool {
        self.card().undo_per_drag
    }
    /// Moves whole items in Item mode (Move, Rotate, Scale and Transform;
    /// `ToolCard::transforms_items`).
    pub fn transforms_items(self) -> bool {
        self.card().transforms_items
    }
    /// Every tool, in toolbox order. `all_lists_every_tool` fails to compile
    /// when a new variant is not added here too.
    pub const ALL: [Tool; 63] = [
        Tool::Box,
        Tool::Sphere,
        Tool::Cylinder,
        Tool::Bevel,
        Tool::TwistBevel,
        Tool::Chamfer,
        Tool::PolyHaul,
        Tool::SketchExtrude,
        Tool::Extrude,
        Tool::SmoothShift,
        Tool::Thicken,
        Tool::Transform,
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::EdgeExtend,
        Tool::EdgeExtrude,
        Tool::EdgeSlide,
        Tool::EdgeCrease,
        Tool::EdgeBevel,
        Tool::EdgeChamfer,
        Tool::EdgeRelax,
        Tool::VertexCreate,
        Tool::VertexBevel,
        Tool::VertexExtrude,
        Tool::VertexMerge,
        Tool::VertexTear,
        Tool::VertexSlide,
        Tool::Tweak,
        Tool::Push,
        Tool::Smooth,
        Tool::Jitter,
        Tool::Quantize,
        Tool::RadialAlign,
        Tool::LinearAlign,
        Tool::Bend,
        Tool::LoopSlice,
        Tool::EdgeSlice,
        Tool::PolygonSlice,
        Tool::Inset,
        Tool::Shift,
        Tool::CurveSlice,
        Tool::PenSlice,
        Tool::PrimitiveSlice,
        Tool::AxisSlice,
        Tool::Contour,
        Tool::Mirror,
        Tool::Lathe,
        Tool::Clone,
        Tool::Array,
        Tool::RadialArray,
        Tool::CurveExtrude,
        Tool::CurveClone,
        Tool::ScatterClone,
        Tool::Symmetrize,
        Tool::SymmetryTool,
        Tool::UvProject,
        Tool::UvViewProject,
        Tool::UvUnwrap,
        Tool::UvRelax,
        Tool::UvPelt,
        Tool::UvPeeler,
        Tool::UvBoxTransform,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Box => "Cube",
            Self::Sphere => "Sphere",
            Self::Cylinder => "Cylinder",
            Self::Bevel => "Bevel",
            Self::TwistBevel => "Twist Bevel",
            Self::Chamfer => "Chamfer Edit",
            Self::PolyHaul => "Poly Haul",
            Self::SketchExtrude => "Sketch Extrude",
            Self::Extrude => "Extrude",
            Self::SmoothShift => "Smooth Shift",
            Self::Thicken => "Thicken",
            Self::Transform => "Transform",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::EdgeExtend => "Extend",
            Self::EdgeExtrude => "Edge Extrude",
            Self::EdgeSlide => "Slide",
            Self::EdgeCrease => "Edge Weight",
            Self::EdgeBevel => "Edge Bevel",
            Self::EdgeChamfer => "Edge Chamfer",
            Self::EdgeRelax => "Edge Relax",
            Self::VertexCreate => "Create",
            Self::VertexBevel => "Vertex Bevel",
            Self::VertexExtrude => "Vertex Extrude",
            Self::VertexMerge => "Merge Tool",
            Self::VertexTear => "Tear",
            Self::VertexSlide => "Vertex Slide",
            // The reference's name (Jason, 2026-11-06); the variant keeps
            // its old name.
            Self::Tweak => "Element Move",
            Self::Push => "Push",
            Self::Smooth => "Smooth",
            Self::Jitter => "Jitter",
            Self::Quantize => "Quantize",
            // The toolbox buttons say Radial and Linear (DEFORM §1).
            Self::RadialAlign => "Radial Align",
            Self::LinearAlign => "Linear Align",
            Self::Bend => "Bend",
            Self::SymmetryTool => "Symmetry Tool",
            Self::LoopSlice => "Loop Slice",
            Self::EdgeSlice => "Edge Slice",
            Self::PolygonSlice => "Polygon Slice",
            Self::Inset => "Inset",
            Self::Shift => "Shift",
            Self::CurveSlice => "Curve Slice",
            Self::PenSlice => "Pen Slice",
            Self::PrimitiveSlice => "Primitive Slice",
            Self::AxisSlice => "Polygon Axis Slice",
            Self::Contour => "Contour",
            Self::Mirror => "Mirror",
            // DS-61 (spec v1.4, Jason): the button says Radial Sweep;
            // "lathe" stays a command-line alias. The variant keeps its name.
            Self::Lathe => "Radial Sweep",
            Self::Clone => "Clone",
            Self::Array => "Array",
            Self::RadialArray => "Radial Array",
            Self::CurveExtrude => "Curve Extrude",
            Self::CurveClone => "Curve Clone",
            Self::ScatterClone => "Scatter Clone",
            Self::Symmetrize => "Symmetrize",
            Self::UvProject => "Project",
            Self::UvViewProject => "Project from View",
            Self::UvUnwrap => "UV Unwrap",
            Self::UvRelax => "UV Relax",
            Self::UvPelt => "Unwrap & Relax",
            Self::UvPeeler => "Peeler",
            Self::UvBoxTransform => "Box Transform",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub bevel: crate::BevelOptions,
    /// Twist Bevel's properties (spec section 2).
    pub twist_bevel: crate::TwistBevelOptions,
    pub chamfer: crate::ChamferParameters,
    /// Edge Bevel / Edge Chamfer options; their width, rows and Depth are
    /// `chamfer.offset`, `chamfer.segments` and `chamfer.depth`.
    pub edge_bevel: crate::EdgeBevelOptions,
    pub poly_haul_offset: f64,
    /// Poly Haul's mode (spec poly-haul-modes-2026-11-06).
    pub poly_haul_mode: crate::PolyHaulMode,
    /// The element the drag started on, for the element modes.
    pub poly_haul_target: crate::HaulTarget,
    /// The other modes' amounts; Offset and Extrude use
    /// `poly_haul_offset`. `vector` is in world space.
    pub poly_haul_amount: crate::HaulAmount,
    pub sketch_path: Vec<DVec3>,
    pub sketch_precision: f64,
    pub smooth_shift: crate::SmoothShiftParameters,
    pub position: DVec3,
    pub size: DVec3,
    pub inset: f64,
    pub shift: f64,
    pub extent: DVec3,
    pub group_polygons: bool,
    pub extrude_sides: u32,
    pub extrude_mode: crate::ExtrudeMode,
    pub extrude_uvs: crate::ExtrudeUvs,
    pub extrude_uv_map: Option<String>,
    pub extrude_uv_selected: bool,
    pub delta: DVec3,
    pub rotation_delta: DVec3,
    pub scale_delta: DVec3,
    pub box_handle_mode: crate::BoxHandleMode,
    pub show_corners: bool,
    pub segments: [u32; 3],
    pub radial: Option<crate::RadialParameters>,
    /// Cube/Sphere/Cylinder generate the "Texture" UV map (provisional).
    pub make_uvs: bool,
    /// ME-T05 Slide: `shift` is a distance along each rail (ES-03) instead
    /// of a fraction of it. Vertex Slide's Interpolation uses it too
    /// (VTO-T07: one Slide tool serves points and edges).
    pub edge_slide_distance: bool,
    /// VTO-T07 Vertex Slide Merge Vertices (recorded default on): a point
    /// that lands on its neighbour merges with it.
    pub vertex_slide_merge: bool,
    /// VTO-T07 Vertex Slide Face Handle (recorded default on). Stored and
    /// shown only: no behaviour depends on it yet (VTO-Q07).
    pub vertex_slide_face_handle: bool,
    /// Slide options from the tool audit (Edge and Vertex Slide).
    pub slide_extras: crate::SlideExtras,
    /// VTO-T02/T03 Use Material for Vertex Bevel and Vertex Extrude
    /// (recorded default off): the new polygons get `vertex_material`.
    pub vertex_use_material: bool,
    /// VTO-T02/T03 Material Name (recorded default empty).
    pub vertex_material: String,
    /// VX-M05 Merge Tool Keep 1-Vertex Polygons (starting value off).
    pub vertex_merge_keep_small: bool,
    /// VX-M05 Merge Tool Test Morph Map Positions (starting value off).
    pub vertex_merge_test_morphs: bool,
    /// VX-B02 Vertex Bevel Round Level (starting value 0): each side of the
    /// cap becomes a fillet of 2 × level segments.
    pub vertex_round_level: u32,
    /// Scale's Negative Scale option: item scales may go below zero
    /// (IT-04/05). Off refuses them and leaves the item as it was.
    pub negative_scale: bool,
    /// Scale Absolute and Size Absolute (snaps-precision B, C): the Scale
    /// tool's absolute handles. `None` is the plain Scale tool.
    pub absolute_handles: Option<crate::AbsoluteHandles>,
    /// Edge Weight's Additive option (EW2-05): add to each edge's weight.
    pub edge_weight_additive: bool,
    /// Tweak: the element the current drag holds; `None` between drags.
    pub tweak: Option<crate::TweakGrab>,
    /// Tweak's Mode, Range, Shape and Connected Elements.
    pub tweak_options: crate::TweakOptions,
    /// What the current Tweak drag moves (weights and Symmetry
    /// counterparts), resolved at its first frame and reused after (see
    /// `tweak_plan`).
    pub tweak_plan: Option<crate::TweakPlan>,
    /// Topology Pen's modes and options (topology_pen.rs, TP-10..TP-40).
    pub topology_pen: crate::TopologyPenOptions,
    /// Cylinder bottom and top caps (PRIM-Y-01).
    pub cylinder_caps: [crate::CylinderCap; 2],
    /// Sphere and Cylinder axis: 0 X, 1 Y, 2 Z.
    pub radial_axis: usize,
    /// Sphere mode: None is Globe, Some(level) Tessellated (PRIM-S-02).
    pub sphere_level: Option<u32>,
    /// Push's Selection Normals; its Distance is `shift`.
    pub push_normals: crate::PushNormals,
    /// Smooth's options.
    pub smooth: crate::SmoothOptions,
    /// Deform finish tools' options (deform_finish.rs).
    pub jitter: crate::JitterOptions,
    pub quantize: crate::QuantizeOptions,
    pub radial_align: crate::RadialAlignOptions,
    pub linear_align: crate::LinearAlignOptions,
    /// Bend's options.
    pub bend: crate::BendOptions,
    /// The Symmetry Tool's options.
    pub symmetry_fix: crate::symmetry_fix::SymmetryFix,
    /// Loop Slice, Edge Slice and Polygon Slice options and cuts.
    pub slice: crate::SliceParameters,
    /// Contour's settings and drawn line (boxed like `duplicate`).
    pub contour: Box<crate::contour::ContourParameters>,
    /// The Duplicate tools' settings (boxed: `Parameters` sits on the
    /// stack in many places).
    pub duplicate: Box<crate::DuplicateParameters>,
    /// Edge Extend's Handles, Plane, Haul and Make UVs. Its Offset is
    /// `delta`, Rotate `rotation_delta` (degrees), Scale `scale_delta`,
    /// Local Inset/Shift `inset`/`shift` and Segments `segments[0]`.
    pub edge_extend: crate::EdgeExtendOptions,
    /// Set when Move / Rotate / Scale / Transform run in a UV view: they
    /// edit the selected UV map in U and V instead of points in 3D (UVE-1
    /// section 2, uv_edit.rs).
    pub uv: Option<crate::uv_edit::UvToolSpace>,
    /// The UV tab's live tools (E2, E3, E4).
    pub uv_project: crate::uv_project::ProjectOptions,
    pub uv_view_project: crate::uv_project::ViewProjectOptions,
    pub uv_unwrap: crate::uv_tools::UnwrapOptions,
    pub uv_relax: crate::uv_relax::RelaxOptions,
    pub uv_pelt: crate::uv_relax::PeltOptions,
    pub uv_peel: crate::uv_peel::PeelOptions,
    pub uv_box: crate::uv_box::BoxTransform,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            bevel: Default::default(),
            twist_bevel: Default::default(),
            chamfer: Default::default(),
            edge_bevel: Default::default(),
            poly_haul_offset: 0.,
            poly_haul_mode: Default::default(),
            poly_haul_target: Default::default(),
            poly_haul_amount: Default::default(),
            sketch_path: Vec::new(),
            sketch_precision: 30.,
            smooth_shift: Default::default(),
            position: DVec3::ZERO,
            size: DVec3::ONE,
            inset: 0.,
            shift: 0.,
            extent: DVec3::ZERO,
            group_polygons: true,
            extrude_sides: 1,
            extrude_mode: Default::default(),
            extrude_uvs: Default::default(),
            extrude_uv_map: Some("Texture".into()),
            extrude_uv_selected: true,
            delta: DVec3::ZERO,
            rotation_delta: DVec3::ZERO,
            scale_delta: DVec3::ONE,
            box_handle_mode: crate::BoxHandleMode::Resize,
            show_corners: false,
            segments: [1; 3],
            radial: None,
            make_uvs: true,
            edge_slide_distance: false,
            vertex_slide_merge: true,
            vertex_slide_face_handle: true,
            slide_extras: Default::default(),
            vertex_use_material: false,
            vertex_material: String::new(),
            vertex_round_level: 0,
            vertex_merge_keep_small: false,
            vertex_merge_test_morphs: false,
            negative_scale: false,
            absolute_handles: None,
            edge_weight_additive: false,
            tweak: None,
            tweak_options: Default::default(),
            tweak_plan: None,
            topology_pen: Default::default(),
            cylinder_caps: [crate::CylinderCap::Single; 2],
            radial_axis: 1,
            sphere_level: None,
            push_normals: Default::default(),
            smooth: Default::default(),
            jitter: Default::default(),
            quantize: Default::default(),
            radial_align: Default::default(),
            linear_align: Default::default(),
            bend: Default::default(),
            symmetry_fix: Default::default(),
            slice: Default::default(),
            contour: Default::default(),
            duplicate: Default::default(),
            edge_extend: Default::default(),
            uv: None,
            uv_project: Default::default(),
            uv_view_project: Default::default(),
            uv_unwrap: Default::default(),
            uv_relax: Default::default(),
            uv_pelt: Default::default(),
            uv_peel: Default::default(),
            uv_box: Default::default(),
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct Live {
    pub identity_base: Option<crate::IdentityScene>,
    pub identity_preview: Option<(crate::EditCorrespondence, crate::MeshIdentity)>,
    pub tool: Tool,
    pub base: Scene,
    pub params: Parameters,
    pub applied: bool,
    pub action_frame: Option<crate::ActionFrame>,
    /// ACS-07: a centre the user placed (a viewport click or a typed
    /// centre, `SetActionCenter`), world space; `None` while the pivot
    /// service places it.
    pub placed_centre: Option<DVec3>,
    pub preview_error: Option<String>,
}
#[derive(Clone, PartialEq)]
pub struct Checkpoint {
    pub identity_before: Option<crate::IdentityScene>,
    pub identity_after: Option<crate::IdentityScene>,
    pub label: String,
    pub before: Scene,
    pub after: Scene,
    /// The gesture this step belongs to (`Editor::undo_fold`).
    pub fold: Option<u64>,
    /// The command lines that made this step (UIC-226).
    pub records: crate::StepRecords,
    /// `Editor::history_mark` right after this step was recorded: which
    /// steps are newer than a mark, however trimming shifted the list.
    pub mark: u64,
}
/// Most memory the undo history takes (`trim_undo_memory`), counting the
/// mesh data each step alone keeps (undo_memory.rs).
pub const UNDO_MEMORY_BUDGET: usize = 4 << 30;
/// Merge Tool's Distance after Reset (recorded, VTO-D0-05).
pub const MERGE_TOOL_DISTANCE: f64 = 0.001;
/// Where the editor asks how much memory is free (bytes). Not editor
/// state: two editors compare equal whatever their sources.
#[derive(Clone, Copy, Debug, Default)]
pub struct FreeMemory(pub Option<fn() -> u64>);
impl PartialEq for FreeMemory {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Editor {
    /// The undo memory budget in bytes; `None` is `UNDO_MEMORY_BUDGET`.
    pub undo_memory_budget: Option<usize>,
    /// Set while a multi-mesh command records one step per mesh that it
    /// then joins into one (multi_tools.rs): trimming in between would
    /// shift the steps it is about to join (Astra P9-001).
    pub undo_trim_paused: bool,
    /// While set, undo steps recorded under this gesture id fold into one
    /// (a colour drag across several items and frames is one step). The
    /// app sets it around a gesture's commands and clears it after.
    pub undo_fold: Option<u64>,
    /// Meshes a tool or command has just run on as one of several active
    /// meshes (multi_tools.rs): their components were written by that
    /// tool for the new geometry, so they survive the topology check in
    /// `prune_component_selections`. Kept while a live tool is open;
    /// cleared after every other command.
    pub multi_fresh: std::collections::BTreeSet<u64>,
    /// Select > Polygon Tag Type: what tag selection modes select by.
    pub polygon_tag_kind: crate::PolygonTagKind,
    pub pen_draft: Option<crate::PenDraft>,
    /// What the open tool's start inferred from another selection mode
    /// (selection_inference.rs), if anything.
    pub inference: Option<crate::selection_inference::Inference>,
    /// A one-off Symmetry message from the last creation (SYM3-25, e.g.
    /// "Crosses the symmetry plane: not mirrored"); the app shows it in the
    /// status bar and clears it.
    pub symmetry_note: Option<String>,
    /// The Symmetry Tool's points still without a mirror partner, in the
    /// active mesh (the viewport marks them).
    pub symmetry_unpaired: Vec<u32>,
    /// The setting Toggle Symmetry turns back on (the last one before Off).
    pub symmetry_resume: Option<crate::Symmetry>,
    /// Mirror partners added during the open selection gesture (lasso,
    /// rectangle, paint), moved after the swept elements when it ends
    /// (spec v3.1 section 11: swept elements first, then partners).
    pub gesture_partners: Vec<(u64, crate::Component)>,
    /// Interactive Pen options, kept between Pen sessions.
    pub pen_options: crate::PenOptions,
    /// The open Solid Sketch session (solid_sketch_editor.rs).
    pub solid_sketch: Option<crate::SketchSession>,
    /// Solid Sketch options, kept between sessions (spec Q18).
    pub solid_sketch_options: crate::solid_sketch::SketchOptions,
    pub i2: crate::LiveExtrudeHistory,
    pub identities: crate::IdentityRuntime,
    pub bevel_states: std::collections::BTreeMap<String, crate::BevelRecipe>,
    pub bevel_state: Option<String>,
    pub bevel_profile: Option<crate::BevelProfile>,
    /// The active falloff (FAL-M-01); `None` is Falloff None.
    pub spatial_falloff: Option<crate::Falloff>,
    /// Constrain to Background, when on (background_constraint.rs). Tool
    /// context like the falloff: not saved, not an undo step.
    pub background_constraint: Option<crate::background_constraint::BackgroundConstraint>,
    /// Its last options, offered when it is turned on again.
    pub background_constraint_preset: crate::background_constraint::BackgroundConstraint,
    /// The view the last press happened in (Screen Axis searches along it).
    pub background_view: crate::background_constraint::LastView,
    /// The background meshes' surface, kept while they don't change.
    pub background_cache: crate::background_constraint::BackgroundCache,
    /// Topology Pen's options for the session: a new activation starts
    /// from them (TP-10, values stick within the session).
    pub topology_pen_memory: crate::TopologyPenOptions,
    /// Falloffs added on top of the active one, each with its Mix Mode
    /// (VMF-10, VMF-11). Picking another falloff type clears them.
    pub added_falloffs: Vec<(crate::Falloff, crate::MixMode)>,
    /// The active falloff's own Mix Mode (FO-11). Shown on every layer;
    /// on the first layer it has no effect (FO-32, measured). Picking
    /// another type resets it.
    pub spatial_falloff_mix: crate::MixMode,
    /// The Element falloff's pick (FO-45): the element a press on the
    /// model grabbed. Tool context; cleared when the falloff changes.
    pub falloff_element: Option<crate::TweakGrab>,
    /// Screen, Lasso and Airbrush weights per (stack layer, item), worked
    /// out by the app when drawn or painted (FO-Q08). Tool context.
    pub falloff_stamps: std::collections::BTreeMap<(usize, u64), std::sync::Arc<Vec<f64>>>,
    /// The Path falloff's control points (FO-53), in its space. Tool
    /// context.
    pub falloff_path: Vec<glam::DVec3>,
    /// The selected weight map's name (VM-02): the Vertex Map Tools paint
    /// it and the Vertex Map falloff reads it. Tool context, not scene
    /// data.
    pub selected_weight_map: Option<String>,
    /// An open tool preset row (Soft Move, Twist, Flex, Flare ...): its
    /// live tool is the row's tool, and the pipe slots it took go back on
    /// drop (tool_preset.rs).
    pub preset: Option<crate::PresetSession>,
    /// Last Linear and Radial settings, offered on reactivation (FAL-M-03).
    pub linear_falloff_preset: crate::LinearFalloff,
    pub radial_falloff_preset: crate::RadialFalloff,
    pub smooth_shift_preset: crate::SmoothShiftParameters,
    pub chamfer_preset: crate::ChamferParameters,
    pub edge_bevel_preset: crate::EdgeBevelPreset,
    pub edge_chamfer_preset: crate::EdgeBevelPreset,
    pub poly_haul_preset: f64,
    pub sketch_precision_preset: Option<f64>,
    /// Merge Tool's last Distance; `None` is the recorded default 0.001
    /// (VTO-T04).
    pub vertex_merge_preset: Option<f64>,
    /// BN-47: Bend's values from its last use (Angle, Spine, axis, Both
    /// Sides, Deform Space); `None` until it has been dropped once.
    pub bend_preset: Option<crate::BendOptions>,
    /// CS-01 (click_start.rs): honour the tools' Auto-Activate flags, so a
    /// tool flagged off only arms when picked. The app turns it on; off,
    /// every tool starts at once as before (scripts, staged one-shots).
    pub auto_activate_flags: bool,
    /// CS-01: the live tool is armed: picked, values editable, nothing
    /// drawn and the mesh as it was, until the first click or Apply.
    pub armed: Option<Tool>,
    /// N25: Edge Extend was started by a click, so its rows exist even at
    /// zero offset (click_start.rs).
    pub extend_from_click: bool,
    /// How deep inside `dispatch` the editor is: arming and the armed
    /// rules act only on top-level commands (click_start.rs).
    #[doc(hidden)]
    pub dispatch_depth: u32,
    /// 3D painting's saved-image record and last-made image and layer.
    #[cfg(feature = "painting")]
    pub paint: crate::paint::PaintSession,
    pub thicken_preset: Option<crate::SmoothShiftParameters>,
    pub surface_states: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, crate::SmoothShiftParameters>,
    >,
    pub surface_state: std::collections::BTreeMap<String, String>,

    pub bridge: Option<crate::NumericBridge>,
    pub bridge_preset: crate::BridgeParameters,
    pub spike: Option<crate::NumericSpike>,
    pub spike_strength: f64,
    /// MP-T11 Polygon Reduction session; `None` when the tool is off.
    pub reduction: Option<crate::NumericReduction>,
    pub ellipsoid: Option<crate::NumericEllipsoid>,
    pub ellipsoid_preset: crate::EllipsoidParameters,
    /// Torus, Capsule, Arc or Tube numeric session (NPR), and each one's
    /// last settings.
    pub shape: Option<crate::NumericShape>,
    pub shape_presets: crate::ShapePresets,
    pub cone: Option<crate::NumericCone>,
    pub cone_preset: crate::ConeParameters,
    pub scene: Scene,
    pub action_context: crate::ActionContext,
    /// Shared Symmetry setting (SYM-ST-01); tool context, not scene data.
    pub symmetry: crate::Symmetry,
    pub live: Option<Live>,
    /// The document's key time, channel/key selections and playback
    /// (animation.rs; per scene tab).
    pub anim: crate::animation::AnimSession,
    /// RG-11/RG-30 Compensation: rig edits keep items where they are in
    /// world space (off by default; the app keeps it in its preferences).
    pub rig_compensation: bool,
    /// Undo and redo steps, shared: the editor is copied for every command
    /// (identity_editor.rs stages each one), and copying each step's two
    /// scenes made a command in a 100,000-item scene take seconds. Change a
    /// step with `Arc::make_mut` (scene-budget, 2026-11-20).
    pub undo: Vec<std::sync::Arc<Checkpoint>>,
    pub redo: Vec<std::sync::Arc<Checkpoint>>,
    /// Steps recorded since the active tool was activated, for tools that
    /// keep running through Undo/Redo (`mid_tool_undo.rs`). `None` when no
    /// such tool is active or its activation isn't known (re-armed by Undo).
    pub tool_steps: Option<usize>,
    /// The tool Undo just closed, which Redo opens again.
    pub tool_reopen: Option<crate::ToolReopen>,
    /// The active tool's own tweak history (`mid_tool_undo.rs`).
    pub tweaks: crate::ToolTweaks,
    /// The transform drag committed last, for Shift-continue (HM-02) and
    /// middle-drag repeat (HM-04). Not document state.
    pub last_transform_step: Option<Box<crate::transform_repeat::TransformStep>>,
    /// Set by the app while a drag or scrub is held, so its changes are one
    /// tweak; `end_tweak` clears it.
    pub tweak_held: bool,
    /// Counts recorded document steps, to tell a tweak from an Apply.
    pub history_serial: u64,
    /// Counts every change recording makes to the undo history: a new
    /// step, a later edit folded into the newest one, or a selection change
    /// joining a selection group (`recorded_since`). Unlike `undo.len()`, it
    /// still moves once Max Undo Level trims the oldest step.
    pub history_mark: u64,
    /// Redo depth when the active tool started; leaving the tool drops the
    /// steps undone above it.
    pub tool_redo_floor: Option<usize>,
    /// Undo preferences (Max Undo Level, selection grouping); set by the app.
    pub history: crate::HistorySettings,
    /// Command records waiting for the next undo step (UIC-226): the app
    /// adds a line before running it, and a tool session's lines (tool on,
    /// `tool.attr`) wait here until its drop. The next recorded step takes
    /// them all; the app drops them when no step came of them.
    pub pending_records: crate::StepRecords,
    /// Selection changes merged into the newest undo step so far; 0 when
    /// that step is not a selection group.
    pub selection_run: u32,
    pub revision: u64,
    pub next_item_id: u64,
    /// Boxed to keep `Editor` small: the app state is near the stack limit.
    pub selection_start: Option<Box<Scene>>,
    pub extrude_preset: Parameters,
    pub extrude_moved_caps: Vec<u32>,
    /// Extrude on several meshes: each mesh's moving caps and the caps it
    /// selects on Drop (multi_tools.rs). Boxed like `selection_start`.
    pub multi_extrude: Option<Box<crate::multi_tools::MultiExtrude>>,
    pub cube_preset: Parameters,
    /// Last dropped Sides/Segments, used by the next activation and unit
    /// insertion. Not scene data.
    pub sphere_unit_resolution: Option<[u32; 2]>,
    pub cylinder_unit_resolution: Option<[u32; 2]>,
    /// Last dropped Cylinder caps (bottom, top) and axis.
    pub cylinder_shape: Option<([crate::CylinderCap; 2], usize)>,
    /// Last dropped Sphere axis and mode.
    pub sphere_shape: Option<(usize, Option<u32>)>,
    /// Make UVs turned off at the last Drop of Cube, Sphere, Cylinder (in
    /// that order). Stored as "off" so the default is on.
    pub primitive_uvs_off: [bool; 3],
    /// Cut/Copy's app-wide clipboard (clipboard.rs); not scene data and
    /// not undone.
    pub clipboard: Option<Box<crate::Clipboard>>,
    /// The Slice tools' options from their last session.
    pub slice_preset: crate::SliceParameters,
    /// Memory free for the app now, in bytes; Subdivide refuses a result
    /// that needs more (`Editor::subdivide_estimate`). The app sets it;
    /// without it (scripts, tests) nothing is checked.
    pub free_memory: FreeMemory,
    /// Subdivide Polygons' last-used options (SUB-05): the buttons set only
    /// the Method and reuse the rest.
    pub subdivide_preset: crate::SubdivideOptions,
    /// UnSubdivide's last-used options (SUB-40).
    pub unsubdivide_preset: crate::UnsubdivideOptions,
    /// Freeze's last-used options (FRZ-03): what a script's `poly.freeze`
    /// starts from (the app's dialog keeps its own copy too).
    pub freeze_preset: crate::freeze::FreezeOptions,
    /// The Duplicate tools' settings from their last session.
    pub duplicate_preset: Box<crate::DuplicateParameters>,
    /// Detriangulate's last-used options (DT-10, UIC-95).
    pub detriangulate_preset: crate::DetriangulateOptions,
    /// Relax Mesh's last-used options (UIC-95).
    pub relax_mesh_preset: crate::RelaxMeshOptions,
    /// UV Unwrap's and Pack UVs' last-used values (the Pack dialog keeps
    /// them between uses, UV-Q10).
    pub uv_unwrap_preset: crate::uv_tools::UnwrapOptions,
    /// Atlas Unwrap's last settings (uv_atlas.rs).
    pub uv_atlas_preset: crate::uv_atlas::AtlasOptions,
    /// The UV map clipboard: a whole map copied from the map list (UVE-1
    /// section 1).
    pub uv_map_clipboard: Option<crate::uv_maps::UvMapClipboard>,
    /// The E7 dialog values plain clicks use (UVT-08, uv_edit.rs).
    pub uv_island_memory: crate::uv_edit::IslandMemory,
    /// The UV clipboard of Copy / Cut / Paste UVs (UVT-90, uv_ops.rs).
    pub uv_clipboard: Option<crate::uv_ops::UvClipboard>,
    /// The Texel Density panel's values (UVT §7.9), kept between uses.
    pub uv_texel: crate::uv_ops::TexelOptions,
    /// The Cut Map popover's UV Seam: the seam map its buttons use (""
    /// is `uv_seams::DEFAULT_SEAM_MAP`).
    pub uv_seam_map: String,
    /// The UV Symmetry layer (UVT-70): mirrors UV edits in a UV view.
    pub uv_symmetry: crate::uv_symmetry::UvSymmetry,
    pub uv_pack_preset: crate::uv_tools::PackOptions,
    /// Automatic Retopology's, Voxel Remesh's and the octree Remesh's
    /// last-used options (remesh.rs).
    pub retopo_preset: crate::remesh::RetopoOptions,
    pub voxel_remesh_preset: crate::remesh::VoxelOptions,
    pub octree_remesh_preset: crate::remesh::OctreeOptions,
    /// A remesh the app worked out on a worker thread, used by the next
    /// remesh command when it matches (`remesh::RemeshPrepared`).
    pub remesh_prepared: Option<Box<crate::remesh::RemeshPrepared>>,
    /// Boolean's last-used options (BOOL-23).
    pub boolean_preset: crate::BooleanOptions,
    /// What the last successful Boolean wants on the status line.
    pub boolean_note: Option<String>,
    /// MeshOps results (meshops.rs): a cache shared by clones, never part
    /// of the scene (SAFE-03).
    pub meshops: crate::meshops::Runtime,
    /// The scene's evaluation graph (eval_state.rs): a cache, never part
    /// of the scene or undo (AR-04, AR-15).
    pub eval: crate::eval_state::EvalState,
    /// The mesh in Edit Base Mesh (MO-27): direct tools edit its Base Mesh.
    pub base_edit: Option<u64>,
    /// The selected row of the Mesh Operations list.
    pub meshop_row: Option<crate::meshops_editor::OpRow>,
    /// Solid Drill's and Axis Drill's last-used options (DRILL-08).
    pub solid_drill_preset: crate::DrillOptions,
    pub axis_drill_preset: crate::DrillOptions,
    /// What the last successful drill wants on the status line.
    pub drill_note: Option<String>,
    /// What the last headless command that writes a file (a bake) wants on
    /// the status line; the app shows and clears it.
    pub command_note: Option<String>,
    /// What the last Reset or Freeze skipped, for the status line
    /// (item_transform_ops.rs, spec section 7).
    pub transform_note: Option<String>,
}
pub const BACKDROP_NO_GEOMETRY: &str =
    "A Backdrop has no geometry to edit; select a mesh, or use Move, Rotate or Scale on it";
pub const LOCATOR_NO_GEOMETRY: &str = "Cameras, lights and locators have no geometry; edit them in Item mode with Move, Rotate or Scale";
fn backdrop_mut(scene: &mut Scene, id: u64) -> Result<&mut crate::backdrop::Backdrop> {
    scene
        .item_mut(id)
        .and_then(|i| i.backdrop.as_mut())
        .ok_or_else(|| "Select a Backdrop".into())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitDestination {
    ExistingMesh,
    NewMesh,
}
#[derive(Clone)]
pub enum Command {
    /// 3D painting: New Image, Add … Texture and the image commands.
    #[cfg(feature = "painting")]
    Paint(crate::paint::PaintEdit),
    FlipPolygons,
    /// SDV-01/02: Tab / Shift+Tab on the selected polygons (all when none).
    ToggleSubdivision(crate::subdivision::Toggle),
    /// SDV-03: the active mesh's subdivision levels and rules.
    SetSubdivision(crate::subdivision::SubdivisionSettings),
    /// FRZ-01..10 / SDV-12: freeze the active mesh's curves and (with
    /// Tessellation) its subdivision surfaces, one undo step.
    Freeze(crate::freeze::FreezeOptions),
    /// SS-01: the default material's smoothing. `continuing` folds the
    /// change into the previous Smoothing step (one undo step per field
    /// drag).
    SetSmoothing {
        settings: crate::shading::SmoothingSettings,
        continuing: bool,
    },
    /// The scene's final-render settings, one undo step.
    SetRenderSettings(crate::render_settings::RenderSettings),
    /// The scene's frame rate, time range and current time
    /// (scene_time.rs), one undo step.
    SetSceneTime(crate::scene_time::SceneTime),
    Pen(crate::PenInput),
    /// `tool.set solid.sketch on` (options: the bare tool's or `SS
    /// Default`'s). `resume` picks up the mesh's stored sketch (spec 5.5).
    SolidSketchStart {
        options: crate::solid_sketch::SketchOptions,
        resume: bool,
    },
    /// One edit of the live Solid Sketch; `continuing` joins the step of the
    /// edit before it (a drag's later frames).
    SolidSketch {
        edit: crate::SketchEdit,
        continuing: bool,
    },
    /// `operandi.solidSketch.strip` on the selected meshes.
    SolidSketchStrip,
    PenStart,
    PenCapture(DVec3),
    /// Make Quads: add one point that ends the strip with a triangle
    /// (Ctrl+click).
    PenCaptureEnd(DVec3),
    PenSelect(usize),
    /// Reverse the Pen draft's point order (so its polygon faces the other
    /// way); the current point follows its point.
    PenReverse,
    PenRefine(DVec3),
    /// Remove draft point `index` (SPEC-DRAW-POINT-MODE DPM-10, D1); with
    /// `and_after` every later point goes too. In a Make Quads strip the
    /// point's whole pair goes (D2).
    PenDelete {
        index: usize,
        and_after: bool,
    },
    /// Interactive Pen panel options (Type, Flip, Close, Select New).
    PenOptions(crate::PenOptions),
    BeginExtrude,
    /// First viewport click of an Extrude when the I2 stage does not apply:
    /// build the extrusion at zero distance (walls in place, caps unmoved).
    StartExtrude,
    ConfigureSmoothShift(crate::SmoothShiftParameters),
    ResetSmoothShift,
    CloneSmoothShift,
    SurfaceStateSave(String),
    SurfaceStateRecall(Option<String>),
    SurfaceStateRename(String),
    SurfaceStateDelete,

    ArmBridge,
    ConfigureBridge(crate::BridgeParameters),
    ResetBridge,
    ArmSpike,
    ConfigureSpike(f64),
    ResetSpike,
    /// RED-001 `tool.set poly.reduct on`: arm with Number 0.
    ArmReduction,
    /// RED-002 stationary viewport click: capture the source and its count.
    InitializeReduction,
    /// RED-002 Number (and unavailable weight/Legacy refusals).
    ConfigureReduction(crate::ReductionParameters),
    ArmEllipsoid,
    ConfigureEllipsoid(crate::EllipsoidParameters),
    ResetEllipsoid,
    /// Put the live Cube/Sphere/Cylinder's segment settings back to their
    /// starting values and forget the ones kept from earlier uses.
    ResetPrimitive,
    /// Turn the live Cube/Sphere/Cylinder's generated UVs on or off.
    PrimitiveUvs(bool),
    InsertEllipsoidUnit(UnitDestination),
    /// Edit group (clipboard.rs). Copy is `Editor::copy_selection`: it
    /// changes nothing in the scene, so it isn't a command.
    CutSelection,
    PasteSelection,
    /// One Paste Tool copy.
    PasteAt(crate::PastePlacement),
    /// Merge group (item_merge.rs).
    MergeItems,
    /// Draw tools: add, change or remove one curve (one undo step).
    SetCurve(Box<crate::CurveEdit>),
    /// Sketch / Draw Patch: add world-space geometry (one undo step).
    AddGeometry(Box<crate::curve_edit::AddGeometry>),
    UnmergeItem,
    /// Add an item to the item selection or take it out (Ctrl/Shift-click
    /// in the Scenes list).
    ToggleItemSelection(u64),
    /// Align group (align_tools.rs).
    GroundAlign(crate::GroundAlign),
    /// Tack (TC-20..22): the selection onto a picked polygon, one step.
    Tack(crate::Tack),
    /// Relax Mesh... (`sculpt.relaxMesh`, DP-20..22): relax the selected
    /// points (whole mesh when none), one step.
    RelaxMesh(crate::RelaxMeshOptions),
    /// Select tab tools (select_tools.rs, SL-xx): one undo step that
    /// changes only the selection.
    SelectTool(crate::select_tools::SelectRequest),
    /// Merge Instance (merge_instance.rs, MT12).
    MergeInstance,
    /// Freeze Replicator (replicator.rs, G8 RP-5): the selected Replicator
    /// becomes instance items in its group.
    FreezeReplicator,
    /// A Replicator's settings (Properties, replicator.rs, G8 RP-2).
    SetReplicator {
        item: u64,
        settings: Box<crate::replicator::Replicator>,
    },
    /// Vertex Normal Tools (vertex_normals.rs, G9 VN-1..VN-10).
    VertexNormals(Box<crate::vertex_normals::NormalCommand>),
    /// The scene's Guides (guides.rs, G9 GD-1..GD-3).
    SetGuides(Box<crate::guides::Guides>),
    /// One Mesh Paint click or stroke (mesh_paint.rs, MP-16).
    MeshPaint(Box<crate::mesh_paint::MeshPaintStroke>),
    /// Diagonal Re-Direct (diagonal_redirect.rs, MT4).
    DiagonalRedirect,
    /// Edge Zip (edge_zip.rs, MT3).
    EdgeZip,
    /// Fix Black Dots (fix_black_dots.rs, MT2).
    FixBlackDots,
    /// Triple Fan (topology_cleanup.rs, TC-01..03).
    TripleFan,
    /// Collapse Selection (TC-10..12).
    CollapseSelection,
    /// Topology Pen Duplicate (TC-45): pull a new quad from edge `a`–`b`
    /// of `item`, its new edge moved by `offset` (world) onto the
    /// background.
    TopologyDuplicate {
        item: u64,
        a: u32,
        b: u32,
        offset: DVec3,
    },
    /// Topology Pen v2 edits (topology_pen.rs), one undo step each.
    TopologyPen(Box<crate::PenEdit>),
    QuickAlign(crate::QuickAlign),
    /// Unit Cone, Torus or Capsule (unit_insert.rs).
    InsertShapeUnit(crate::UnitShape, UnitDestination),
    /// Start Torus, Capsule, Arc or Tube with its last settings (again on
    /// the same tool: Drop it).
    ArmShape(crate::ShapeKind),
    ConfigureShape(crate::ShapeParameters),
    /// Put the active primitive back to its Reset values.
    ResetShape,
    ArmCone,
    ConfigureCone(crate::ConeParameters),
    InsertUnit {
        tool: Tool,
        destination: UnitDestination,
    },
    Activate(Tool),
    /// Deform tab presets by their handle; the same as `ActivatePreset`
    /// with the preset's row id.
    ActivateDeform(crate::DeformPreset),
    /// Start a tool preset row by id or phrase (tool_preset.rs, TP-10).
    ActivatePreset(String),
    /// Flip It / Flip It Origin (tool presets 6b): mirror the selected
    /// polygons across `axis` (0 X, 1 Y, 2 Z) through the row's action
    /// centre, facing out. One undo step.
    FlipIt {
        preset: String,
        axis: usize,
    },
    ActionContext(crate::ActionContext),
    /// Move a live Move/Rotate/Scale/Transform's pivot to a world point
    /// before it has been applied (Jason, 2026-11-03: a click away from the
    /// model puts the handle there; provisional, VAC-Q02).
    SetActionCenter(DVec3),
    /// A click in a 3D view that places the live tool's centre (CS-20,
    /// BN-24/30a/30b): `point` is where the click landed (snapped point,
    /// mesh surface or work plane) and `plane` the view's work plane
    /// (0 ground XZ, 1 side YZ, 2 front XY). Bend moves its centre there,
    /// refits its spine and puts its Axis on the plane's normal.
    ToolClick {
        point: DVec3,
        plane: u8,
    },
    /// Put a live Move/Rotate/Scale/Transform's pivot back where its
    /// Action Center puts it for the selection, undoing `SetActionCenter`
    /// (Jason, 2026-11-15: a press on the selected element brings the
    /// widget back; provisional, VAC-Q02).
    ResetActionCenter,
    /// End of one Move/Rotate/Scale/Transform drag: record the change so far
    /// as its own undo step and keep the tool live from the result (Jason,
    /// 2026-11-03). Nothing to record is not an error.
    CommitTransformStep,
    /// HM-02: Shift-drag with a transform tool takes up the drag just
    /// committed again, so its values keep growing instead of starting from
    /// zero. Nothing to continue (no drag yet, or the scene changed since)
    /// is not an error: the drag then starts fresh.
    ContinueTransformStep,
    /// HM-06 in Vertex/Edge/Polygon mode: copy the whole polygons the
    /// selection covers into the same mesh and select the copy, so a
    /// transform tool then moves the copy and leaves the original.
    DuplicateSelectedPolygons,
    /// Set the shared Symmetry axis and offset.
    Symmetry(crate::Symmetry),
    /// Symmetry on/off with one key (symmetry-2026-11-13 section 8, Q2):
    /// off remembers the setting, on brings it back (X the first time).
    ToggleSymmetry,
    /// SYM-SEL-01: add the counterparts of the selected elements.
    SelectSymmetrical,
    /// SYM-SZ-01: make the active mesh exactly symmetric from one side.
    Symmetrize(crate::SymmetrizeSide),
    /// Symmetrize (polygons): delete the target half, mirror the source
    /// half across, merge the seam (symmetry-2026-11-13 section 9).
    SymmetrizePolygons {
        side: crate::SymmetrizeSide,
        merge: bool,
    },
    Preview(Parameters),
    ConfigureBevel(Parameters),
    ConfigureChamfer(crate::ChamferParameters),
    ChamferOnce(crate::ChamferParameters),
    PolyHaulOnce(f64),
    /// Poly Haul settings back to their defaults (Offset 0, spec PH-D0).
    ResetPolyHaul,
    /// Sketch Extrude settings back to their defaults (Precision 30, SE-D0).
    ResetSketchExtrude,
    /// Merge Tool Distance back to 0.001 (VTO-D0-05).
    ResetVertexMerge,
    /// Bend back to Angle 0 and Spine (0, 1 m, 0) (BN-47).
    ResetBend,
    SketchExtrudeOnce(Vec<DVec3>),
    ResetBevel,
    /// MN-05: Reset Tool Attributes: the active tool's values back to the
    /// factory, whichever tool it is (`tool_reset.rs`).
    ResetToolAttributes,
    BevelStateSave(String),
    BevelStateRecall(Option<String>),
    BevelStateRename(String),
    BevelStateDelete,
    ResetBevelState,
    BevelProfile(Option<crate::BevelProfile>),
    SpatialFalloff(Option<crate::Falloff>),
    /// Constrain to Background on (with these options) or off. A live tool
    /// stays and re-applies.
    BackgroundConstraint(Option<crate::background_constraint::BackgroundConstraint>),
    /// The falloffs added on top of the active one (VMF-10), replacing the
    /// list. Refused with no active falloff.
    AddedFalloffs(Vec<(crate::Falloff, crate::MixMode)>),
    /// The active falloff's own Mix Mode (FO-11). Refused with no active
    /// falloff.
    SpatialFalloffMix(crate::MixMode),
    /// The Element falloff's pick (FO-45), or none. A live tool stays.
    FalloffElementPick(Option<crate::TweakGrab>),
    /// The Screen, Lasso and Airbrush stamps, per (layer, item), replacing
    /// them (FO-Q08). A live tool re-applies.
    FalloffStamps(std::collections::BTreeMap<(usize, u64), Vec<f64>>),
    /// The Path falloff's control points (FO-53), replacing them. A live
    /// tool re-applies.
    FalloffPath(Vec<glam::DVec3>),
    /// Select a weight map by name, or none (VM-02).
    SelectWeightMap(Option<String>),
    ConfigureExtrude(Parameters),
    ResetExtrude,
    Apply,
    Continue,
    Drop,
    Cancel,
    Undo,
    Redo,
    /// New Mesh Item: an empty top-level mesh, the only selected item.
    NewMesh,
    /// The item list's New: like New Mesh, but beside the selected item
    /// (same parent; spec new-mesh NM08).
    NewMeshBeside,
    /// BD-01 Create Backdrop: a new Backdrop item at the origin, selected.
    NewBackdrop,
    /// BD-03 load an image file (already read by the app for its pixel
    /// size) and show it on Backdrop `item`. A file the scene already has
    /// reuses its entry.
    BackdropLoadImage {
        item: u64,
        path: String,
        width: u32,
        height: u32,
    },
    /// One Backdrop panel control.
    EditBackdrop {
        item: u64,
        edit: crate::backdrop::BackdropEdit,
    },
    /// BD-30..33, BD-61 Auto Fit to the active meshes' box.
    BackdropAutoFit(u64),
    /// BD-60 an image dropped into a 3D view, or `backdrop.add` with
    /// arguments: a new Backdrop (BD-01 origin and sole selection, its name
    /// or "Backdrop Item"; BD-02 defaults) facing `projection` and showing
    /// `image` (path and pixel size, loaded as an image entry, BD-03), if
    /// given. One undo step (provisional, BD-Q21).
    AddBackdrop {
        name: Option<String>,
        image: Option<(String, u32, u32)>,
        projection: crate::backdrop::Projection,
    },
    /// BD-60 an image dropped outside a 3D view (or `clip.addStill`): only
    /// a scene image entry (BD-03), no Backdrop.
    LoadImage {
        path: String,
        width: u32,
        height: u32,
    },
    /// BD-19 Backdrop Size (`height` set) or Backdrop Size Keep Aspect
    /// (`height` none); provisional, BD-Q08.
    BackdropSize {
        item: u64,
        width: f64,
        height: Option<f64>,
    },
    /// IL-03 Add Item: one new item of that type with factory defaults at
    /// the origin, alone selected, placed directly after the selected item
    /// as its sibling (end of the root list with nothing selected).
    AddItem(crate::item_list::AddItemKind),
    /// IL-05 Select popover action on the items `query` matches.
    ItemPattern {
        query: crate::item_list::PatternQuery,
        action: crate::item_list::PatternAction,
    },
    /// CL-1: a new camera at the origin with default settings, selected.
    NewCamera,
    /// CL-1/CL-2 Item > Create Light: a new light of that kind at the
    /// origin with an identity transform, alone selected (CL-Q03).
    NewLight(crate::camera_light::LightKind),
    /// One environment property (its item settings or its Environment
    /// Material), one undo step named after the property (ENV §11).
    EditEnvironment {
        item: u64,
        edit: crate::environment::EnvironmentEdit,
    },
    /// Add, delete or change a texture layer under an environment (ENV
    /// §5, §8), one undo step.
    EditEnvironmentLayers {
        item: u64,
        edit: crate::environment::EnvironmentLayerEdit,
    },
    /// One camera panel control, applied to `item` and to every other
    /// selected camera (CP-X3). One undo step named after the property.
    EditCamera {
        item: u64,
        edit: crate::camera_light::CameraEdit,
    },
    /// CP-E03 Autofocus: camera `item`'s Focus Distance to the surface
    /// under the centre of its view. One undo step.
    CameraAutofocus(u64),
    /// CP-V26: read a picture for camera `item`'s Background Image (one
    /// undo step).
    CameraBackgroundLoad {
        item: u64,
        path: String,
        width: u32,
        height: u32,
    },
    /// CP-V12 Sync to View: move camera `item` to the view's placement.
    SyncCameraToView {
        item: u64,
        transform: crate::scene::Transform,
        target_distance: f64,
    },
    /// CP-V07 / spec freeze-transforms section 3 Reset: part of every
    /// item's transform back to identity, one undo step (`transform.reset`).
    ResetTransform {
        items: Vec<u64>,
        part: crate::camera_props::TransformReset,
    },
    /// Spec freeze-transforms section 4 Freeze: part of every mesh item's
    /// transform baked into its points, nothing moving; one undo step
    /// (`transform.freeze`, item_transform_ops.rs).
    FreezeTransform {
        items: Vec<u64>,
        part: crate::camera_props::TransformReset,
    },
    /// One light panel control, applied to `item` and to every other
    /// selected light (LP-51). One undo step named after the property.
    EditLight {
        item: u64,
        edit: crate::camera_light::LightEdit,
    },
    /// One Display or Assembly tab control (item_common.rs), applied to
    /// every item in `items` (IC-2). One undo step named after the
    /// property (IC-1).
    EditItemCommon {
        items: Vec<u64>,
        edit: crate::item_common::ItemCommonEdit,
    },
    /// Geometry lock or quick hide (element_state.rs, LCK-30..33,
    /// HID-10..13); one undo step each.
    EditElements {
        kind: crate::ElementKind,
        op: crate::ElementOp,
    },
    /// Item groups (item_groups.rs), each one undo step: New Group from
    /// `members`, Rename, Delete (items stay), Add / Remove items.
    NewGroup {
        name: Option<String>,
        members: Vec<u64>,
    },
    RenameGroup {
        group: u64,
        name: String,
    },
    DeleteGroup(u64),
    AddToGroup {
        group: u64,
        items: Vec<u64>,
    },
    RemoveFromGroup {
        group: u64,
        items: Vec<u64>,
    },
    /// Select a group's members (Item mode), like clicking items: not an
    /// undo step.
    SelectGroup(u64),
    /// Select exactly these items, the first one primary (a Shift range or
    /// a parent's double-click in the Items list, UIC-31/38). Not an undo
    /// step.
    SelectItems(Vec<u64>),
    /// LP-PH-01/02 Photometry File: read the IES file at `path` and set the
    /// light's file, Radiant Intensity and Width/Height from it. One undo
    /// step; a bad file changes nothing.
    LoadPhotometry {
        item: u64,
        path: String,
    },
    /// Make `item` the scene's render camera.
    SetRenderCamera(u64),
    /// CL-14: with Move, Rotate or Transform active on a selected camera or
    /// light in Item mode, edit one of its own channels from the tool's
    /// panel or handles. A tweak of the tool: its own undo step inside the
    /// tool, kept on Drop, undone by Cancel.
    ToolItemChannel {
        item: u64,
        edit: crate::camera_light::ItemChannelEdit,
    },
    /// CL-1 Camera view navigation: move camera `item` (its own transform)
    /// and set its Target Distance, as one undo step per press-to-release.
    /// Light view navigation moves a light the same way; a light has no
    /// Target Distance, so `target_distance` is ignored for it.
    CameraView {
        item: u64,
        transform: crate::scene::Transform,
        target_distance: f64,
    },
    DeleteItem,
    /// Delete every selected item, deleting their children too or moving
    /// them to the root in place (item_delete.rs). One undo step. Plain
    /// `DeleteItem` on an item with children moves them to the root.
    DeleteItems(crate::item_delete::ChildrenOnDelete),
    /// POP-PHY-10 Duplicate: a coincident copy of the selected item, named
    /// "Name (2)" (the next free number), alone selected.
    DuplicateItem,
    /// DX-38 Instance: an instance of each selected mesh item in the same
    /// place (duplicate_items.rs); the instances end up selected.
    InstanceItems,
    /// DX-51 Duplicate Hierarchy: each selected item copied with all its
    /// children, links kept (duplicate_hierarchy.rs). One undo step.
    DuplicateHierarchy,
    /// Items arriving from another open scene (a drag between scenes): the
    /// first is the branch root, the rest its descendants, as made by
    /// `Scene::branch`. They get new ids and free names; the root goes
    /// under `parent` keeping its world position. Provisional (MS-Q10).
    InsertItems {
        items: Vec<crate::scene::Item>,
        parent: Option<u64>,
        /// The source scene's pictures the items' Backdrops show; each is
        /// matched to this scene's entry for the same file, or added.
        images: Vec<crate::backdrop::SceneImage>,
    },
    /// Remove an item and all its descendants (the source side of moving
    /// items to another scene).
    RemoveBranch(u64),
    ItemTransform {
        item: u64,
        transform: crate::scene::Transform,
    },
    Rename(String),
    Visibility(u64),
    /// Show or hide several items in one undo step (an eye sweep or Alt+click
    /// in a tree list, a group's eye; UIC-138). Locked items are left as
    /// they are (LCK-05); all of them locked is refused.
    SetItemsVisible {
        items: Vec<u64>,
        visible: bool,
        /// Hiding: the items' eyes also hide their descendants ("all off",
        /// Auto Hide Child Items on, GV-01); the children's own eyes are
        /// left alone. Ignored when showing.
        hides_children: bool,
    },
    /// Toggle an item's visibility and give all its descendants the same
    /// visibility (Jason, 2026-11-02). No eye click uses it since
    /// GV-01..06 (2026-11-10); scripts still can.
    VisibilityWithChildren(u64),
    /// SEL-IT-01..03: parent `item` to `parent` (None unparents). With
    /// `keep_position` the item stays where it is in the world and its own
    /// transform changes; without it the transform is kept and it moves.
    SetParent {
        item: u64,
        parent: Option<u64>,
        keep_position: bool,
    },
    /// GL-04 Parent to Group Locator: the selected items go under one new
    /// Group Locator, which becomes the only selected item (locator.rs).
    ParentToGroupLocator,
    /// UIC-143 tree drop: put `items` under `parent` (None: top level),
    /// just before the sibling `before` (None: last). Reparented items
    /// keep their world position. One undo step: "Reorder" when no parent
    /// changed, else "Parent" / "Unparent".
    MoveItems {
        items: Vec<u64>,
        parent: Option<u64>,
        before: Option<u64>,
    },
    /// Drop Action (drop-action-2026-11-06): `sources` dropped onto
    /// `destination` in the viewport; one undo step named after the action.
    /// `ray` is the eye-to-pointer ray Place and Place and Align use.
    DropAction {
        action: crate::DropAction,
        sources: Vec<u64>,
        destination: u64,
        ray: Option<crate::drop_action::Ray>,
    },
    Mode(Mode),
    /// Polygon tag selection mode (note 04): Polygon mode where a click
    /// selects whole tagged regions of `Editor::polygon_tag_kind`.
    /// `convert` (Alt+click) grows the current selection to its regions.
    EnterTagMode {
        convert: bool,
    },
    /// The global Polygon Tag Type (Select > Polygon Tag Type). In a tag
    /// mode it changes what clicks select; the mode stays.
    SetPolygonTagKind(crate::PolygonTagKind),
    SelectItem(Option<u64>),
    Select {
        item: u64,
        hit: crate::Component,
        replace: bool,
        remove: bool,
    },
    BeginSelection,
    EndSelection {
        cancel: bool,
    },
    ClearSelection,
    Selection(crate::SelectionOperation),
    /// Polygon Selection Fill from a double-clicked polygon inside the perimeter.
    SelectionFill {
        item: u64,
        face: u32,
    },
    /// Edge-mode double-click: select edge `a`-`b` of `item` and its edge
    /// loop; `add` (Shift) adds the loop to the existing edge selection.
    SelectEdgeLoop {
        item: u64,
        a: u32,
        b: u32,
        add: bool,
    },
    /// Polygon-mode double-click: select polygon `face` of `item` and every
    /// polygon connected to it; `add` (Shift) adds that piece to the
    /// existing polygon selection.
    SelectConnectedPolygons {
        item: u64,
        face: u32,
        add: bool,
    },
    SelectPolygonSet(String),
    /// SEL-081 Assign Selection Set: save the current Vertex, Edge or
    /// Polygon selection of the active mesh under a name (adds to an
    /// existing set of that name).
    AssignSelectionSet(String),
    /// SEL-082 Use Selection Set: select (Add), deselect (Remove) or
    /// Replace with a current-mode set.
    UseSelectionSet {
        name: String,
        mode: crate::UseSetMode,
    },
    /// SEL-082 Delete Selection Set: one current-mode set by name, or all of
    /// them with `None`. Never deletes geometry.
    DeleteSelectionSet(Option<String>),
    /// Create Selection Set Group: a named group of current-mode sets (all
    /// of them when the list is empty).
    CreateSelectionSetGroup {
        name: String,
        sets: Vec<String>,
    },
    /// Delete a selection set group; its sets stay.
    DeleteSelectionSetGroup(String),
    /// One-shot Set Part on the selected polygons, or all of them when
    /// nothing is selected (SP-05).
    SetPart(crate::PartAssignment),
    SetPosition(crate::PositionAssignment),
    /// One Absolute Scaling button press (absolute_scaling.rs).
    AbsoluteScale(crate::AbsoluteScale),
    SetMaterial(crate::MaterialAssignment),
    /// Shading panel edits (ST-1): add, delete, move, eye, layer edits.
    ShaderTree(crate::ShaderTreeEdit),
    /// Load a preset (preset.rs, PB-31): one undo step. `group` is the
    /// Material Group a material preset goes into.
    LoadPreset {
        preset: Box<crate::preset::Preset>,
        group: Option<crate::shader_tree::NodeId>,
    },
    /// Add a material preset's layers to the shader tree without replacing
    /// anything (`operandi.materialx.import`): directly above `beside`, in
    /// its group, or where Add Layer puts a group with none. One undo step.
    InsertMaterialPreset {
        preset: Box<crate::preset::Preset>,
        beside: Option<crate::shader_tree::NodeId>,
    },
    /// Make the picture at `path` an environment's image (preset browser
    /// CD-13, Load Image…, an HDR dropped on a view): one undo step.
    /// `width`/`height` are its pixel size. `item` is the environment;
    /// `None` is the top (newest) one, made when there is none.
    SetEnvironmentImage {
        path: String,
        width: u32,
        height: u32,
        item: Option<u64>,
    },
    /// Apply environment preset `preset` (EP §Apply) as `mode`, one undo
    /// step; its lights and meshes remember `name` (the preset's name).
    ApplyEnvironmentPreset {
        preset: Box<crate::preset::Preset>,
        mode: crate::preset::ApplyMode,
        name: String,
    },
    /// A material preset dropped onto geometry (PB-110): polygons `faces`
    /// of `item` (all when empty) get a new tag and the preset's material.
    ApplyMaterialPreset {
        preset: Box<crate::preset::Preset>,
        item: u64,
        faces: Vec<usize>,
    },
    UnifyPolygons(crate::UnifyRequest),
    SplitPolygon,
    QuadruplePolygon,
    SpinQuads,
    /// MP-C08 one-shot Align of an explicitly selected adjacent quad pair.
    AlignPolygons,
    /// MP-C04 one-shot Triple on the explicit polygon selection.
    Triple,
    /// MP-C10 one-shot Remove of the explicit polygon selection.
    RemovePolygons,
    /// Forward Delete in a component mode (`delete`): the elements go with
    /// the points they leave unused (Jason, 2026-11-10).
    DeleteComponents,
    /// MP-C09 one-shot Merge of the explicit polygon selection.
    MergePolygons,
    /// MP-C11 one-shot Collapse of one explicitly selected simple quad.
    CollapsePolygon,
    /// MP-C14 one-shot Flatten Ngons of the active mesh, scoped by an explicit
    /// polygon selection (core only; FN-NP-1A).
    FlattenNgons,
    /// MP-C13 one-shot Scale Offset of one isolated convex polygon by a
    /// signed boundary distance in scene units (positive contracts).
    ScaleOffsetPolygon(f64),
    /// PolyScale Offset Detach: inset the selected polygons onto new
    /// points, leaving them unconnected (spec polyscale-offset B4).
    ScaleOffsetDetach(f64),
    /// CLEAN-001: remove floating vertices from an explicit idle Item target.
    CleanupFloating(Option<crate::CleanupTarget>),
    /// Mesh Cleanup with its twelve options (`CleanupOption` order).
    MeshCleanup {
        flags: [bool; 12],
        target: Option<crate::CleanupTarget>,
    },
    /// MP-C01 one-shot Make: fill one four-corner opening from the two
    /// ordered selected boundary edges (`poly.make auto`).
    MakePolygon,
    /// Make Polygon with an explicit Type and Close Curve (the menu entry's
    /// dialog); `MakePolygon` is the P key's Auto.
    MakePolygonAs {
        kind: crate::MakeType,
        close: bool,
    },
    /// CC-01 Make Open / Closed Curve through the selected vertices in
    /// selection order (curve_commands.rs).
    MakeCurve {
        closed: bool,
    },
    /// CC-10 Make Curve Fill on the selected closed curves.
    MakeCurveFill,
    /// CC-20 Align Winding of the selected curves.
    AlignWinding(crate::curve_commands::WindingOptions),
    /// CB-xx Curve Boolean on the selected curves (curve_boolean.rs).
    CurveBoolean(crate::curve_boolean::CurveBooleanOptions),
    /// CC-50..53 Convert Curves into a new item (curve_convert.rs).
    ConvertCurves {
        from: crate::curve_convert::CurveType,
        to: crate::curve_convert::CurveType,
    },
    /// CC-40..43 Curve Offset (Trio: both sides).
    OffsetCurves {
        closed: bool,
        offset: f64,
        trio: bool,
    },
    /// CC-60..64 Bezier Tangent on the selected knots.
    BezierTangent(crate::curve_convert::TangentOptions),
    /// GO-20 UV Rectangle (generator_ops.rs).
    UvRectangle(crate::generator_ops::RectangleOptions),
    /// Dicer (dicer.rs): cut the active mesh into pieces.
    Dicer(crate::dicer::DicerOptions),
    /// GO-30 Transfer Vertex Map from the background mesh.
    TransferVertexMap {
        map: String,
        kind: crate::generator_ops::TransferType,
        /// Sampling Space World (placements count) or Local (GO-33).
        world: bool,
        sampling: crate::generator_ops::TransferSampling,
    },
    /// ME-C01..C13 one-shot Edge command rows on the selected edges.
    EdgeCommand(crate::EdgeCommand),
    /// CMD-B3-01 Subdivide with a Count (intervals, at least 2) or, with
    /// Count 1, a Percent along each edge.
    SubdivideEdges {
        count: u32,
        percent: Option<f64>,
    },
    /// SUB-05..15 Subdivide Polygons (`poly.subdivide`), `times` passes as
    /// one undo step (Alt+click SDS Subdivide: 2).
    SubdividePolygons {
        options: crate::SubdivideOptions,
        times: u32,
    },
    /// SUB-40..44 UnSubdivide (`poly.unsubdiv`).
    UnsubdividePolygons(crate::UnsubdivideOptions),
    /// Detriangulate the selected polygons (all when none), one step.
    Detriangulate(crate::DetriangulateOptions),
    /// UV Unwrap of the visible polygons into the target UV map, one step.
    UvUnwrap(crate::uv_tools::UnwrapOptions),
    /// Atlas Unwrap of the visible polygons into the target UV map, one step.
    UvAtlas(crate::uv_atlas::AtlasOptions),
    /// UV Relax of the UV map as one step (`uv.relax` command form).
    UvRelax(crate::uv_relax::RelaxOptions),
    /// The UV map list (UVE-1 section 1): new, select, rename, delete,
    /// copy, paste.
    UvMap(crate::uv_maps::UvMapEdit),
    /// One island affine run (E7: Fit, Flip, Mirror, Axis Align).
    UvIsland(crate::uv_edit::IslandOp),
    /// One of the UV tab's other buttons (uv_ops.rs: align, orient,
    /// texel density, split and sew, UV queries and values).
    UvOp(crate::uv_ops::UvOp),
    /// The Texel Density panel's values (no undo step).
    UvTexel(crate::uv_ops::TexelOptions),
    /// The Cut Map popover's seam map buttons (uv_seams.rs).
    UvSeam(crate::uv_seams::SeamEdit),
    /// The UV Symmetry layer (no undo step, like 3D Symmetry).
    UvSymmetry(crate::uv_symmetry::UvSymmetry),
    /// UV Symmetry's Center button: weld the UVs on the line.
    UvSymmetryCenter,
    /// Symmetrize UVs (UVT-71).
    UvSymmetrize(crate::uv_symmetry::SymmetrizeOptions),
    /// Convert UVs to Mesh (UVT-120): a new flat mesh item.
    UvToMesh,
    /// Transfer UVs (UVT-92): the primary mesh's map to the others.
    UvTransfer,
    /// A click on an E7 button of the tool preset table.
    UvIslandPreset {
        preset: String,
        click: crate::uv_edit::Click,
    },
    /// Put the open Move / Rotate / Scale / Transform in UV space on the
    /// named map (`None` in the name: the mesh's selected map), or back in
    /// 3D with `None`. Only before the tool has changed anything.
    UvToolSpace(Option<Option<String>>),
    /// Pack UVs: the islands of the target UV map, one step.
    PackUvs(crate::uv_tools::PackOptions),
    /// `mesh.retopo`, `sculpt.remesh` or `operandi.remesh` on the active
    /// mesh, one step (remesh.rs).
    Remesh(crate::remesh::RemeshOp),
    /// Select Connected in a UV view (UVV-04): the UV islands of the
    /// selection in that map (the mesh's UV map when `None`).
    SelectUvIslands(Option<String>),
    /// BOOL-01..25 Boolean (`poly.boolean`) of the active mesh, one step.
    Boolean(crate::BooleanOptions),
    /// MeshOps (meshops_editor.rs): add an op of this type to the
    /// selected mesh (`meshop.create`).
    MeshOpCreate(String),
    /// Delete ops (`item.delete item:<op>`).
    MeshOpDelete(Vec<crate::meshops_editor::OpRef>),
    /// Move an op to a position in evaluation order (`item.parent`).
    MeshOpMove {
        op: crate::meshops_editor::OpRef,
        position: usize,
    },
    /// An op's eye (`deformer.enable`).
    MeshOpEnable {
        op: crate::meshops_editor::OpRef,
        on: bool,
    },
    /// Set op values by attribute name (`item.channel ... item:<op>`).
    MeshOpSet {
        op: crate::meshops_editor::OpRef,
        values: serde_json::Map<String, serde_json::Value>,
    },
    /// Select a row of the Mesh Operations list (`select.item <op>`).
    MeshOpSelect(Option<crate::meshops_editor::OpRow>),
    /// Select by Index Set/Clear/Add/Remove (`selop.selection`).
    MeshOpIndex {
        op: crate::meshops_editor::OpRef,
        edit: crate::meshops_editor::IndexEdit,
    },
    /// Add, change or remove one of an op's selection ops (`selop.create`,
    /// `selop.edit`).
    MeshOpSelOp {
        op: crate::meshops_editor::OpRef,
        edit: crate::meshops_editor::SelOpEdit,
    },
    /// Freeze an op and those below it (`deformer.freeze`).
    MeshOpFreeze {
        op: crate::meshops_editor::OpRef,
        duplicate: bool,
    },
    /// Freeze the selected mesh's whole stack (`mesh.freeze`).
    MeshOpFreezeAll,
    /// Edit Base Mesh on or off (`deformer.selectBaseMesh`).
    MeshOpEditBase {
        item: u64,
        on: bool,
    },
    /// DRILL-01..27 Solid Drill (`poly.solidDrill`) of the active mesh.
    SolidDrill(crate::DrillOptions),
    /// DRILL-01..27 Axis Drill (`poly.drill`) of the active mesh.
    AxisDrill(crate::DrillOptions),
    /// CEN-01..04 Center (`vert.center`) on the world axes that are on.
    CenterSelection([bool; 3]),
    /// M3 Vertex command rows (MV-C04..C15) on the selected points.
    VertexCommand(crate::VertexCommand),
    /// MV-C01 New Vertex at a world position.
    NewVertex(DVec3),
    /// MV-T05 Weld `sources` onto `target`.
    WeldVertices {
        sources: Vec<u32>,
        target: u32,
    },
    /// WD-01..05: one Weld drag of point `source`, released onto another
    /// point (merge) or at a world position (move). One undo step.
    WeldDrag {
        source: u32,
        drop: crate::WeldDrop,
    },
    /// One finished sculpt stroke (sculpt.rs, SC-90): the points it moved,
    /// one undo step named after the brush.
    SculptStroke(Box<crate::SculptEdit>),
    /// One finished Vertex Map Tools stroke (VMT-19): the weights it set,
    /// one undo step named after the tool.
    WeightStroke(Box<crate::sculpt::WeightEdit>),
    /// New Weight Map (VM-03) on the active meshes; `value` fills every
    /// point (Init Value), `None` leaves them with no value. Selects it.
    NewWeightMap {
        name: String,
        value: Option<f64>,
    },
    /// MV-C16 Sync Index Order from a captured source item and two polygons
    /// to the active item's two selected polygons.
    SyncVertexOrder {
        source_item: u64,
        source_faces: [u32; 2],
    },
    /// ME-T09 Add Point: split edge `a`–`b` of the active mesh at `t` (0..1).
    AddEdgePoint {
        a: u32,
        b: u32,
        t: f64,
    },
    /// ME-T08 Add Loop: cut the polygon band crossing edge `a`–`b` at `t`.
    AddEdgeLoop {
        a: u32,
        b: u32,
        t: f64,
    },
    /// Add Point / Add Loop with placement options (Both Sides, Absolute
    /// Distance, Preserve Curvature). Distances are world metres.
    AddEdgePointWith {
        a: u32,
        b: u32,
        t: f64,
        options: crate::EdgeInsertOptions,
    },
    AddEdgeLoopWith {
        a: u32,
        b: u32,
        t: f64,
        options: crate::EdgeInsertOptions,
    },
    Workplane(crate::Workplane),
}
impl Editor {
    /// Read-only availability check for UI affordances. Dispatch resolves the
    /// same frame again before publishing any context or live-state change.
    pub fn validate_action_context(&self, context: crate::ActionContext) -> Result<()> {
        self.action_context_frame(context).map(|_| ())
    }
    fn action_context_frame(
        &self,
        context: crate::ActionContext,
    ) -> Result<Option<crate::ActionFrame>> {
        // Selection, Parent and Item axes come from the scene (the
        // selection, the item hierarchy), not from the setting alone.
        if !matches!(
            context.axis,
            crate::AxisOrientation::Selection
                | crate::AxisOrientation::Parent
                | crate::AxisOrientation::Item
        ) {
            context.axis.basis()?;
        }
        if context.axis == crate::AxisOrientation::Selection && self.live.is_none() {
            // The same world-space frame a tool gets on activation, so a
            // moved mesh doesn't grey the choice out.
            Self::extrude_frame(context, Tool::Move, &self.scene)?;
        }
        if let Some(live) = &self.live {
            if live.action_frame.is_none() {
                return Err(
                    "Drop or Cancel this tool before changing Action Center or axes".into(),
                );
            }
            // A tool in a UV view: the frame in UV (UVT-133).
            if live.params.uv.is_some() {
                return self.uv_action_frame(context);
            }
            Ok(Some(
                crate::pivot_service::PivotRequest {
                    context,
                    ..self.pivot_request(&live.base, live.tool)
                }
                .frame()?,
            ))
        } else {
            Ok(None)
        }
    }
    /// Change the Symmetry setting (tool context, not saved). Turning it off
    /// remembers the last setting for Toggle Symmetry.
    ///
    /// Tool pipe (Jason, 2026-11-14): Symmetry is a layer under every tool,
    /// so it changes at any time and the open tool follows at once: its
    /// handle moves to the picked side (or back), and whatever it shows is
    /// rebuilt under the new plane. Not an undo step of its own. If the
    /// open tool can't take the new setting, everything stays as it was.
    fn set_symmetry(&mut self, symmetry: crate::Symmetry) -> Result<()> {
        symmetry.validate()?;
        let previous = self.symmetry;
        let staged = (
            self.scene.clone(),
            self.live.clone(),
            self.cone.clone(),
            self.ellipsoid.clone(),
            self.shape.clone(),
            self.solid_sketch.clone(),
        );
        self.symmetry = symmetry;
        if let Err(error) = self.follow_symmetry(previous) {
            self.symmetry = previous;
            (
                self.scene,
                self.live,
                self.cone,
                self.ellipsoid,
                self.shape,
                self.solid_sketch,
            ) = staged;
            return Err(error);
        }
        if previous.axis.is_some() && symmetry.axis.is_none() {
            self.symmetry_resume = Some(previous);
        }
        // Section 8's unpaired warning follows the new plane too.
        self.note_unpaired_points();
        Ok(())
    }
    /// Off, or back on with the last setting (X the first time).
    fn toggle_symmetry(&mut self) -> Result<()> {
        let next = if self.symmetry.axis.is_some() {
            crate::Symmetry::OFF
        } else {
            self.symmetry_resume.unwrap_or_else(crate::Symmetry::x)
        };
        self.set_symmetry(next)
    }
    /// The open tool picks up a Symmetry change (see `set_symmetry`).
    fn follow_symmetry(&mut self, previous: crate::Symmetry) -> Result<()> {
        let previous = previous.through(self.scene.workplane.center);
        let current = self.active_symmetry();
        if let Some(live) = &self.live
            && let Some(frame) = live.action_frame
            && live.tool != Tool::Bend
        {
            // The handle follows the pivot service under the new plane
            // (ACS-01). A centre the user placed (SetActionCenter, ACS-07)
            // is kept.
            let request = |symmetry| crate::pivot_service::PivotRequest {
                symmetry,
                ..self.pivot_request(&live.base, live.tool)
            };
            let _ = (previous, frame);
            if live.placed_centre.is_none() {
                let now = request(current).frame()?;
                if let Some(live) = &mut self.live {
                    live.action_frame = Some(now);
                }
            }
        }
        if let Some(parameters) = self
            .live
            .as_ref()
            .filter(|live| live.applied)
            .map(|live| live.params.clone())
        {
            self.preview(parameters, false)?;
        }
        if let Some(session) = self.cone.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_cone(session.requested.clone())?;
        }
        if let Some(session) = self.ellipsoid.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_ellipsoid(session.requested.clone())?;
        }
        if let Some(session) = self.shape.as_ref().filter(|s| s.preview.is_some()) {
            self.configure_shape(session.requested.clone())?;
        }
        let mirror = crate::solid_sketch_symmetry::SketchMirror::of(current);
        if let Some(session) = &mut self.solid_sketch {
            session.follow_symmetry(mirror);
        }
        Ok(())
    }
    pub(crate) fn dispatch_inner(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        // Tool context: works with any tool open, a Pen draft too.
        if let Command::BackgroundConstraint(c) = cmd {
            return self.set_background_constraint(c);
        }
        // Symmetry too, Solid Sketch and every other session included: the
        // open tool follows the change (tool pipe, Jason 2026-11-14).
        match cmd {
            Command::ToggleSymmetry => return self.toggle_symmetry(),
            Command::Symmetry(symmetry) => return self.set_symmetry(symmetry),
            _ => {}
        }
        // Several active meshes: the command runs on each (multi_tools.rs).
        if let Some(targets) = self.command_mesh_targets(&cmd) {
            return self.command_on_meshes(&targets, &cmd);
        }
        // Selection inference for the point and edge commands that take it
        // (selection_inference.rs): run on the converted set, mode kept.
        if let Some(target) = Self::command_takes(&cmd)
            && matches!(
                self.scene.selection.mode,
                Mode::Vertex | Mode::Edge | Mode::Polygon
            )
            && self.scene.selection.mode != target
        {
            return self.dispatch_inferred(target, cmd, report);
        }
        if let Command::CleanupFloating(target) = &cmd {
            return self.cleanup_floating(target.clone());
        }
        if let Command::MeshCleanup { flags, target } = &cmd {
            return self.mesh_cleanup(*flags, target.clone());
        }
        if matches!(&cmd, Command::QuadruplePolygon) {
            return self.quadruple_polygon();
        }
        if let Command::UnifyPolygons(request) = &cmd {
            return self.unify_polygons(request.clone());
        }
        if matches!(&cmd, Command::SpinQuads) {
            return self.spin_quads();
        }
        if matches!(&cmd, Command::AlignPolygons) {
            return self.align_polygons();
        }
        // Like Align, Flatten Ngons is idle-only; its own preflight refuses
        // drafts, live stages and sessions instead of consuming them.
        if matches!(&cmd, Command::FlattenNgons) {
            return self.flatten_ngons();
        }
        if matches!(&cmd, Command::SplitPolygon) {
            return self.split_polygon();
        }
        if matches!(&cmd, Command::MergeItems) {
            return self.merge_items();
        }
        if let Command::SetCurve(edit) = cmd {
            return self.set_curve(*edit);
        }
        if let Command::AddGeometry(add) = cmd {
            return self.add_geometry(*add);
        }
        if matches!(&cmd, Command::UnmergeItem) {
            return self.unmerge_item();
        }
        if matches!(&cmd, Command::MergeInstance) {
            return self.merge_instance();
        }
        if matches!(&cmd, Command::FreezeReplicator) {
            return self.freeze_replicator();
        }
        if let Command::SetReplicator { item, settings } = cmd {
            return self.set_replicator(item, *settings);
        }
        if let Command::VertexNormals(c) = cmd {
            return self.vertex_normals(*c);
        }
        if let Command::SetGuides(g) = cmd {
            return self.set_guides(*g);
        }
        if let Command::MeshPaint(stroke) = cmd {
            return self.mesh_paint(*stroke);
        }
        if matches!(&cmd, Command::DiagonalRedirect) {
            return self.diagonal_redirect();
        }
        if matches!(&cmd, Command::EdgeZip) {
            return self.edge_zip();
        }
        if matches!(&cmd, Command::FixBlackDots) {
            return self.fix_black_dots();
        }
        if matches!(&cmd, Command::TripleFan) {
            return self.triple_fan();
        }
        if let Command::TopologyDuplicate { item, a, b, offset } = cmd {
            return self.topology_duplicate(item, a, b, offset);
        }
        if matches!(&cmd, Command::CollapseSelection) {
            return self.collapse_selection();
        }
        if let Command::TopologyPen(edit) = cmd {
            return self.topology_pen(*edit);
        }
        if let Command::SelectTool(request) = cmd {
            return self.select_tool(request);
        }
        if let Command::RelaxMesh(options) = cmd {
            return self.relax_mesh(options);
        }
        if let Command::Tack(t) = cmd {
            return self.tack(t);
        }
        if let Command::GroundAlign(options) = cmd {
            return self.ground_align(options);
        }
        if let Command::QuickAlign(q) = cmd {
            return self.quick_align(q);
        }
        if matches!(&cmd, Command::CutSelection) {
            return self.cut_selection();
        }
        if matches!(&cmd, Command::PasteSelection) {
            return self.paste(None);
        }
        if let Command::PasteAt(placement) = cmd {
            return self.paste(Some(placement));
        }
        // MK-C03: idle-only like Align; its own preflight refuses a Pen draft,
        // live stage, session or selection gesture instead of consuming it.
        if matches!(&cmd, Command::MakePolygon) {
            return self.make_polygon(crate::MakeType::Auto, false);
        }
        if let Command::MakePolygonAs { kind, close } = cmd {
            return self.make_polygon(kind, close);
        }
        if let Command::MakeCurve { closed } = cmd {
            return self.make_curve_command(closed);
        }
        if matches!(&cmd, Command::MakeCurveFill) {
            return self.make_curve_fill_command();
        }
        if let Command::AlignWinding(options) = cmd {
            return self.align_winding_command(options);
        }
        if let Command::Dicer(o) = cmd {
            return self.cut_pieces("Dicer", |m| crate::dicer::dicer(m, &o));
        }
        if let Command::CurveBoolean(o) = cmd {
            return self.curve_boolean_command(o);
        }
        if let Command::ConvertCurves { from, to } = cmd {
            return self.convert_curves_command(from, to);
        }
        if let Command::OffsetCurves {
            closed,
            offset,
            trio,
        } = cmd
        {
            return self.offset_curves_command(closed, offset, trio);
        }
        if let Command::BezierTangent(o) = cmd {
            return self.bezier_tangent_command(o);
        }
        if let Command::UvRectangle(options) = cmd {
            return self.uv_rectangle_command(options);
        }
        if let Command::TransferVertexMap {
            map,
            kind,
            world,
            sampling,
        } = cmd
        {
            return self.transfer_vertex_map_command(map, kind, world, sampling);
        }
        if let Command::AddEdgePoint { a, b, t } = cmd {
            return self.add_edge_point(a, b, t, &Default::default());
        }
        if let Command::AddEdgePointWith { a, b, t, options } = &cmd {
            return self.add_edge_point(*a, *b, *t, options);
        }
        if let Command::AddEdgeLoopWith { a, b, t, options } = &cmd {
            return self.add_edge_loop(*a, *b, *t, options);
        }
        if let Command::EdgeCommand(command) = cmd {
            return self.edge_command(command);
        }
        if let Command::SubdivideEdges { count, percent } = cmd {
            return self.subdivide_edges(count, percent);
        }
        if let Command::SubdividePolygons { options, times } = cmd {
            return self.subdivide_polygons(options, times);
        }
        if let Command::UnsubdividePolygons(options) = cmd {
            return self.unsubdivide(options);
        }
        if let Command::Detriangulate(options) = cmd {
            return self.detriangulate(options);
        }
        if let Command::UvUnwrap(options) = cmd {
            return self.uv_unwrap(options);
        }
        if let Command::UvAtlas(options) = cmd {
            return self.uv_atlas(options);
        }
        if let Command::UvRelax(options) = cmd {
            return self.uv_relax_once(options);
        }
        if let Command::PackUvs(options) = cmd {
            return self.pack_uvs(options);
        }
        if let Command::Remesh(op) = cmd {
            return self.remesh(op);
        }
        if let Command::SelectUvIslands(map) = cmd {
            return self.select_uv_islands(map);
        }
        if let Command::Boolean(options) = cmd {
            return self.boolean(options);
        }
        if let Command::SolidDrill(options) = cmd {
            return self.drill(crate::DrillKind::Solid, options);
        }
        if let Command::AxisDrill(options) = cmd {
            return self.drill(crate::DrillKind::Axis, options);
        }
        match cmd {
            Command::MeshOpCreate(t) => self.meshop_create(&t),
            Command::MeshOpDelete(ops) => self.meshop_delete(&ops),
            Command::MeshOpMove { op, position } => self.meshop_move(op, position),
            Command::MeshOpEnable { op, on } => self.meshop_enable(op, on),
            Command::MeshOpSet { op, values } => self.meshop_set(op, &values),
            Command::MeshOpSelect(row) => self.meshop_select(row),
            Command::MeshOpIndex { op, edit } => self.meshop_index(op, edit),
            Command::MeshOpSelOp { op, edit } => self.meshop_selop(op, edit),
            Command::MeshOpFreeze { op, duplicate } => self.meshop_freeze(op, duplicate),
            Command::MeshOpFreezeAll => self.meshop_freeze_all(),
            Command::MeshOpEditBase { item, on } => self.meshop_edit_base(item, on),
            cmd => self.dispatch_after_meshops(cmd, report),
        }
    }
    fn dispatch_after_meshops(
        &mut self,
        cmd: Command,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        if let Command::CenterSelection(axes) = cmd {
            return self.center_selection(axes);
        }
        if let Command::ResetTransform { items, part } = cmd {
            return self.reset_transform(items, part);
        }
        if let Command::FreezeTransform { items, part } = cmd {
            return self.freeze_transform(items, part);
        }
        if let Command::VertexCommand(command) = cmd {
            return self.vertex_command(command);
        }
        if let Command::NewVertex(world) = cmd {
            return self.new_vertex(world);
        }
        if let Command::WeldVertices { sources, target } = &cmd {
            return self.weld_vertices(sources.clone(), *target);
        }
        if let Command::WeldDrag { source, drop } = cmd {
            return self.weld_drag(source, drop);
        }
        if let Command::SculptStroke(edit) = cmd {
            return self.sculpt_stroke(*edit);
        }
        if let Command::WeightStroke(edit) = cmd {
            return self.weight_stroke(*edit);
        }
        if let Command::NewWeightMap { name, value } = cmd {
            return self.new_weight_map(name, value);
        }
        if let Command::SelectWeightMap(name) = cmd {
            // VM-02: tool context; a live tool under a Vertex Map falloff
            // re-applies with the new map.
            if name.as_ref().is_some_and(|n| n.trim().is_empty()) {
                return Err("A weight map needs a name".into());
            }
            let mut next = self.clone();
            next.selected_weight_map = name;
            if next.spatial_falloff.is_some()
                && let Some(l) = next.live.as_ref().filter(|l| l.applied)
            {
                next.dispatch(Command::Preview(l.params.clone()))?;
            }
            *self = next;
            return Ok(());
        }
        if let Command::SyncVertexOrder {
            source_item,
            source_faces,
        } = cmd
        {
            return self.sync_vertex_order(source_item, source_faces);
        }
        if let Command::AddEdgeLoop { a, b, t } = cmd {
            return self.add_edge_loop(a, b, t, &Default::default());
        }
        if let Command::SetPosition(assignment) = &cmd {
            return self.set_position(assignment.clone());
        }
        // Idle-only like Set Position: its preflight refuses live state.
        if let Command::AbsoluteScale(scale) = &cmd {
            return self.absolute_scale(*scale);
        }
        // SO-C03: idle-only like Set Position; its preflight refuses a Pen
        // draft, live stage or session instead of confirming or consuming it.
        if let Command::ScaleOffsetPolygon(distance) = &cmd {
            return self.scale_offset_polygon(*distance);
        }
        if let Command::ScaleOffsetDetach(distance) = &cmd {
            return self.scale_offset_detach(*distance);
        }
        // MAT-001: like Set Position, Set Material is an idle-only modal edit.
        // Route it before Pen, I2 and session dispatch so its own preflight
        // refuses a Pen draft, live stage or Reduction/Bridge session atomically
        // instead of confirming, dropping or consuming that unrelated state.
        if let Command::SetMaterial(assignment) = &cmd {
            return self.set_material(assignment.clone());
        }
        if let Command::ShaderTree(edit) = &cmd {
            return self.edit_shader_tree(edit.clone());
        }
        #[cfg(feature = "painting")]
        if let Command::Paint(edit) = &cmd {
            return self.edit_paint(edit.clone());
        }
        if let Command::ApplyEnvironmentPreset { preset, mode, name } = &cmd {
            return self
                .apply_environment_preset((**preset).clone(), *mode, name)
                .map(|_| ());
        }
        if let Command::LoadPreset { preset, group } = &cmd {
            return self.load_preset((**preset).clone(), *group);
        }
        if let Command::InsertMaterialPreset { preset, beside } = &cmd {
            return self.insert_material_preset((**preset).clone(), *beside);
        }
        if let Command::SetEnvironmentImage {
            path,
            width,
            height,
            item,
        } = &cmd
        {
            return self.set_environment_image(path, *width, *height, *item);
        }
        if let Command::ApplyMaterialPreset {
            preset,
            item,
            faces,
        } = &cmd
        {
            return self.apply_material_preset((**preset).clone(), *item, faces);
        }
        // One rule for picking a tool while another is open (Jason,
        // 2026-11-05): the same tool's button drops it; another tool drops
        // the open one first, keeping its geometry, then starts. Both run on
        // a staged copy, so a Drop that fails (a field error) or a start that
        // fails leaves the open tool, scene and history as they were.
        // Unit insertion while a tool is open drops it first, the same way:
        // its work is one step and the unit another, or nothing changes.
        if matches!(
            &cmd,
            Command::InsertUnit { .. }
                | Command::InsertEllipsoidUnit(_)
                | Command::InsertShapeUnit(..)
        ) && (self.has_active_task() || self.i2_active())
        {
            if self.selection_start.is_some() {
                return Err("Finish the selection gesture before unit insertion".into());
            }
            let mut staged = self.clone();
            staged.dispatch(Command::Drop)?;
            if staged.has_active_task() || staged.i2_active() {
                return Err("Finish the current tool before unit insertion".into());
            }
            staged.dispatch(cmd)?;
            *self = staged;
            return Ok(());
        }
        if let Some(switch) = self.tool_switch(&cmd) {
            return match switch {
                crate::active_task::ToolSwitch::Toggle => self.dispatch(Command::Drop),
                crate::active_task::ToolSwitch::Replace => {
                    let mut staged = self.clone();
                    staged.dispatch(Command::Drop)?;
                    if staged.has_active_task() || staged.i2_active() {
                        return Err("Finish the current tool before starting another".into());
                    }
                    staged.dispatch(cmd)?;
                    *self = staged;
                    Ok(())
                }
            };
        }
        // Clicking another item in the Items tree while Move, Rotate, Scale
        // or Transform is live keeps the tool and re-targets it (Jason,
        // 2026-11-08): the open tool's work is dropped as its own step, the
        // selection changes, and the same tool starts again on the new
        // selection with a fresh zero offset and its handle there. If the
        // tool can't start there (nothing selected, an empty mesh), the
        // selection still stands and the tool stays dropped, as after Undo
        // past a transform's activation. Runs on a staged copy so a Drop or
        // selection error leaves tool, scene and history as they were.
        // Activation may switch a camera/light/backdrop to Item mode, as
        // activating the tool by hand does.
        if matches!(
            &cmd,
            Command::SelectItem(_)
                | Command::SelectItems(_)
                | Command::ToggleItemSelection(_)
                | Command::SelectGroup(_)
        ) && self.preset.is_none()
            && self.selection_start.is_none()
            && let Some(live) = self.live.as_ref()
            && live.tool.transforms_items()
        {
            let tool = live.tool;
            return self.retarget_transform(tool, cmd);
        }
        // Mode has one shared, ordered structural resolver. It runs before
        // the Pen and session routes so an open tool (a Pen draft too) is
        // dropped first instead of refusing the mode change.
        if let Command::Mode(mode) = &cmd {
            return self.dispatch_mode(*mode);
        }
        if let Command::SetPolygonTagKind(kind) = cmd {
            if let Some(reason) = kind.unavailable_reason() {
                return Err(format!("{}: {reason}", kind.label()));
            }
            self.polygon_tag_kind = kind;
            self.scene.set_tag_kind(kind);
            self.revision = self.revision.wrapping_add(1);
            return Ok(());
        }
        if let Command::EnterTagMode { convert } = cmd {
            let kind = self.polygon_tag_kind;
            if let Some(reason) = kind.unavailable_reason() {
                return Err(format!("{}: {reason}", kind.label()));
            }
            // The shared mode route drops an open tool first.
            let mode = self.scene.selection.mode;
            self.dispatch_mode(Mode::Polygon)?;
            let before = self.scene.clone();
            // Convert reads the selection of the mode we came from.
            self.scene.selection.mode = mode;
            self.scene.enter_tag_mode(kind, convert)?;
            if convert && self.scene.selection.faces != before.selection.faces {
                self.record_selection("Convert to Tags".into(), before);
            }
            return Ok(());
        }
        if let Command::SolidSketchStrip = &cmd {
            return self.strip_solid_sketch();
        }
        if self.solid_sketch.is_some()
            || matches!(
                &cmd,
                Command::SolidSketchStart { .. } | Command::SolidSketch { .. }
            )
        {
            return self.solid_sketch_command(cmd, report);
        }
        if self.pen_draft.is_some()
            || matches!(
                &cmd,
                Command::PenStart
                    | Command::PenCapture(_)
                    | Command::PenCaptureEnd(_)
                    | Command::PenSelect(_)
                    | Command::PenRefine(_)
            )
        {
            let point_undo = self.pen_draft.is_some() && matches!(&cmd, Command::Undo);
            let cancel = self.pen_draft.is_some() && matches!(&cmd, Command::Cancel);
            let result = self.interactive_pen_command(cmd);
            if result.is_ok()
                && let Some(report) = report
            {
                if point_undo {
                    report.effect = crate::HistoryEffect::PenPointUndo;
                }
                if cancel {
                    report.effect = crate::HistoryEffect::Cancel;
                }
            }
            return result;
        }
        if matches!(&cmd, Command::FlipPolygons) {
            return self.flip_selected_polygons();
        }
        if let Command::FlipIt { preset, axis } = &cmd {
            return self.flip_it(preset, *axis);
        }
        if let Command::UvToolSpace(space) = &cmd {
            return self.uv_tool_space(space.clone());
        }
        if let Command::UvMap(edit) = &cmd {
            return self.uv_map_edit(edit.clone());
        }
        if let Command::UvIsland(op) = &cmd {
            return self.uv_island(*op);
        }
        if let Command::UvOp(op) = &cmd {
            return self.uv_op(op.clone());
        }
        if let Command::UvTexel(o) = &cmd {
            self.uv_texel = *o;
            return Ok(());
        }
        if let Command::UvSeam(edit) = &cmd {
            return self.uv_seam(edit.clone());
        }
        if let Command::UvSymmetry(s) = &cmd {
            s.validate()?;
            self.uv_symmetry = *s;
            // A live UV tool redoes its result with the new layer.
            if let Some(l) = self
                .live
                .as_ref()
                .filter(|l| l.applied && l.params.uv.is_some())
            {
                let p = l.params.clone();
                return self.dispatch(Command::Preview(p));
            }
            return Ok(());
        }
        if matches!(cmd, Command::UvSymmetryCenter) {
            return self.uv_symmetry_center();
        }
        if let Command::UvSymmetrize(o) = &cmd {
            return self.uv_symmetrize(*o);
        }
        if matches!(cmd, Command::UvToMesh) {
            return self.uv_to_mesh();
        }
        if matches!(cmd, Command::UvTransfer) {
            return self.uv_transfer();
        }
        if let Command::UvIslandPreset { preset, click } = &cmd {
            return self.uv_island_preset(preset, *click);
        }
        match &cmd {
            Command::ToggleSubdivision(toggle) => return self.toggle_subdivision(*toggle),
            Command::SetSubdivision(settings) => return self.set_subdivision(settings.clone()),
            Command::Freeze(options) => return self.freeze_mesh(options),
            Command::SetSmoothing {
                settings,
                continuing,
            } => return self.set_smoothing(settings.clone(), *continuing),
            Command::SetRenderSettings(settings) => {
                return self.set_render_settings(settings.clone());
            }
            Command::SetSceneTime(time) => return self.set_scene_time(*time),
            _ => {}
        }
        // MN-05: every tool session takes Reset Tool Attributes, so it runs
        // before the sessions' own input guards.
        if matches!(cmd, Command::ResetToolAttributes) {
            return self.reset_tool_attributes();
        }
        if self.bridge.is_some() && matches!(&cmd, Command::Pen(_)) {
            return Err("Drop or Cancel Bridge before Pen".into());
        }
        if let Command::Pen(input) = &cmd {
            return self.create_pen(input);
        }
        if self.i2_dispatch(&cmd, report)? {
            self.revision = self.revision.wrapping_add(1);
            return Ok(());
        }
        // RED-001: the Reduction session owns input until Drop or Cancel.
        // Picking another tool is handled by `tool_switch` above.
        if self.reduction.is_some()
            && !matches!(
                &cmd,
                Command::InitializeReduction
                    | Command::ConfigureReduction(_)
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(
                "Drop or Cancel Polygon Reduction before changing selection or context".into(),
            );
        }
        if self.bridge.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureBridge(_)
                    | Command::ResetBridge
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err("Drop or Cancel Bridge before changing selection or context".into());
        }
        if self.spike.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureSpike(_)
                    | Command::ResetSpike
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err("Drop or Cancel Spike before changing selection or context".into());
        }
        if self.ellipsoid.is_some()
            && !matches!(
                &cmd,
                Command::ConfigureEllipsoid(_)
                    | Command::ResetEllipsoid
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    // History refusal is owned by the shared policy below.
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(
                "Drop or Cancel numeric Ellipsoid before changing selection/context or history"
                    .into(),
            );
        }
        // Torus/Capsule/Arc/Tube own input like the numeric Ellipsoid.
        if let Some(active) = self.shape.as_ref().map(|s| s.kind())
            && !matches!(
                &cmd,
                Command::ConfigureShape(_)
                    | Command::ResetShape
                    | Command::Apply
                    | Command::Drop
                    | Command::Cancel
                    // History refusal is owned by the shared policy below.
                    | Command::Undo
                    | Command::Redo
            )
        {
            return Err(format!(
                "Drop or Cancel {} before changing selection/context or history",
                active.label()
            ));
        }
        // Retargeting resolves the current surface stage through the existing
        // atomic Drop contract before selection can own it.
        if self.live.as_ref().is_some_and(|l| l.tool.is_smooth_shift())
            && matches!(
                &cmd,
                Command::SelectItem(_)
                    | Command::Select { .. }
                    | Command::ClearSelection
                    | Command::Selection(_)
                    | Command::SelectionFill { .. }
                    | Command::SelectEdgeLoop { .. }
                    | Command::SelectConnectedPolygons { .. }
                    | Command::NewMesh
                    | Command::NewMeshBeside
                    | Command::AddItem(_)
                    | Command::ItemPattern { .. }
            )
        {
            let mut staged = self.clone();
            staged.dispatch(Command::Drop)?;
            staged.dispatch(cmd)?;
            *self = staged;
            return Ok(());
        }
        match cmd {
            Command::FlipPolygons | Command::FlipIt { .. } => {
                unreachable!("Flip is handled before session routing")
            }
            Command::UvToolSpace(_)
            | Command::UvMap(_)
            | Command::UvIsland(_)
            | Command::UvOp(_)
            | Command::UvTexel(_)
            | Command::UvSeam(_)
            | Command::UvSymmetry(_)
            | Command::UvSymmetryCenter
            | Command::UvSymmetrize(_)
            | Command::UvToMesh
            | Command::UvTransfer
            | Command::UvIslandPreset { .. } => {
                unreachable!("UV edits are handled before session routing")
            }
            Command::ToggleSubdivision(_)
            | Command::SetSubdivision(_)
            | Command::Freeze(_)
            | Command::SetSmoothing { .. }
            | Command::SetRenderSettings(_)
            | Command::SetSceneTime(_) => {
                unreachable!("Subdivision and smoothing are handled before session routing")
            }
            Command::Pen(_)
            | Command::PenStart
            | Command::PenCapture(_)
            | Command::PenCaptureEnd(_)
            | Command::PenSelect(_)
            | Command::PenRefine(_) => unreachable!("Pen is handled before session routing"),
            Command::SolidSketchStart { .. }
            | Command::SolidSketch { .. }
            | Command::SolidSketchStrip => {
                unreachable!("Solid Sketch is handled before session routing")
            }
            Command::BeginExtrude => unreachable!("handled by I2 boundary"),
            Command::StartExtrude => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Extrude && !l.applied)
                    .ok_or("Activate Extrude first")?;
                let mut p = live.params.clone();
                p.extent = DVec3::ZERO;
                self.preview(p, true)?;
            }
            cmd @ (Command::ConfigureSmoothShift(_)
            | Command::ResetSmoothShift
            | Command::CloneSmoothShift
            | Command::SurfaceStateSave(_)
            | Command::SurfaceStateRecall(_)
            | Command::SurfaceStateRename(_)
            | Command::SurfaceStateDelete) => self.dispatch_surface(cmd)?,
            Command::ArmBridge => self.arm_bridge()?,
            Command::ConfigureBridge(p) => self.configure_bridge(p)?,
            Command::ResetBridge => self.configure_bridge(Default::default())?,
            Command::ArmSpike => self.arm_spike()?,
            Command::ConfigureSpike(s) => self.configure_spike(s)?,
            Command::ResetSpike => self.configure_spike(0.)?,
            Command::ResetPrimitive => self.reset_primitive()?,
            Command::PrimitiveUvs(on) => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_primitive())
                    .ok_or("Make UVs needs a live Cube, Sphere or Cylinder")?;
                let params = Parameters {
                    make_uvs: on,
                    ..live.params.clone()
                };
                self.set_primitive_params(params)?;
            }
            Command::ArmReduction => self.arm_reduction()?,
            Command::InitializeReduction => self.initialize_reduction()?,
            Command::ConfigureReduction(p) => self.configure_reduction(p)?,
            Command::ArmEllipsoid => self.arm_ellipsoid()?,
            Command::ConfigureEllipsoid(p) => self.configure_ellipsoid(p)?,
            Command::ResetEllipsoid => {
                if self.live.is_some() || self.cone.is_some() {
                    return Err("Drop or Cancel the other tool before Ellipsoid Reset".into());
                }
                let mut staged = self.clone();
                staged.ellipsoid_preset = Default::default();
                if staged.ellipsoid.is_none() {
                    staged.arm_ellipsoid()?;
                }
                staged.configure_ellipsoid(Default::default())?;
                *self = staged;
            }
            Command::InsertEllipsoidUnit(destination) => self.insert_ellipsoid_unit(destination)?,
            Command::InsertShapeUnit(shape, destination) => {
                self.insert_shape_unit(shape, destination)?
            }
            Command::ArmShape(kind) => {
                if self.shape.is_some() {
                    return self.dispatch(Command::Drop);
                }
                if self.live.is_some() || self.cone.is_some() || self.ellipsoid.is_some() {
                    let mut staged = self.clone();
                    staged.dispatch(Command::Drop)?;
                    staged.dispatch(Command::ArmShape(kind))?;
                    *self = staged;
                    return Ok(());
                }
                self.arm_shape(kind)?;
            }
            Command::ConfigureShape(p) => self.configure_shape(p)?,
            Command::ResetShape => self.reset_shape()?,
            Command::ArmCone => self.arm_cone()?,
            Command::ConfigureCone(p) => self.configure_cone(p)?,
            Command::InsertUnit { tool, destination } => self.insert_unit(tool, destination)?,
            Command::ActionContext(context) => self.handle_action_context(context)?,
            Command::SetActionCenter(point)
                if self.live.as_ref().is_some_and(|l| l.tool == Tool::Bend) =>
            {
                self.set_bend_center(point, None)?;
            }
            Command::ToolClick { point, plane } => {
                if plane > 2 {
                    return Err("The work plane must be 0, 1 or 2".into());
                }
                match self.live.as_ref().map(|l| l.tool) {
                    Some(Tool::Bend) => self.set_bend_center(point, Some(plane))?,
                    Some(_) => return Err("This tool has no click setup".into()),
                    None => return Err("No active tool".into()),
                }
            }
            Command::SetActionCenter(point) => {
                let live = self.live.as_mut().ok_or("No active transform tool")?;
                // ACS-07: any tool that pivots through the pivot service
                // takes a placed centre (the transforms, and Smooth Shift
                // and Thicken, ACS-20; the polygon stage's side, ACS-21).
                if live.action_frame.is_none() {
                    return Err("This tool has no center to move".into());
                }
                if live.applied {
                    return Err("Drop the current transform before moving its center".into());
                }
                if !point.is_finite() || point.abs().max_element() > 1e9 {
                    return Err("The center must be a finite point".into());
                }
                let frame = live.action_frame.ok_or("Missing action frame")?;
                live.action_frame = Some(crate::ActionFrame {
                    center: point,
                    ..frame
                });
                live.placed_centre = Some(point);
            }
            Command::ResetActionCenter => {
                let live = self.live.as_ref().ok_or("No active transform tool")?;
                let frame = live.action_frame.ok_or("This tool has no center to move")?;
                // Resolved against the tool's base, so after a drag the
                // handle (center + delta) still sits on the moved selection.
                let own = self.pivot_frame(&live.base, live.tool)?;
                if let Some(live) = &mut self.live {
                    live.action_frame = Some(crate::ActionFrame {
                        center: own.center,
                        ..frame
                    });
                    live.placed_centre = None;
                }
            }
            Command::CommitTransformStep => self.commit_transform_step()?,
            Command::ContinueTransformStep => self.continue_transform_step()?,
            Command::DuplicateSelectedPolygons => self.duplicate_selected_polygons()?,
            Command::ConfigureChamfer(parameters) => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Chamfer)
                    .ok_or("Activate Chamfer Edit first")?;
                let mut p = live.params.clone();
                p.chamfer = parameters;
                self.preview(p, false)?;
                self.chamfer_preset = parameters;
            }
            Command::ChamferOnce(parameters) => {
                if self.live.is_some() {
                    return Err("Finish or Cancel the active tool first".into());
                }
                let mut staged = self.clone();
                staged.dispatch(Command::Activate(Tool::Chamfer))?;
                staged.dispatch(Command::ConfigureChamfer(parameters))?;
                staged.dispatch(Command::Drop)?;
                *self = staged;
            }
            Command::PolyHaulOnce(offset) => {
                if self.live.is_some() {
                    return Err("Finish or Cancel the active tool first".into());
                }
                let mut staged = self.clone();
                staged.dispatch(Command::Activate(Tool::PolyHaul))?;
                let mut params = staged.live.as_ref().unwrap().params.clone();
                params.poly_haul_offset = offset;
                staged.dispatch(Command::Preview(params))?;
                staged.dispatch(Command::Drop)?;
                *self = staged;
            }
            Command::SketchExtrudeOnce(path) => self.handle_sketch_extrude_once(path)?,
            cmd @ (Command::ConfigureBevel(_)
            | Command::ResetBevel
            | Command::BevelStateSave(_)
            | Command::BevelStateRecall(_)
            | Command::BevelStateRename(_)
            | Command::BevelStateDelete
            | Command::ResetBevelState
            | Command::BevelProfile(_)
            | Command::SpatialFalloff(_)
            | Command::AddedFalloffs(_)
            | Command::SpatialFalloffMix(_)
            | Command::FalloffElementPick(_)
            | Command::FalloffStamps(_)
            | Command::FalloffPath(_)) => {
                self.dispatch_bevel(cmd)?;
            }
            Command::ConfigureExtrude(p) => self.handle_configure_extrude(p)?,
            Command::ResetPolyHaul => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::PolyHaul)
                    .ok_or("Activate Poly Haul first")?;
                let mut p = live.params.clone();
                p.poly_haul_offset = 0.;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetSketchExtrude => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::SketchExtrude)
                    .ok_or("Activate Sketch Extrude first")?;
                let mut p = live.params.clone();
                p.sketch_precision = 30.;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetBend => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::Bend)
                    .ok_or("Activate Bend first")?;
                let mut p = live.params.clone();
                p.bend = crate::bend::reset(p.bend, self.bend_axes(&self.scene));
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetVertexMerge => {
                let live = self
                    .live
                    .as_ref()
                    .filter(|l| l.tool == Tool::VertexMerge)
                    .ok_or("Activate the Merge Tool first")?;
                let mut p = live.params.clone();
                p.shift = MERGE_TOOL_DISTANCE;
                self.dispatch(Command::Preview(p))?;
            }
            Command::ResetExtrude => {
                let mut p = Parameters::default();
                if let Some(l) = &self.live {
                    p.extrude_uv_map = l.params.extrude_uv_map.clone();
                    p.extrude_uv_selected = l.params.extrude_uv_selected;
                }
                // Reset is atomic and can restore U while setters are unavailable.
                let mut staged = self.clone();
                if let Some(l) = &mut staged.live {
                    l.params.extrude_uvs = p.extrude_uvs;
                }
                staged.dispatch(Command::ConfigureExtrude(p))?;
                *self = staged;
            }
            // BN-01 / G3: every way of starting Bend (button, Ctrl+E, the
            // command line, scripts) goes through its tool-presets row, so
            // Automatic centre and Auto axis are in place for the life of
            // the tool and go back on drop (TP-12).
            Command::Activate(Tool::Bend)
                if !self.preset.as_ref().is_some_and(|p| p.tool == Tool::Bend) =>
            {
                self.activate_preset(crate::tool_preset::BEND_ROW)?;
                // BN-47: a remembered Angle shows on the mesh at once, so
                // the panel and the preview agree.
                if let Some(l) = self.live.as_ref().filter(|l| l.tool == Tool::Bend)
                    && l.params.bend.angle != 0.
                {
                    let p = l.params.clone();
                    self.dispatch(Command::Preview(p))?;
                }
            }
            Command::Activate(t) => {
                self.close_copier_row_for(t);
                let after = self.handle_activate(t)?;
                // Section 8: warn when the points the tool edits lack
                // mirror partners.
                self.note_unpaired_points();
                if let AfterCommand::Done = after {
                    return Ok(());
                }
            }
            Command::ActivateDeform(preset) => self.activate_preset(preset.id())?,
            Command::ActivatePreset(name) => self.activate_preset(&name)?,
            Command::Preview(p) => {
                if let Err(error) = self.preview(p, false) {
                    if let Some(live) = self.live.as_mut()
                        && (matches!(live.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                            || live.tool.is_smooth_shift())
                    {
                        live.preview_error = Some(error.clone());
                    }
                    return Err(error);
                }
            }
            Command::ToolItemChannel { item, edit } => self.tool_item_channel(item, &edit)?,
            Command::Continue => self.handle_continue()?,
            Command::Apply => {
                if let AfterCommand::Done = self.handle_apply()? {
                    return Ok(());
                }
            }
            Command::Drop => {
                if let AfterCommand::Done = self.handle_drop()? {
                    return Ok(());
                }
            }
            Command::Cancel => self.handle_cancel(report)?,
            Command::Undo => self.handle_undo(report)?,
            Command::Redo => self.handle_redo(report)?,
            Command::Mode(_) => unreachable!("handled by Mode route"),
            Command::BeginSelection => {
                self.require_idle()?;
                if self.selection_start.is_none() {
                    self.selection_start = Some(Box::new(self.scene.clone()));
                }
            }
            Command::EndSelection { cancel } => {
                let partners = std::mem::take(&mut self.gesture_partners);
                if !cancel {
                    self.scene.partners_last(&partners);
                }
                if let Some(before) = self.selection_start.take() {
                    if cancel {
                        self.scene.selection = before.selection;
                    } else if self.scene != *before {
                        self.record_selection("Selection".into(), *before);
                    }
                }
            }
            Command::Select {
                item,
                hit,
                replace,
                remove,
            } => {
                self.require_idle()?;
                let before = self.selection_start.is_none().then(|| self.scene.clone());
                self.scene.select_component(item, hit, replace, remove)?;
                // SYM-B2-10: with Symmetry on, a pick also selects the mirror
                // counterpart (measured for polygons; points and edges, and
                // removing, follow the same rule provisionally).
                if let Some(mirror) =
                    self.scene
                        .symmetric_counterpart(item, hit, self.active_symmetry())
                    && mirror != hit
                {
                    self.scene.select_component(item, mirror, false, remove)?;
                    if self.selection_start.is_some() && !remove {
                        self.gesture_partners.push((item, mirror));
                    }
                }
                if let Some(before) = before
                    && self.scene != before
                {
                    self.record_selection("Selection".into(), before);
                }
            }
            Command::SelectPolygonSet(name) => self.handle_select_polygon_set(name)?,
            Command::AssignSelectionSet(name) => self.assign_selection_set(name)?,
            Command::UseSelectionSet { name, mode } => self.use_selection_set(name, mode)?,
            Command::DeleteSelectionSet(name) => self.delete_selection_set(name)?,
            Command::CreateSelectionSetGroup { name, sets } => {
                self.create_selection_set_group(name, sets)?
            }
            Command::DeleteSelectionSetGroup(name) => self.delete_selection_set_group(name)?,
            Command::SetPart(assignment) => self.set_part(assignment)?,
            Command::QuadruplePolygon => unreachable!("Quadruple is handled before task routing"),
            Command::SpinQuads => unreachable!("Spin Quad is handled before task routing"),
            Command::AlignPolygons => unreachable!("Align is handled before task routing"),
            Command::MakePolygon | Command::MakePolygonAs { .. } => {
                unreachable!("Make is handled before task routing")
            }
            Command::MakeCurve { .. }
            | Command::MakeCurveFill
            | Command::AlignWinding(_)
            | Command::UvRectangle(_)
            | Command::ConvertCurves { .. }
            | Command::CurveBoolean(_)
            | Command::OffsetCurves { .. }
            | Command::BezierTangent(_)
            | Command::Dicer(_)
            | Command::TransferVertexMap { .. } => {
                unreachable!("curve commands are handled before task routing")
            }
            Command::AddEdgePoint { .. }
            | Command::AddEdgeLoop { .. }
            | Command::AddEdgePointWith { .. }
            | Command::AddEdgeLoopWith { .. }
            | Command::EdgeCommand(_)
            | Command::SubdivideEdges { .. }
            | Command::SubdividePolygons { .. }
            | Command::UnsubdividePolygons(_)
            | Command::Detriangulate(_)
            | Command::UvUnwrap(_)
            | Command::UvAtlas(_)
            | Command::UvRelax(_)
            | Command::PackUvs(_)
            | Command::Remesh(_)
            | Command::SelectUvIslands(_)
            | Command::Boolean(_)
            | Command::MeshOpCreate(_)
            | Command::MeshOpDelete(_)
            | Command::MeshOpMove { .. }
            | Command::MeshOpEnable { .. }
            | Command::MeshOpSet { .. }
            | Command::MeshOpSelect(_)
            | Command::MeshOpIndex { .. }
            | Command::MeshOpSelOp { .. }
            | Command::MeshOpFreeze { .. }
            | Command::MeshOpFreezeAll
            | Command::MeshOpEditBase { .. }
            | Command::SolidDrill(_)
            | Command::AxisDrill(_)
            | Command::CenterSelection(_) => {
                unreachable!("Edge commands are handled before task routing")
            }
            Command::FlattenNgons => unreachable!("Flatten Ngons is handled before task routing"),
            Command::CleanupFloating(_) | Command::MeshCleanup { .. } => {
                unreachable!("Mesh Cleanup is handled before task routing")
            }
            Command::UnifyPolygons(_) => unreachable!("Unify is handled before task routing"),
            Command::SplitPolygon => unreachable!("Split is handled before task routing"),
            Command::SetPosition(_) => unreachable!("Set Position is handled before task routing"),
            Command::AbsoluteScale(_) => {
                unreachable!("Absolute Scaling is handled before task routing")
            }
            Command::ScaleOffsetPolygon(_) | Command::ScaleOffsetDetach(_) => {
                unreachable!("Scale Offset is handled before task routing")
            }
            Command::SetMaterial(_) => unreachable!("Set Material is handled before task routing"),
            Command::ShaderTree(_) => {
                unreachable!("Shader tree edits are handled before task routing")
            }
            Command::LoadPreset { .. } | Command::InsertMaterialPreset { .. } => {
                unreachable!("Preset loads are handled before task routing")
            }
            Command::SetEnvironmentImage { .. } | Command::ApplyEnvironmentPreset { .. } => {
                unreachable!("Environment images and presets are set before task routing")
            }
            Command::ApplyMaterialPreset { .. } => {
                unreachable!("Material drops are handled before task routing")
            }
            Command::Triple => self.triple()?,
            Command::RemovePolygons => self.remove_polygons()?,
            Command::DeleteComponents => self.delete_components()?,
            Command::MergePolygons => self.merge_polygons()?,
            Command::CollapsePolygon => self.collapse_polygon()?,
            Command::Selection(operation) => {
                self.require_idle()?;
                let before = self.scene.clone();
                // SYM-LOOP-01..04: with Symmetry on, seed-reading commands
                // count only the picked side, then add the mirror side.
                let symmetry = self.active_symmetry();
                self.scene
                    .on_active_meshes(|s| s.selection_operation_symmetric(operation, symmetry))?;
                if self.scene != before {
                    self.record_selection(operation.label().into(), before);
                }
            }
            Command::ToggleSymmetry => self.toggle_symmetry()?,
            Command::Symmetry(symmetry) => self.set_symmetry(symmetry)?,
            Command::SelectSymmetrical => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_symmetrical(self.active_symmetry())?;
                if self.scene != before {
                    self.record_selection("Select Symmetrical".into(), before);
                }
            }
            Command::SymmetrizePolygons { side, merge } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene
                    .symmetrize_polygons(self.active_symmetry(), side, merge)?;
                if self.scene != before {
                    self.record("Symmetrize".into(), before);
                }
            }
            Command::Symmetrize(side) => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.symmetrize(self.active_symmetry(), side)?;
                if self.scene != before {
                    self.record("Symmetrize".into(), before);
                }
            }
            Command::SelectionFill { item, face } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.fill_polygon_selection(item, face)?;
                // SYM3-30: the mirror region is filled too.
                if let Some(crate::Component::Polygon(g)) =
                    self.mirror_pick(item, crate::Component::Polygon(face))
                    && !self.scene.selection.faces.contains(&g)
                {
                    self.scene.fill_polygon_selection(item, g)?;
                }
                if self.scene != before {
                    self.record_selection("Selection Fill".into(), before);
                }
            }
            Command::SelectEdgeLoop { item, a, b, add } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_edge_loop(item, (a, b), add)?;
                // SYM3-30: the mirror loop is added too.
                if let Some(crate::Component::Edge(x, y)) =
                    self.mirror_pick(item, crate::Component::Edge(a, b))
                    && !self
                        .scene
                        .selection
                        .edges
                        .iter()
                        .any(|&(p, q)| (p.min(q), p.max(q)) == (x.min(y), x.max(y)))
                {
                    self.scene.select_edge_loop(item, (x, y), true)?;
                }
                if self.scene != before {
                    self.record_selection("Select Loop".into(), before);
                }
            }
            Command::SelectConnectedPolygons { item, face, add } => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.select_connected_polygons(item, face, add)?;
                // SYM3-30: the mirror piece is added too.
                if let Some(crate::Component::Polygon(g)) =
                    self.mirror_pick(item, crate::Component::Polygon(face))
                    && !self.scene.selection.faces.contains(&g)
                {
                    self.scene.select_connected_polygons(item, g, true)?;
                }
                if self.scene != before {
                    self.record_selection("Select Connected".into(), before);
                }
            }
            Command::ClearSelection => {
                self.require_idle()?;
                let before = self.scene.clone();
                self.scene.clear_components();
                if self.selection_start.is_none() && self.scene != before {
                    self.record_selection("Clear Selection".into(), before);
                }
            }
            Command::Workplane(plane) => {
                self.require_idle()?;
                if plane.plane > 2 || !plane.center.is_finite() {
                    return Err("Invalid construction plane".into());
                }
                self.scene.workplane = plane;
            }
            Command::SelectItem(id) => self.handle_select_item(id)?,
            Command::SelectGroup(group) => {
                self.require_idle()?;
                let members = self.scene.group_members(group);
                if self.scene.group(group).is_none() {
                    return Err("Missing group".into());
                }
                self.scene.select_item(members.first().copied());
                self.scene.selection.also = members.iter().skip(1).copied().collect();
            }
            Command::SelectItems(ids) => {
                self.require_idle()?;
                if ids.iter().any(|id| !self.scene.item(*id).is_some()) {
                    return Err("Missing item".into());
                }
                let mut ids = ids;
                let mut seen = std::collections::HashSet::new();
                ids.retain(|id| seen.insert(*id));
                self.scene.select_item(ids.first().copied());
                self.scene.selection.also = ids.into_iter().skip(1).collect();
            }
            Command::ToggleItemSelection(id) => {
                self.require_idle()?;
                if !self.scene.item(id).is_some() {
                    return Err("Missing item".into());
                }
                self.scene.toggle_item(id);
            }
            cmd => {
                self.require_idle()?;
                let before = self.scene.clone();
                let label = match cmd {
                    Command::NewMesh | Command::NewMeshBeside => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        // Item list New: the selected item's parent (NM08).
                        let parent = match cmd {
                            Command::NewMeshBeside => self
                                .scene
                                .selection
                                .item
                                .and_then(|id| self.scene.item(id))
                                .and_then(|i| i.parent),
                            _ => None,
                        };
                        self.scene.next_id = next;
                        self.scene.new_mesh()?;
                        if let Some(item) = self.scene.items.last_mut() {
                            item.parent = parent;
                        }
                        self.next_item_id = self.scene.next_id;
                        "New Mesh"
                    }
                    Command::NewBackdrop => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene
                            .new_backdrop(crate::backdrop::Backdrop::new_default())?;
                        self.next_item_id = self.scene.next_id;
                        "New Backdrop"
                    }
                    Command::BackdropLoadImage {
                        item,
                        path,
                        width,
                        height,
                    } => {
                        backdrop_mut(&mut self.scene, item)?;
                        let image = self.scene.image_for_path(&path, width, height)?;
                        backdrop_mut(&mut self.scene, item)?.image = Some(image);
                        "Backdrop Image"
                    }
                    Command::EditBackdrop { item, edit } => {
                        if let crate::backdrop::BackdropEdit::Image(Some(image)) = &edit
                            && self.scene.image(*image).is_none()
                        {
                            return Err("Choose a loaded image".into());
                        }
                        backdrop_mut(&mut self.scene, item)?.apply(edit)?;
                        "Backdrop Setting"
                    }
                    Command::BackdropAutoFit(item) => {
                        self.scene.refuse_locked(&[item])?;
                        // BD-61: the active meshes' combined box.
                        let meshes = self.scene.auto_fit_meshes();
                        let bounds = self
                            .scene
                            .mesh_bounds(&meshes)
                            .ok_or(crate::backdrop::AUTO_FIT_NO_MESH)?;
                        self.scene.auto_fit_backdrop(item, bounds)?;
                        "Auto Fit"
                    }
                    Command::AddBackdrop {
                        name,
                        image,
                        projection,
                    } => {
                        if name.as_deref().is_some_and(|n| n.trim().is_empty()) {
                            return Err("A Backdrop name can't be empty".into());
                        }
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        let image = match image {
                            Some((path, width, height)) => {
                                Some(self.scene.image_for_path(&path, width, height)?)
                            }
                            None => None,
                        };
                        let settings = crate::backdrop::Backdrop {
                            image,
                            projection,
                            ..crate::backdrop::Backdrop::new_default()
                        };
                        self.scene.new_backdrop(settings)?;
                        if let (Some(name), Some(item)) = (name, self.scene.items.last_mut()) {
                            item.name = name;
                        }
                        self.next_item_id = self.scene.next_id;
                        "Add Backdrop"
                    }
                    Command::LoadImage {
                        path,
                        width,
                        height,
                    } => {
                        self.scene.image_for_path(&path, width, height)?;
                        "Load Image"
                    }
                    Command::BackdropSize {
                        item,
                        width,
                        height,
                    } => {
                        self.scene.refuse_locked(&[item])?;
                        self.scene.size_backdrop(item, width, height)?;
                        "Backdrop Size"
                    }
                    Command::AddItem(kind) => {
                        use crate::item_list::AddItemKind;
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        self.scene.next_id = next;
                        let anchor = self.scene.selection.item;
                        let render_camera = self.scene.render_camera();
                        match kind {
                            AddItemKind::Mesh => self.scene.new_mesh()?,
                            AddItemKind::Backdrop => {
                                self.scene.new_backdrop(Default::default())?;
                            }
                            AddItemKind::Camera => {
                                self.scene
                                    .add_camera(Default::default(), Default::default())?;
                            }
                            AddItemKind::Light(light) => {
                                self.scene.add_light(
                                    crate::camera_light::LightSettings::new(light),
                                    Default::default(),
                                )?;
                            }
                            AddItemKind::GroupLocator => {
                                self.scene.add_group_locator()?;
                            }
                            AddItemKind::Environment => {
                                self.scene.add_environment()?;
                            }
                        }
                        let id = self.scene.items.last().map(|i| i.id);
                        if kind != AddItemKind::Environment {
                            self.scene.place_new_item(anchor, render_camera);
                        }
                        self.scene.select_item(id);
                        self.next_item_id = self.scene.next_id;
                        // The same step names as the older New commands.
                        match kind {
                            AddItemKind::Mesh => "New Mesh",
                            AddItemKind::Backdrop => "New Backdrop",
                            AddItemKind::Camera => "New Camera",
                            AddItemKind::Light(_) => "New Light",
                            AddItemKind::GroupLocator => "New Group Locator",
                            AddItemKind::Environment => "New Environment",
                        }
                    }
                    Command::ItemPattern { query, action } => {
                        use crate::item_list::PatternAction;
                        let matches = self.scene.pattern_matches(&query);
                        if matches.is_empty() {
                            return Err("No items match the pattern".into());
                        }
                        match action {
                            PatternAction::Select => {
                                self.scene.select_item(Some(matches[0]));
                                self.scene.selection.also = matches[1..].to_vec();
                            }
                            PatternAction::Unselect => {
                                let before = self.scene.selected_items();
                                let rest: Vec<u64> = before
                                    .iter()
                                    .copied()
                                    .filter(|id| !matches.contains(id))
                                    .collect();
                                if rest.len() == before.len() {
                                    return Err("No selected items match the pattern".into());
                                }
                                if rest.first() != self.scene.selection.item.as_ref() {
                                    // The active item went: the next one left
                                    // becomes active, components cleared.
                                    self.scene.select_item(rest.first().copied());
                                }
                                self.scene.selection.also = rest.iter().skip(1).copied().collect();
                            }
                            PatternAction::Hide => {
                                let mut changed = false;
                                for item in &mut self.scene.items {
                                    if item.visible && matches.contains(&item.id) {
                                        item.visible = false;
                                        changed = true;
                                    }
                                }
                                if !changed {
                                    return Err("The matching items are hidden already".into());
                                }
                            }
                        }
                        match action {
                            PatternAction::Select => "Select Items",
                            PatternAction::Unselect => "Unselect Items",
                            PatternAction::Hide => "Hide Items",
                        }
                    }
                    Command::EditEnvironmentLayers { item, edit } => {
                        let label = edit.label();
                        let mut next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.edit_environment_layers(item, edit)?;
                        next = self.scene.next_id;
                        self.next_item_id = next;
                        label
                    }
                    Command::EditEnvironment { item, edit } => {
                        if let crate::environment::EnvironmentEdit::SunLight(Some(sun)) = &edit {
                            let directional = self.scene.items.iter().any(|i| {
                                i.id == *sun
                                    && i.light.as_ref().is_some_and(|l| {
                                        l.kind == crate::camera_light::LightKind::Directional
                                    })
                            });
                            if !directional {
                                return Err("Sun Light must be a directional light".into());
                            }
                        }
                        let env = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.environment.as_mut())
                            .ok_or("Select an environment")?;
                        edit.apply(env)?;
                        edit.label()
                    }
                    Command::NewCamera | Command::NewLight(_) => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        let id = match cmd {
                            Command::NewLight(kind) => self.scene.add_light(
                                crate::camera_light::LightSettings::new(kind),
                                Default::default(),
                            )?,
                            _ => self
                                .scene
                                .add_camera(Default::default(), Default::default())?,
                        };
                        self.next_item_id = self.scene.next_id;
                        self.scene.select_item(Some(id));
                        if matches!(cmd, Command::NewCamera) {
                            "New Camera"
                        } else {
                            "New Light"
                        }
                    }
                    Command::EditCamera { item, edit } => {
                        let label = edit.label();
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.camera.is_some())
                        {
                            return Err("Select a camera".into());
                        }
                        if let crate::camera_light::CameraEdit::BackgroundImage(Some(image)) = &edit
                            && self.scene.image(*image).is_none()
                        {
                            return Err("Choose a loaded image".into());
                        }
                        // CP-X3: the other selected cameras follow; all or
                        // nothing.
                        let mut targets = vec![item];
                        if self.scene.selection.item == Some(item) {
                            targets.extend(self.scene.selection.also.iter().copied());
                        }
                        let mut next = Vec::new();
                        for (index, target) in self.scene.items.iter().enumerate() {
                            if let Some(camera) = target
                                .camera
                                .as_ref()
                                .filter(|_| targets.contains(&target.id))
                            {
                                let mut camera = camera.clone();
                                camera.apply(edit.clone())?;
                                next.push((index, camera));
                            }
                        }
                        for (index, camera) in next {
                            self.scene.items[index].camera = Some(camera);
                        }
                        label
                    }
                    Command::CameraAutofocus(item) => {
                        let d = self.scene.autofocus_distance(item)?;
                        self.scene
                            .item_mut(item)
                            .and_then(|i| i.camera.as_mut())
                            .ok_or("Select a camera")?
                            .apply(crate::camera_light::CameraEdit::FocusDistance(d))?;
                        "Autofocus"
                    }
                    Command::CameraBackgroundLoad {
                        item,
                        path,
                        width,
                        height,
                    } => {
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.camera.is_some())
                        {
                            return Err("Select a camera".into());
                        }
                        let image = self.scene.image_for_path(&path, width, height)?;
                        let camera = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.camera.as_mut())
                            .ok_or("Select a camera")?;
                        camera.background.image = Some(image);
                        camera.background.enable = true;
                        "Background Image"
                    }
                    Command::SyncCameraToView {
                        item,
                        transform,
                        target_distance,
                    } => {
                        self.scene.refuse_locked(&[item])?;
                        let target = self
                            .scene
                            .items
                            .iter_mut()
                            .find(|i| i.id == item && i.camera.is_some())
                            .ok_or("Select a camera")?;
                        target.transform = transform;
                        target.camera.as_mut().ok_or("Select a camera")?.apply(
                            crate::camera_light::CameraEdit::TargetDistance(target_distance),
                        )?;
                        "Sync to View"
                    }
                    Command::EditLight { item, edit } => {
                        let label = edit.label();
                        if !self
                            .scene
                            .items
                            .iter()
                            .any(|i| i.id == item && i.light.is_some())
                        {
                            return Err("Select a light".into());
                        }
                        // LP-31: an Item Group is one of the scene's groups.
                        if let crate::camera_light::LightEdit::LinkGroup(Some(g)) = &edit
                            && self.scene.group(*g).is_none()
                        {
                            return Err("Choose one of the scene's groups".into());
                        }
                        // LP-MS-01: a prototype is a mesh item.
                        if let crate::camera_light::LightEdit::Prototype(Some(p)) = &edit
                            && !self.scene.items.iter().any(|i| i.id == *p && i.is_mesh())
                        {
                            return Err("Choose a mesh item as the prototype".into());
                        }
                        // LP-51: the other selected lights follow; all or
                        // nothing.
                        let mut targets = vec![item];
                        if self.scene.selection.item == Some(item) {
                            targets.extend(self.scene.selection.also.iter().copied());
                        }
                        let mut next = Vec::new();
                        for (index, target) in self.scene.items.iter().enumerate() {
                            if let Some(light) = target
                                .light
                                .as_ref()
                                .filter(|_| targets.contains(&target.id))
                            {
                                let mut light = light.clone();
                                light.apply(edit.clone())?;
                                next.push((index, light));
                            }
                        }
                        for (index, light) in next {
                            self.scene.items[index].light = Some(light);
                        }
                        label
                    }
                    Command::NewGroup { name, members } => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.new_group(name.as_deref(), &members)?;
                        self.next_item_id = self.scene.next_id;
                        "New Group"
                    }
                    Command::RenameGroup { group, name } => {
                        self.scene.rename_group(group, &name)?;
                        "Rename Group"
                    }
                    Command::DeleteGroup(group) => {
                        self.scene.delete_group(group)?;
                        "Delete Group"
                    }
                    Command::AddToGroup { group, items } => {
                        self.scene.add_to_group(group, &items)?;
                        "Add to Group"
                    }
                    Command::RemoveFromGroup { group, items } => {
                        self.scene.remove_from_group(group, &items)?;
                        "Remove from Group"
                    }
                    Command::LoadPhotometry { item, path } => {
                        let file =
                            crate::photometry::Photometry::load(std::path::Path::new(&path))?;
                        let target = self
                            .scene
                            .item_mut(item)
                            .and_then(|i| i.light.as_mut())
                            .filter(|l| l.kind == crate::camera_light::LightKind::Photometric)
                            .ok_or("Select a photometric light")?;
                        target.apply(crate::camera_light::LightEdit::Photometry {
                            path,
                            intensity: file.radiant_intensity(),
                            width: file.width,
                            height: file.length,
                        })?;
                        "Photometry File"
                    }
                    Command::EditItemCommon { items, edit } => {
                        self.scene.apply_item_common(&items, &edit)?;
                        edit.label()
                    }
                    Command::SetRenderCamera(item) => {
                        self.scene.set_render_camera(item)?;
                        "Render Camera"
                    }
                    Command::CameraView {
                        item,
                        transform,
                        target_distance,
                    } => {
                        let target = self
                            .scene
                            .items
                            .iter_mut()
                            .find(|i| i.id == item && (i.camera.is_some() || i.light.is_some()))
                            .ok_or("Select a camera or light")?;
                        if target.is_locked() {
                            return Err(format!("{} is locked (Assembly > Lock)", target.name));
                        }
                        // Check the Target Distance before moving anything, so
                        // an error leaves the item where it was.
                        let camera = match target.camera.clone() {
                            Some(mut c) => {
                                c.apply(crate::camera_light::CameraEdit::TargetDistance(
                                    target_distance,
                                ))?;
                                Some(c)
                            }
                            None => None,
                        };
                        target.transform = transform;
                        match camera {
                            Some(c) => {
                                target.camera = Some(c);
                                "Camera View"
                            }
                            None => "Light View",
                        }
                    }
                    Command::DeleteItem => {
                        let id = self
                            .scene
                            .selection
                            .item
                            .ok_or("Select an item to delete")?;
                        let locators = self.scene.environment_locators(&[id]);
                        if self.scene.any_has_children(&[id]) {
                            // Scripts' default (provisional, GL-Q6).
                            self.scene.delete_items(&[id], Default::default())?;
                        } else {
                            self.scene.items.retain(|i| i.id != id);
                            self.scene.select_item(None);
                        }
                        self.scene.drop_environment_locators(&locators);
                        "Delete Mesh"
                    }
                    Command::DeleteItems(children) => {
                        let ids = self.scene.selected_items();
                        let locators = self.scene.environment_locators(&ids);
                        self.scene.delete_items(&ids, children)?;
                        self.scene.drop_environment_locators(&locators);
                        "Delete Item"
                    }
                    Command::InstanceItems => {
                        crate::duplicate_items::instance_selected(
                            &mut self.scene,
                            self.next_item_id,
                        )?;
                        self.next_item_id = self.scene.next_id;
                        "Instance"
                    }
                    Command::DuplicateHierarchy => {
                        self.duplicate_hierarchy()?;
                        "Duplicate Hierarchy"
                    }
                    Command::DuplicateItem => {
                        let id = self
                            .scene
                            .selection
                            .item
                            .ok_or("Select an item to duplicate")?;
                        let next = self.scene.next_id.max(self.next_item_id);
                        if next >= u64::MAX - 1 || self.scene.items.len() >= crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        let source = self.scene.item(id).ok_or("Select an item to duplicate")?;
                        let mut copy = source.clone();
                        let base = source.name.clone();
                        let name = (2..)
                            .map(|n| format!("{base} ({n})"))
                            .find(|name| self.scene.items.iter().all(|i| &i.name != name))
                            .unwrap_or(base);
                        copy.id = next;
                        copy.name = name;
                        copy.source_id = None;
                        copy.source_display_name = None;
                        // ENV §6: a copied environment gets its own layer
                        // ids and Environment Material name.
                        if let Some(env) = copy.environment.as_mut() {
                            self.scene.renumber_environment(env)?;
                        }
                        self.scene.items.push(copy);
                        self.scene.next_id = next + 1;
                        self.next_item_id = self.scene.next_id;
                        self.scene.select_item(Some(next));
                        "Duplicate Item"
                    }
                    Command::InsertItems {
                        mut items,
                        parent,
                        images,
                    } => {
                        let next = self.scene.next_id.max(self.next_item_id);
                        if items.is_empty() {
                            return Err("Nothing to insert".into());
                        }
                        if next >= u64::MAX - items.len() as u64
                            || self.scene.items.len() + items.len() > crate::MAX_SCENE_ITEMS
                        {
                            return Err("Scene item/identity budget exhausted".into());
                        }
                        if parent.is_some_and(|p| self.scene.items.iter().all(|i| i.id != p)) {
                            return Err("Choose an existing parent item".into());
                        }
                        // Backdrops keep their pictures: the same file is the
                        // same entry here.
                        let mut scene = self.scene.clone();
                        for item in &mut items {
                            let Some(b) = item.backdrop.as_mut() else {
                                continue;
                            };
                            b.image = match b.image.and_then(|i| images.iter().find(|s| s.id == i))
                            {
                                Some(source) => Some(scene.image_for_path(
                                    &source.path,
                                    source.width,
                                    source.height,
                                )?),
                                None => None,
                            };
                        }
                        self.scene.images = scene.images;
                        let ids: std::collections::BTreeMap<u64, u64> = items
                            .iter()
                            .enumerate()
                            .map(|(k, item)| (item.id, next + k as u64))
                            .collect();
                        let root = ids[&items[0].id];
                        for item in items {
                            let mut copy = item.clone();
                            copy.id = ids[&item.id];
                            copy.parent = item.parent.and_then(|p| ids.get(&p).copied());
                            copy.instance_of = item.instance_of.and_then(|p| ids.get(&p).copied());
                            let base = item.name.clone();
                            if self.scene.items.iter().any(|i| i.name == base) {
                                copy.name = (2..)
                                    .map(|n| format!("{base} ({n})"))
                                    .find(|name| self.scene.items.iter().all(|i| &i.name != name))
                                    .unwrap_or(base);
                            }
                            copy.source_id = None;
                            copy.source_display_name = None;
                            self.scene.items.push(copy);
                        }
                        self.scene.next_id = next + ids.len() as u64;
                        self.next_item_id = self.scene.next_id;
                        if parent.is_some() {
                            // Keep the world position; a parent that would
                            // need shear keeps the transform instead.
                            if self.scene.set_parent(root, parent, true).is_err() {
                                self.scene.set_parent(root, parent, false)?;
                            }
                        }
                        self.scene.select_item(Some(root));
                        "Insert Items"
                    }
                    Command::RemoveBranch(id) => {
                        let branch = self.scene.branch(id)?;
                        let gone: std::collections::BTreeSet<u64> =
                            branch.iter().map(|i| i.id).collect();
                        self.scene.items.retain(|i| !gone.contains(&i.id));
                        if self.scene.selection.item.is_some_and(|i| gone.contains(&i)) {
                            self.scene.select_item(None);
                        }
                        "Move Items Out"
                    }
                    Command::ItemTransform { item, transform } => {
                        self.scene.refuse_locked(&[item])?;
                        let target = self.scene.item_mut(item).ok_or("Select an item")?;
                        // IT-PHY-01: a new rotation order keeps the Euler
                        // values, so the item's orientation changes.
                        target.transform = transform;
                        // Planar spec: a Texture Locator's Size stops at 1 mm.
                        if target.texture_locator().is_some() {
                            target.transform.scale =
                                crate::texture_locator::clamp_size(target.transform.scale);
                        }
                        "Item Transform"
                    }
                    Command::Rename(name) => {
                        if name.trim().is_empty() || name.len() > 1024 {
                            return Err("Name must contain 1–1024 bytes".into());
                        }
                        self.scene.active_mut().ok_or("Select an item")?.name =
                            name.trim().to_owned();
                        "Rename Mesh"
                    }
                    Command::SetParent {
                        item,
                        parent,
                        keep_position,
                    } => {
                        // LCK-05: a locked item keeps its links.
                        self.scene.refuse_locked(&[item])?;
                        self.scene.set_parent(item, parent, keep_position)?;
                        if parent.is_some() {
                            "Parent"
                        } else {
                            "Unparent"
                        }
                    }
                    Command::ParentToGroupLocator => {
                        // LCK-05: a locked item keeps its links.
                        self.scene.refuse_locked(&self.scene.selected_items())?;
                        let next = self.scene.next_id.max(self.next_item_id);
                        self.scene.next_id = next;
                        self.scene.parent_to_group_locator()?;
                        self.next_item_id = self.scene.next_id;
                        crate::locator::PARENT_TO_GROUP_LOCATOR
                    }
                    Command::MoveItems {
                        items,
                        parent,
                        before,
                    } => {
                        if self.scene.move_items(&items, parent, before)? {
                            if parent.is_some() {
                                "Parent"
                            } else {
                                "Unparent"
                            }
                        } else {
                            "Reorder"
                        }
                    }
                    Command::DropAction {
                        action,
                        sources,
                        destination,
                        ray,
                    } => {
                        self.scene.drop_action(action, &sources, destination, ray)?;
                        action.label()
                    }
                    Command::EditElements { kind, op } => {
                        self.scene.edit_elements(kind, op)?;
                        kind.label(op)
                    }
                    Command::Visibility(id) => {
                        // LCK-05: a locked item's eye is refused.
                        self.scene.refuse_locked(&[id])?;
                        let i = self.scene.item_mut(id).ok_or("Missing item")?;
                        i.visible = !i.visible;
                        // A plain toggle hides only the item (GV-03).
                        i.hides_children = false;
                        "Mesh Visibility"
                    }
                    Command::SetItemsVisible {
                        items,
                        visible,
                        hides_children,
                    } => {
                        let free: Vec<u64> = items
                            .iter()
                            .copied()
                            .filter(|id| self.scene.refuse_locked(&[*id]).is_ok())
                            .collect();
                        if free.is_empty() {
                            self.scene.refuse_locked(&items)?;
                            return Err("No items to show or hide".into());
                        }
                        for item in &mut self.scene.items {
                            if free.contains(&item.id) {
                                item.visible = visible;
                                item.hides_children = !visible && hides_children;
                            }
                        }
                        "Mesh Visibility"
                    }
                    Command::VisibilityWithChildren(id) => {
                        self.scene.refuse_locked(&[id])?;
                        let i = self.scene.item_mut(id).ok_or("Missing item")?;
                        i.visible = !i.visible;
                        let visible = i.visible;
                        let mut family = vec![id];
                        let mut at = 0;
                        while let Some(&parent) = family.get(at) {
                            at += 1;
                            for child in
                                self.scene.items.iter().filter(|i| i.parent == Some(parent))
                            {
                                family.push(child.id);
                            }
                        }
                        for item in &mut self.scene.items {
                            if family.contains(&item.id) {
                                item.visible = visible;
                                item.hides_children = false;
                            }
                        }
                        "Mesh Visibility"
                    }
                    _ => return Err("Unsupported command".into()),
                };
                // Deleted items leave every group.
                self.scene.prune_groups();
                if let Err(e) = self.scene.validate() {
                    self.scene = before;
                    return Err(e);
                }
                if before != self.scene {
                    self.record(label.into(), before);
                }
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    /// Start tool `t`, dropping or keeping the current task as its rules
    /// require. Selection inference runs first (pipe layer 0): a point or
    /// edge tool started from another mode starts on the converted set as
    /// if in its own mode, and the mode you're in stays.
    pub(crate) fn handle_activate(&mut self, t: Tool) -> Result<AfterCommand> {
        // An open tool keeps its own inference; the native start refuses.
        if self.has_active_task() {
            return self.handle_activate_native(t);
        }
        let saved = self.scene.selection.clone();
        if let Err(e) = self.infer_tool_selection(t) {
            self.scene.selection = saved;
            return Err(e);
        }
        let Some(target) = self.inferred_mode_for(t) else {
            return self.handle_activate_native(t);
        };
        self.scene.selection.mode = target;
        match self.handle_activate_native(t) {
            Ok(after) => {
                self.scene.selection.mode = saved.mode;
                if let Some(live) = self.live.as_mut() {
                    live.base.selection.mode = saved.mode;
                }
                Ok(after)
            }
            Err(e) => {
                self.scene.selection = saved;
                self.inference = None;
                Err(e)
            }
        }
    }
    fn handle_activate_native(&mut self, t: Tool) -> Result<AfterCommand> {
        // RPT-08: a new tool session starts with nothing to repeat.
        self.last_transform_step = None;
        if self.spatial_falloff.is_some() && !t.card().layers.falloff {
            return Err(crate::tool_card::falloff_refusal());
        }
        // Another open tool was dropped first by `tool_switch`.
        if self.has_active_task() {
            return Err("Drop or Cancel the active tool first".into());
        }
        self.multi_extrude = None;
        // Several active meshes: the tool starts on each (multi_tools.rs).
        if let Some(targets) = Self::tool_mesh_targets(t, &self.scene) {
            self.activate_on_meshes(t, &targets)?;
            return Ok(AfterCommand::BumpRevision);
        }
        // A Backdrop is moved, rotated and scaled like any item (BD-41),
        // whatever the selection mode: its transform tools switch to Item
        // mode (Jason, 2026-11-04). It has no components to keep.
        // Cameras and lights too: they are edited in Item mode only (CL-1).
        if self.scene.active().is_some_and(|i| !i.is_mesh())
            && t.transforms_items()
            && self.scene.selection.mode != Mode::Item
        {
            self.dispatch(Command::Mode(Mode::Item))?;
        }
        let item = self
            .scene
            .active()
            .ok_or("Select a mesh; use New Mesh before creating a primitive")?;
        let item_transform = self.scene.selection.mode == Mode::Item && t.transforms_items();
        // RG-04: a rig piece drives the transform the tool would change;
        // dragging it would only edit the hidden own value.
        if item_transform {
            let groups: &[&str] = match t {
                Tool::Move => &["pos"],
                Tool::Rotate => &["rot"],
                Tool::Scale => &["scl"],
                _ => &["pos", "rot", "scl"],
            };
            for id in self.scene.selected_items() {
                for g in groups {
                    for a in ["X", "Y", "Z"] {
                        if let Some(p) = crate::rig::driven_by(&self.scene, id, &format!("{g}.{a}"))
                        {
                            return Err(format!("Driven by {}", crate::rig::label(&p.kind)));
                        }
                    }
                }
            }
        }
        // Other tools have no geometry to edit on a Backdrop.
        if item.backdrop.is_some() && !item_transform {
            return Err(BACKDROP_NO_GEOMETRY.into());
        }
        // Group Locators (GL-03) and texture items too.
        if !item.is_mesh() && !item_transform {
            return Err(LOCATOR_NO_GEOMETRY.into());
        }
        if !self.scene.shows(item) && !item_transform {
            return Err("Show Mesh with its eye before component editing".into());
        }
        // Component tools work on moved/rotated/scaled items: frames and
        // handles are resolved in world space, edits map back to local.
        // Primitives are drawn in world space and mapped into the item's
        // local space (`Scene::world_to_local`), which refuses a zero scale
        // with its own message.
        if t.is_primitive() {
            self.scene.world_to_local(item.id)?;
        } else if !item_transform && self.scene.world_matrix(item.id)?.determinant() == 0. {
            return Err("Component tools need an invertible item transform".into());
        }
        // Move/Rotate/Scale also work on loose points (FAL-O weights are
        // measured on point-only meshes).
        // (The per-drag-undo tools: the transforms and Tweak.)
        let loose_points = t.is_transform() && !item.mesh.vertices.is_empty();
        if !t.is_primitive()
            && t != Tool::VertexCreate
            // Contour adds its curves to an empty mesh too (MT13-10).
            && t != Tool::Contour
            && !t.is_duplicate()
            && !loose_points
            && item.is_mesh()
            && item.mesh.faces.is_empty()
        {
            return Err("The selected mesh is empty".into());
        }
        if matches!(t, Tool::Bevel | Tool::TwistBevel | Tool::Extrude) || t.is_smooth_shift() {
            if self.scene.selection.mode != Mode::Polygon {
                return Err("Select polygons explicitly in Polygon mode".into());
            }
            if t == Tool::Extrude {
                // PE-O-03: with nothing selected Extrude works on every
                // polygon.
                let faces = &self.scene.selection.faces;
                let all: Vec<u32>;
                let faces = if faces.is_empty() {
                    all = (0..item.mesh.faces.len() as u32).collect();
                    &all
                } else {
                    faces
                };
                item.mesh.validate_extrude_selection(faces)?;
            } else if t.is_bevel() && self.scene.selection.faces.is_empty() {
                // PB-O-01: with nothing selected Bevel works on every polygon.
                // Checked per polygon here; a closed mesh grouped as one
                // region is refused by the preview, where Group is known.
                let all: Vec<u32> = (0..item.mesh.faces.len() as u32).collect();
                item.mesh.validate_polygon_selection(&all, false)?;
            } else if !t.is_smooth_shift() {
                item.mesh
                    .validate_polygon_selection(&self.scene.selection.faces, true)?;
            }
        }
        if t == Tool::SymmetryTool && self.active_symmetry().axis.is_none() {
            return Err("Turn Symmetry on (X, Y or Z) to use the Symmetry Tool".into());
        }
        if t.card().family == crate::tool_card::Family::Deform
            && self.scene.selection.mode == Mode::Item
        {
            return Err(format!(
                "{} works on points, edges or polygons; pick a component mode first",
                t.label()
            ));
        }
        let params = Parameters {
            position: self.scene.workplane.center,
            // User-admitted Operandi sample settings, not factory defaults.
            radial: t.initial_radial(),
            delta: if t == Tool::Scale {
                DVec3::ONE
            } else {
                DVec3::ZERO
            },
            ..Default::default()
        };
        // Bend v2 (BN-02, BN-24 revised): the pivot is the action centre
        // and the spine is fitted from it over the target's box.
        let mut bend_frame = None;
        let params = if t == Tool::Bend {
            let plane = self.scene.workplane.plane;
            // With Symmetry on, the centre and spine come from the side the
            // user works on; the other side bends with the mirror image
            // (SYM3-12).
            let one_side =
                crate::symmetry_sides::primary_side_scene(&self.scene, self.active_symmetry());
            let target = one_side.as_ref().unwrap_or(&self.scene);
            let frame = Self::extrude_frame(self.action_context, t, target)?;
            bend_frame = Some(frame);
            let fitted = crate::BendOptions::fitted(target, frame.center, plane, frame.basis)?;
            // BN-47: later uses start from the last use's values. The
            // centre still comes from the action centre (BN-02; BH-Q13,
            // provisional), so the ring and the pivot agree.
            let bend = match self.bend_preset {
                Some(last) => crate::BendOptions {
                    base: fitted.base,
                    ..last
                },
                None => fitted,
            };
            Parameters { bend, ..params }
        } else {
            params
        };
        if t.card().family == crate::tool_card::Family::Edge {
            // TA-06 (tool-activation v1): Edge Bevel with nothing selected
            // acts on every edge, and so does Edge Extrude (extrude-2026-11-05,
            // EE07). The other edge tools still need a selection.
            let all_edges;
            let edges = if matches!(t, Tool::EdgeBevel | Tool::EdgeExtrude)
                && self.scene.selection.mode == Mode::Edge
                && self.scene.selection.edges.is_empty()
            {
                all_edges = item.mesh.edges();
                &all_edges
            } else {
                &self.scene.selection.edges
            };
            if self.scene.selection.mode != Mode::Edge || edges.is_empty() {
                return Err(format!("{} needs selected edges in Edge mode", t.label()));
            }
            // Probe eligibility with a tiny change; the session starts at zero.
            if t == Tool::EdgeExtend {
                // Any axis not parallel to the edges proves eligibility
                // (a vertical edge cannot extend along Y).
                let probes = [DVec3::Y, DVec3::X, DVec3::Z]
                    .map(|d| crate::edge_extend_geometry(&item.mesh, edges, d * 1e-6, 1));
                if probes.iter().all(|r| r.is_err()) {
                    probes.into_iter().next().unwrap()?;
                }
            } else if t == Tool::EdgeSlide {
                crate::edge_slide_geometry(&item.mesh, edges, 0.)?;
            } else if t == Tool::EdgeCrease {
                crate::edge_crease_geometry(&item.mesh, edges, 0., false)?;
            } else if matches!(t, Tool::EdgeBevel | Tool::EdgeChamfer) {
                crate::edge_bevel_geometry(
                    &item.mesh,
                    edges,
                    1e-6,
                    1,
                    100.,
                    t == Tool::EdgeChamfer,
                )?;
            } else if t == Tool::EdgeRelax {
                crate::edge_relax_geometry(&item.mesh, edges, 0.)?;
            } else {
                crate::edge_extrude_geometry(&item.mesh, edges, 1e-6, 1e-6)?;
            }
        }
        if t.card().family == crate::tool_card::Family::Vertex {
            // Vertex Extrude with nothing selected acts on every point
            // (extrude-2026-11-05, VE08).
            let all_points: Vec<u32>;
            let points = if t == Tool::VertexExtrude
                && self.scene.selection.mode == Mode::Vertex
                && self.scene.selection.vertices.is_empty()
            {
                all_points = (0..item.mesh.vertices.len() as u32).collect();
                &all_points
            } else {
                &self.scene.selection.vertices
            };
            if self.scene.selection.mode != Mode::Vertex || points.is_empty() {
                return Err(format!(
                    "{} needs selected points in Vertex mode",
                    t.label()
                ));
            }
            // Probe eligibility with a tiny change; the session starts at zero.
            match t {
                Tool::VertexBevel => {
                    crate::vertex_bevel_geometry(&item.mesh, points, 1e-9)?;
                }
                Tool::VertexExtrude => {
                    crate::vertex_extrude_geometry(&item.mesh, points, 0., 1e-9)?;
                }
                Tool::VertexMerge => {
                    crate::vertex_merge_geometry(&item.mesh, points, 0.)?;
                }
                Tool::VertexTear => {
                    crate::vertex_tear_geometry(
                        &item.mesh,
                        points,
                        &self.scene.selection.faces,
                        DVec3::X * 1e-9,
                    )?;
                }
                _ => {
                    crate::vertex_slide_geometry(&item.mesh, points, DVec3::X, 1e-9)?;
                }
            }
        }
        if t == Tool::VertexCreate && self.scene.selection.mode != Mode::Vertex {
            return Err("Create works in Vertex mode".into());
        }
        // The path tools start only in the modes their card names; the
        // copier buttons (DX-11/30) work on items in any mode.
        if matches!(t, Tool::CurveExtrude | Tool::CurveClone)
            && !t.card().modes.contains(self.scene.selection.mode)
            && !self.copier_row_open(t)
        {
            return Err(format!(
                "{} works in {}",
                t.label(),
                t.card().modes.describe()
            ));
        }
        // The action-centre layer: a tool that needs the neutral context
        // refuses any other, and the modes its card leaves out.
        let card = t.card();
        if card.layers.action_centre == crate::tool_card::ActionCentre::Neutral
            && (!card.modes.contains(self.scene.selection.mode)
                || self.action_context != crate::ActionContext::default()
                || self.spatial_falloff.is_some())
        {
            return Err(format!(
                "{} requires {} and neutral context",
                t.label(),
                card.modes.describe()
            ));
        }
        if t == Tool::Chamfer {
            let probe = crate::ChamferParameters {
                offset: 1e-6,
                ..self.chamfer_preset
            };
            let selected = &self.scene.selection.faces;
            if let Err(error) = crate::chamfer_geometry(&item.mesh, selected, probe) {
                // A chamfer of several segments starts the tool too; its
                // Source Segments is set next (CE12-CE15, rule 6).
                let across = crate::ChamferParameters {
                    source_segments: (selected.len() as u32).max(1),
                    ..probe
                };
                if across.source_segments == probe.source_segments
                    || crate::chamfer_geometry(&item.mesh, selected, across).is_err()
                {
                    return Err(error);
                }
            }
        }
        if matches!(t, Tool::Inset | Tool::Shift) {
            if self.scene.selection.mode != Mode::Polygon {
                return Err(format!("{} works in Polygon mode", t.label()));
            }
            // Probe eligibility; nothing selected means every polygon (IN10).
            crate::inset_geometry(&item.mesh, &self.scene.selection.faces, 0.)?;
        }
        // Poly Haul: Tear and Slide pick points, so Vertex mode works too (C8).
        if t == Tool::PolyHaul && !self.scene.selection.faces.is_empty() {
            crate::poly_haul_geometry(&item.mesh, &self.scene.selection.faces, 0.)?;
        }
        if t == Tool::SketchExtrude {
            crate::validate_sketch_target(&item.mesh, &self.scene.selection.faces)?;
        }
        // Edge Weight starts at the first selected edge's current weight.
        let params = if t == Tool::EdgeCrease {
            let first = self.scene.selection.edges.first().copied();
            let weight = first.and_then(|(a, b)| {
                let (a, b) = (a.min(b), a.max(b));
                item.mesh
                    .attributes
                    .creases
                    .iter()
                    .find(|c| (c.a, c.b) == (a, b))
                    .map(|c| c.weight)
            });
            // With Additive the field is the amount added, from 0.
            Parameters {
                shift: if params.edge_weight_additive {
                    0.
                } else {
                    weight.unwrap_or(0.)
                },
                ..params
            }
        } else {
            params
        };
        // Edge and Vertex Extrude start with a Width, so dragging the Extrude
        // arrow alone extrudes (Width 0 changes nothing). Provisional
        // (EXW-Q01).
        let params = match t {
            Tool::EdgeExtrude => Parameters {
                inset: crate::edge_extrude_start_width(&item.mesh, &self.scene.selection.edges)
                    .unwrap_or(0.),
                ..params
            },
            Tool::VertexExtrude => Parameters {
                inset: crate::vertex_extrude_start_width(
                    &item.mesh,
                    &self.scene.selection.vertices,
                )
                .unwrap_or(0.),
                ..params
            },
            _ => params,
        };
        // Vertex Slide's rail runs from the first point to its first
        // neighbour, in world space.
        let params = if t == Tool::VertexSlide {
            let rail = crate::vertex_slide_direction(&item.mesh, &self.scene.selection.vertices)
                .unwrap_or(DVec3::X);
            let world = self.scene.world_matrix(item.id)?.transform_vector3(rail);
            Parameters {
                delta: world.normalize_or_zero(),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::Chamfer {
            Parameters {
                chamfer: self.chamfer_preset,
                ..params
            }
        } else {
            params
        };
        let params = if matches!(t, Tool::EdgeBevel | Tool::EdgeChamfer) {
            let preset = if t == Tool::EdgeBevel {
                &self.edge_bevel_preset
            } else {
                &self.edge_chamfer_preset
            };
            Parameters {
                chamfer: crate::ChamferParameters {
                    segments: preset.segments,
                    depth: preset.depth,
                    ..params.chamfer
                },
                edge_bevel: preset.options.clone(),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::PolyHaul {
            Parameters {
                poly_haul_offset: self.poly_haul_preset,
                ..params
            }
        } else {
            params
        };
        // Merge Tool: Distance 0.001 after Reset, else the last use's value
        // (VTO-T04).
        let params = if t == Tool::VertexMerge {
            Parameters {
                shift: self.vertex_merge_preset.unwrap_or(MERGE_TOOL_DISTANCE),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::SketchExtrude {
            Parameters {
                sketch_precision: self.sketch_precision_preset.unwrap_or(30.),
                ..params
            }
        } else {
            params
        };
        let params = if t == Tool::Extrude {
            self.extrude_preset.clone()
        } else {
            params
        };
        let params = if t.is_duplicate() {
            let mut duplicate = self.duplicate_preset.options_only();
            // A copier comes only from the open preset row (DX-20/30), so a
            // plain button never inherits the last row's.
            duplicate.copier = self
                .preset
                .as_ref()
                .filter(|p| p.tool == t)
                .and_then(|p| p.row().actor.copier);
            Parameters {
                duplicate: Box::new(duplicate),
                ..params
            }
        } else {
            params
        };
        let params = if t.is_slice() {
            Parameters {
                slice: crate::slice::start(t, &self.scene, &self.slice_preset),
                ..params
            }
        } else {
            params
        };
        let params = if t.is_smooth_shift() {
            Parameters {
                smooth_shift: if t == Tool::Thicken {
                    self.thicken_preset
                        .unwrap_or_else(crate::SmoothShiftParameters::thicken_initial)
                } else {
                    self.smooth_shift_preset
                },
                ..params
            }
        } else {
            params
        };
        // Jason decision 2026-11-01 (BOX-Q09/SP-Q08/CYL-Q11): primitives
        // start with the segment settings they had last time, like Cone and
        // Ellipsoid; Reset restores the starting values. Size and position
        // still come from the drag.
        let params = match t {
            Tool::Box | Tool::Sphere | Tool::Cylinder => Parameters {
                make_uvs: !self.primitive_uvs_off[primitive_slot(t)],
                ..params
            },
            _ => params,
        };
        let params = match t {
            Tool::Box => Parameters {
                segments: self.cube_preset.segments,
                ..params
            },
            Tool::Sphere | Tool::Cylinder => {
                let mut params = params;
                if let (Some(radial), Some([sides, segments])) =
                    (params.radial.as_mut(), self.unit_resolution(t))
                {
                    radial.sides = sides;
                    radial.segments = segments;
                }
                if let (Tool::Cylinder, Some((caps, axis))) = (t, self.cylinder_shape) {
                    params.cylinder_caps = caps;
                    params.radial_axis = axis;
                }
                if let (Tool::Sphere, Some((axis, level))) = (t, self.sphere_shape) {
                    params.radial_axis = axis;
                    params.sphere_level = level;
                }
                params
            }
            _ => params,
        };
        self.live = Some(Live {
            identity_base: Some(self.identities.current.clone()),
            identity_preview: None,
            tool: t,
            base: self.scene.clone(),
            params,
            applied: false,
            preview_error: None,
            // The action-centre layer: tools that work about a frame
            // capture it now.
            placed_centre: None,
            action_frame: if let Some(frame) = bend_frame {
                Some(frame)
            } else if t.card().layers.action_centre == crate::tool_card::ActionCentre::Frame {
                Some(self.pivot_frame(&self.scene, t)?)
            } else {
                None
            },
        });
        // Smooth works straight away at its starting Strength; Drag and the
        // fields then adjust that one result (deform.rs). So do the Deform
        // finish tools (deform_finish.rs).
        if t.shows_result_at_once() {
            let p = self.live.as_ref().ok_or("No active tool")?.params.clone();
            self.preview(p, false)?;
        }
        // Loop Slice with a ring from the selection shows its slice at once
        // (part of the activation step). A ring it can't cut is dropped, and
        // the tool waits for a click on an edge.
        if t == Tool::LoopSlice
            && let Some(p) = self.live.as_ref().map(|l| l.params.clone())
            && !p.slice.loop_slice.rings.is_empty()
            && self.preview(p, false).is_err()
            && let Some(live) = self.live.as_mut()
        {
            live.params.slice.loop_slice.rings.clear();
        }
        Ok(AfterCommand::BumpRevision)
    }

    /// Continue the live tool on a new stage (e.g. Shift-click on caps).
    fn handle_continue(&mut self) -> Result<()> {
        let live = self
            .live
            .as_ref()
            .ok_or("Activate Bevel or Extrude first")?;
        if !live.tool.card().continues {
            return Err("Continuation is available for Bevel, Extrude and Edge Extend only".into());
        }
        if live.tool == Tool::EdgeExtend {
            return self.continue_edge_extend();
        }
        if let Some(error) = &live.preview_error {
            return Err(format!(
                "Correct the current stage before continuation: {error}"
            ));
        }
        self.scene.validate()?;
        let mut next = live.clone();
        if live.tool == Tool::Extrude
            && live.applied
            && let Some(multi) = self.multi_extrude.as_ref()
        {
            // Several meshes: each continues from its own moving caps.
            next.base = self.scene.clone();
            for (&id, caps) in &multi.moved {
                next.base.selection.set_faces_of(id, caps.clone());
            }
        } else if live.tool == Tool::Extrude && live.applied && !self.extrude_moved_caps.is_empty()
        {
            next.base = self.scene.clone();
            next.base.selection.faces = self.extrude_moved_caps.clone();
        } else {
            next.base = self.scene.clone();
        }
        if next.tool.is_smooth_shift() {
            next.params.smooth_shift.offset = 0.;
            next.params.smooth_shift.scale = 1.;
        }
        next.params.shift = 0.;
        next.params.inset = 0.;
        // Twist Bevel's fresh stage keeps its shape settings (twist, bend,
        // profile); only its size starts again.
        next.params.twist_bevel.shift = 0.;
        next.params.twist_bevel.inset = 0.;
        next.params.extent = DVec3::ZERO;
        next.applied = false;
        // DS-34: a Duplicate tool starts a new placement on the result.
        next.params.duplicate.placed = false;
        next.action_frame = if next.tool.is_duplicate() {
            None
        } else {
            Some(self.pivot_frame(&next.base, next.tool)?)
        };
        next.placed_centre = None;
        // Inside Bevel's tool session the finished bevel stays a step of the
        // tool's own (`tweak_before`); Drop records the session from its
        // start (`tweak_session_base`).
        let tool_step = live.tool.is_bevel() && self.tweak_session().is_some();
        if live.base != self.scene && !tool_step {
            self.identities.record_before = live.identity_base.clone();
            let before = live.base.clone();
            let label = live.tool.label().to_string();
            self.record(label, before);
        }
        next.identity_base = Some(self.identities.current.clone());
        next.identity_preview = None;
        self.scene.selection = next.base.selection.clone();
        self.live = Some(next);
        Ok(())
    }

    /// Edge Extend's Shift+click: keep this extension and start a new one
    /// from the edges it made (selected by the preview), with the same
    /// values, shown at once (spec: "extend it back once, Shift-click,
    /// extend it back twice"). The action frame is resolved again on the
    /// new edges, so the centre follows them.
    fn continue_edge_extend(&mut self) -> Result<()> {
        let live = self.live.as_ref().ok_or("Activate Edge Extend first")?;
        if !live.applied || live.base == self.scene {
            return Err("Extend the edges first; Shift+click then extends again".into());
        }
        let mut next = live.clone();
        next.base = self.scene.clone();
        next.applied = false;
        next.action_frame = Some(self.pivot_frame(&next.base, Tool::EdgeExtend)?);
        let tool_step = self.tweak_session().is_some();
        if !tool_step {
            self.identities.record_before = live.identity_base.clone();
            let before = live.base.clone();
            self.record("Extend".to_string(), before);
        }
        next.identity_base = Some(self.identities.current.clone());
        next.identity_preview = None;
        let params = next.params.clone();
        self.live = Some(next);
        self.preview(params, false)
    }

    /// Apply the live tool or numeric session.
    fn handle_apply(&mut self) -> Result<AfterCommand> {
        // PF-12: Apply on an Edge Extend still at its Reset values adds the
        // new zero-width row, as a click start does (`extend_from_click`);
        // a live preview at those values leaves the mesh alone.
        if let Some(l) = self.live.as_ref()
            && l.tool == Tool::EdgeExtend
            && !l.applied
            && !self.extend_from_click
        {
            self.extend_from_click = true;
            let p = l.params.clone();
            if let Err(e) = self.dispatch(Command::Preview(p)) {
                self.extend_from_click = false;
                return Err(e);
            }
        }
        if let Some(l) = self.live.as_ref().filter(|l| l.tool.is_smooth_shift())
            && let Some(e) = &l.preview_error
        {
            return Err(format!("Correct this stage before Apply: {e}"));
        }

        if self.reduction.is_some() {
            self.apply_reduction()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.bridge.is_some() {
            self.apply_bridge()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.spike.is_some() {
            self.apply_spike()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.ellipsoid.is_some() {
            self.commit_ellipsoid(false)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.shape.is_some() {
            self.commit_shape(false)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.cone.is_some() {
            self.apply_cone()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.live.as_ref().is_some_and(|l| {
            (matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                || l.tool.is_smooth_shift())
                && !l.applied
        }) {
            let mut staged = self.clone();
            let p = staged.live.as_ref().unwrap().params.clone();
            let before = staged.scene.clone();
            staged.preview(p.clone(), true)?;
            if staged.scene != before {
                staged.record(staged.live.as_ref().unwrap().tool.label().into(), before);
            }
            if staged.live.as_ref().unwrap().tool == Tool::Extrude {
                staged.extrude_preset = p;
            }
            let base = staged.scene.clone();
            let l = staged.live.as_mut().unwrap();
            l.base = base;
            l.identity_base = Some(staged.identities.current.clone());
            l.identity_preview = None;
            l.applied = false;
            *self = staged;
        } else if self
            .live
            .as_ref()
            .is_some_and(|l| l.tool.is_bevel() && l.applied && l.preview_error.is_none())
        {
            // Panel edits make Bevel live at once (TA-01), so Apply finds a
            // live stage: it keeps that bevel as it is and the next edit or
            // Apply bevels the result again (bevel-activation rule 6).
            let l = self.live.as_ref().unwrap();
            let mut next = l.clone();
            if l.base != self.scene {
                self.identities.record_before = l.identity_base.clone();
                let before = l.base.clone();
                self.record(l.tool.label().into(), before);
            }
            next.base = self.scene.clone();
            next.identity_base = Some(self.identities.current.clone());
            next.identity_preview = None;
            next.applied = false;
            self.live = Some(next);
        } else if self.live.as_ref().is_some_and(|l| {
            matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::Extrude)
                || l.tool.is_smooth_shift()
        }) {
            return Err(
                "Apply is unavailable during a live polygon preview; Drop or Continue this stage"
                    .into(),
            );
        } else if let Some(live) = self.live.as_ref() {
            // Legacy internal callers remain idempotent. The GUI disables
            // Apply while interactive, as the approved presentation policy.
            let p = live.params.clone();
            self.preview(p, true)?;
        } else {
            let mut staged = self.clone();
            staged.dispatch(Command::Activate(Tool::Box))?;
            staged.dispatch(Command::Preview(self.cube_preset.clone()))?;
            staged.dispatch(Command::Drop)?;
            *self = staged;
        }
        Ok(AfterCommand::BumpRevision)
    }

    fn unit_resolution(&self, t: Tool) -> Option<[u32; 2]> {
        match t {
            Tool::Sphere => self.sphere_unit_resolution,
            Tool::Cylinder => self.cylinder_unit_resolution,
            _ => None,
        }
    }

    fn reset_primitive(&mut self) -> Result<()> {
        let live = self
            .live
            .as_ref()
            .filter(|l| l.tool.is_primitive())
            .ok_or("Reset needs a live Cube, Sphere or Cylinder")?;
        let tool = live.tool;
        let mut params = live.params.clone();
        params.segments = Parameters::default().segments;
        params.make_uvs = true;
        params.cylinder_caps = Parameters::default().cylinder_caps;
        params.radial_axis = Parameters::default().radial_axis;
        params.sphere_level = None;
        if let (Some(radial), Some(initial)) = (params.radial.as_mut(), tool.initial_radial()) {
            radial.sides = initial.sides;
            radial.segments = initial.segments;
        }
        let mut staged = self.clone();
        staged.primitive_uvs_off[primitive_slot(tool)] = false;
        match tool {
            Tool::Box => staged.cube_preset.segments = params.segments,
            Tool::Sphere => {
                staged.sphere_unit_resolution = None;
                staged.sphere_shape = None;
            }
            _ => {
                staged.cylinder_unit_resolution = None;
                staged.cylinder_shape = None;
            }
        }
        staged.set_primitive_params(params)?;
        *self = staged;
        Ok(())
    }

    /// Replace a live primitive's settings: rebuild the preview when
    /// something is drawn, otherwise just store them without creating geometry.
    fn set_primitive_params(&mut self, params: Parameters) -> Result<()> {
        let live = self.live.as_ref().ok_or("No active primitive")?;
        if self.scene != live.base {
            let mut staged = self.clone();
            staged.dispatch(Command::Preview(params))?;
            *self = staged;
        } else if let Some(l) = self.live.as_mut() {
            l.params = params;
        }
        Ok(())
    }

    /// Drop the live tool or numeric session, keeping its result.
    /// PE-M-03 / PE-O-01/03: after Extrude both caps are selected, the new
    /// one and, where Thick (or Auto on an isolated region) keeps it, the
    /// reversed source polygon. During the drag only the moving caps are
    /// selected; each appended moving cap's source is its kept bottom cap.
    pub(crate) fn select_extrude_caps(&mut self, l: &Live) {
        if let Some(faces) = extrude_caps_on_drop(l, &self.scene.selection.faces) {
            self.scene.selection.faces = faces;
        }
    }

    fn handle_drop(&mut self) -> Result<AfterCommand> {
        if self.reduction.is_some() {
            self.drop_reduction()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.bridge.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.spike.take().is_some() {
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.ellipsoid.is_some() {
            self.commit_ellipsoid(true)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.shape.is_some() {
            self.commit_shape(true)?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if self.cone.is_some() {
            self.drop_cone()?;
            self.revision = self.revision.wrapping_add(1);
            return Ok(AfterCommand::Done);
        }
        if let Some(error) = self.live.as_ref().and_then(|l| l.preview_error.as_ref()) {
            return Err(format!(
                "Correct the current stage or Cancel before Drop: {error}"
            ));
        }
        let session_base = self.tweak_session_base();
        if let Some(l) = self.live.take() {
            // RPT-08: dropping the tool forgets the amount a middle-click
            // repeats.
            self.last_transform_step = None;
            if l.tool.is_smooth_shift() {
                self.retain_surface(l.tool, l.params.smooth_shift);
            }
            if l.tool == Tool::Chamfer {
                self.chamfer_preset = l.params.chamfer;
            }
            if matches!(l.tool, Tool::EdgeBevel | Tool::EdgeChamfer) {
                let preset = crate::EdgeBevelPreset {
                    segments: l.params.chamfer.segments,
                    depth: l.params.chamfer.depth,
                    options: l.params.edge_bevel.clone(),
                };
                if l.tool == Tool::EdgeBevel {
                    self.edge_bevel_preset = preset;
                } else {
                    self.edge_chamfer_preset = preset;
                }
            }
            if l.tool == Tool::PolyHaul {
                self.poly_haul_preset = l.params.poly_haul_offset;
            }
            if l.tool == Tool::SketchExtrude {
                self.sketch_precision_preset = Some(l.params.sketch_precision);
            }
            if l.tool == Tool::VertexMerge {
                self.vertex_merge_preset = Some(l.params.shift);
            }
            if l.tool == Tool::Bend {
                self.bend_preset = Some(l.params.bend);
            }
            if l.tool.is_slice() {
                self.slice_preset = l.params.slice.options_only();
            }
            if l.tool.is_duplicate() {
                *self.duplicate_preset = l.params.duplicate.options_only();
            }
            if l.tool == Tool::Extrude {
                self.extrude_preset = l.params.clone();
                if let Some(multi) = self.multi_extrude.take() {
                    // Several meshes: each selects its own caps.
                    if l.applied {
                        for (id, caps) in multi.dropped {
                            self.scene.selection.set_faces_of(id, caps);
                        }
                    }
                } else {
                    self.select_extrude_caps(&l);
                }
            }
            if let Some(p) = l.params.radial {
                match l.tool {
                    Tool::Sphere => {
                        self.sphere_unit_resolution = Some([p.sides, p.segments]);
                        self.sphere_shape = Some((l.params.radial_axis, l.params.sphere_level));
                    }
                    Tool::Cylinder => {
                        self.cylinder_unit_resolution = Some([p.sides, p.segments]);
                        self.cylinder_shape = Some((l.params.cylinder_caps, l.params.radial_axis));
                    }
                    _ => {}
                }
            }
            if l.tool.is_primitive() {
                self.primitive_uvs_off[primitive_slot(l.tool)] = !l.params.make_uvs;
            }
            if l.tool == Tool::Box {
                self.cube_preset = l.params;
            }
            // The whole session is one step: from before its first tweak.
            let (base, identity_base) = match session_base {
                Some((scene, ids)) => (scene, Some(ids)),
                None => (l.base, l.identity_base),
            };
            if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend) {
                self.record_bevel_session(l.tool, base, identity_base);
            } else if base != self.scene {
                self.identities.record_before = identity_base;
                // A tool preset's session is named after the preset.
                let label = self
                    .preset
                    .as_ref()
                    .filter(|p| p.tool == l.tool)
                    .map_or(l.tool.label(), crate::PresetSession::label);
                self.record(label.to_string(), base);
            }
        }
        Ok(AfterCommand::BumpRevision)
    }

    /// Cancel the live tool or numeric session, discarding its preview.
    fn handle_cancel(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        if let Some(report) = report {
            report.effect = crate::HistoryEffect::Cancel;
        }
        if self.reduction.is_some() {
            self.cancel_reduction()?;
        }
        self.bridge = None;
        self.spike = None;
        self.ellipsoid = None;
        self.shape = None;
        self.cone = None;
        let session_base = self.tweak_session_base();
        if let Some(l) = self.live.take() {
            self.last_transform_step = None;
            if matches!(l.tool, Tool::Bevel | Tool::TwistBevel | Tool::EdgeExtend) {
                // Cancel restores this stage and keeps the bevels finished
                // before it, recorded as one step from the session start.
                self.scene = l.base;
                if let Some(ids) = l.identity_base {
                    self.identities.current = ids;
                    self.identities.observe(&self.scene)?;
                }
                if let Some((start, ids)) = session_base {
                    self.record_bevel_session(l.tool, start, Some(ids));
                }
                return Ok(());
            }
            // Cancel takes back every tweak of the session.
            let (base, ids) = match session_base {
                Some((scene, ids)) => (scene, Some(ids)),
                None => (l.base, l.identity_base),
            };
            self.scene = base;
            if let Some(ids) = ids {
                self.identities.current = ids;
                self.identities.observe(&self.scene)?;
            }
        }
        Ok(())
    }

    /// Undo one history step.
    fn handle_undo(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        self.history_eligibility(crate::HistoryAction::Undo)
            .permit_reported(report)?;
        // Inside a tool, Undo takes back its last tweak and keeps the tool
        // (Jason, 2026-11-04).
        if let Some(label) = self.undo_tweak()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::TweakUndone {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        // Mid-tool undo (MTU-Q1, provisional): a numeric primitive's
        // unrecorded preview goes first, and the tool stays active.
        if let Some(label) = self.discard_primitive_preview(true)? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::PreviewDiscarded { label };
            }
            return Ok(());
        }
        // With none of its own steps left, Undo undoes the tool's
        // activation: the tool closes, geometry stays (MU-MID-02/03), and a
        // drawn primitive's unfinished preview goes with it (U1).
        if let Some(label) = self.close_tool_by_undo()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::ToolClosed {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        if let Some(label) = self.discard_primitive_preview(false)? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::PreviewDiscarded { label };
            }
            return Ok(());
        }
        // A transform tool's pending change was recorded as its own step
        // before this transaction (see `dispatch_transaction`).
        let transform = self
            .live_transform_tool()
            .or_else(|| self.live_primitive_tool());
        if transform.is_some() && self.undo.is_empty() {
            return Ok(());
        }
        // Only supported unapplied surface tools and transform tools can
        // reach ordinary Undo with live state; the policy above is its
        // single guard.
        self.live = None;
        self.selection_run = 0;
        if let Some(c) = self.undo.pop() {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::Completed {
                    action: crate::HistoryAction::Undo,
                    label: c.label.clone(),
                };
            }
            let _undone = std::mem::replace(&mut self.scene, c.before.clone());
            // TK-05: undo never changes the current time.
            crate::animation::keep_time(&_undone, &mut self.scene);
            // Images aren't undone (PNT-11).
            #[cfg(feature = "painting")]
            crate::paint::carry_images(&_undone, &mut self.scene);
            if let Some(ids) = &c.identity_before {
                self.identities.current = ids.clone();
                self.identities.observe(&self.scene)?;
            }
            if let Some(n) = self.tool_steps.as_mut() {
                *n = n.saturating_sub(1);
            }
            self.redo.push(c);
        }
        if let Some(tool) = transform {
            self.resume_transform(tool);
        }
        self.rebase_numeric_primitive();
        Ok(())
    }

    /// Redo one history step.
    fn handle_redo(
        &mut self,
        report: &mut Option<crate::history_completion::HistoryReport>,
    ) -> Result<()> {
        self.history_eligibility(crate::HistoryAction::Redo)
            .permit_reported(report)?;
        if let Some(label) = self.redo_tweak()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::TweakRedone {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        // Redo of a tool's activation step opens it again (U4).
        if let Some(label) = self.reopen_tool_by_redo()? {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::ToolReopened {
                    label: label.into(),
                };
            }
            return Ok(());
        }
        self.spike = None;
        let transform = self
            .live_transform_tool()
            .or_else(|| self.live_primitive_tool());
        if transform.is_some() && !self.redo.is_empty() {
            self.live = None;
        }
        self.selection_run = 0;
        if let Some(c) = self.redo.pop() {
            if let Some(report) = report {
                report.effect = crate::HistoryEffect::Completed {
                    action: crate::HistoryAction::Redo,
                    label: c.label.clone(),
                };
            }
            let _redone = std::mem::replace(&mut self.scene, c.after.clone());
            crate::animation::keep_time(&_redone, &mut self.scene);
            #[cfg(feature = "painting")]
            crate::paint::carry_images(&_redone, &mut self.scene);
            if let Some(ids) = &c.identity_after {
                self.identities.current = ids.clone();
                self.identities.observe(&self.scene)?;
            }
            if let Some(n) = self.tool_steps.as_mut() {
                *n += 1;
            }
            self.undo.push(c);
            if let Some(tool) = transform {
                self.resume_transform(tool);
            }
            self.rebase_numeric_primitive();
        }
        Ok(())
    }

    /// A live Cube, Sphere or Cylinder: like the transform tools it keeps
    /// running through Undo/Redo and restarts on the restored scene.
    fn live_primitive_tool(&self) -> Option<Tool> {
        self.live
            .as_ref()
            .map(|l| l.tool)
            .filter(|t| t.is_primitive())
    }

    /// Throw away a primitive's preview that isn't a step yet, keeping the
    /// tool active. Numeric sessions keep their values; a drawn Cube, Sphere
    /// or Cylinder restarts empty, as after activation. Returns the tool's
    /// name, or None when no such preview shows.
    fn discard_primitive_preview(&mut self, numeric_only: bool) -> Result<Option<String>> {
        let Some(label) = self.primitive_session().and_then(|s| s.pending_preview) else {
            return Ok(None);
        };
        if numeric_only && self.live.is_some() {
            return Ok(None);
        }
        if let Some(s) = &mut self.ellipsoid {
            s.preview = None;
        } else if let Some(s) = &mut self.shape {
            s.preview = None;
        } else if let Some(s) = &mut self.cone {
            s.preview = None;
        } else if let Some(l) = self.live.take() {
            self.scene = l.base;
            if let Some(ids) = l.identity_base {
                self.identities.current = ids;
                self.identities.observe(&self.scene)?;
            }
            self.resume_transform(l.tool);
        }
        Ok(Some(label.to_string()))
    }

    /// After Undo/Redo changed the scene under a numeric primitive session:
    /// keep it on its mesh when that mesh can still take the shape, close
    /// it otherwise (MTU-Q6, provisional). Its values stay; nothing is
    /// previewed until a value changes.
    fn rebase_numeric_primitive(&mut self) {
        if let Some(s) = &self.ellipsoid
            && self.ellipsoid_target_ok(s.target).is_err()
        {
            self.ellipsoid = None;
        }
        if let Some(s) = &self.shape
            && self.shape_target_ok(s.target, s.kind()).is_err()
        {
            self.shape = None;
        }
        if let Some(s) = &self.cone
            && !self
                .scene
                .items
                .iter()
                .any(|i| i.id == s.target && self.scene.shows(i))
        {
            self.cone = None;
        }
        for preview in [
            self.ellipsoid.as_mut().map(|s| &mut s.preview),
            self.shape.as_mut().map(|s| &mut s.preview),
            self.cone.as_mut().map(|s| &mut s.preview),
        ]
        .into_iter()
        .flatten()
        {
            *preview = None;
        }
    }

    /// A live transform tool shows a change not yet recorded as a step
    /// (a typed field value, or a drag still in progress).
    pub fn transform_change_pending(&self) -> bool {
        self.live
            .as_ref()
            .is_some_and(|l| l.tool.is_transform() && l.base != self.scene)
    }

    fn live_transform_tool(&self) -> Option<Tool> {
        self.live
            .as_ref()
            .map(|l| l.tool)
            .filter(|t| t.is_transform())
    }

    /// Record the live transform tool's change since its last step as one
    /// undo step, then keep the tool live from the result with zero offset.
    /// The center follows a move so the handle stays on the moved
    /// selection. Provisional (Jason, 2026-11-03; HND-Q03): the tool's
    /// fields restart from zero after each drag.
    pub(crate) fn commit_transform_step(&mut self) -> Result<()> {
        let live = self
            .live
            .as_ref()
            .filter(|l| l.tool.is_transform())
            .ok_or("No active transform tool")?;
        if let Some(error) = &live.preview_error {
            return Err(format!("Correct the current transform first: {error}"));
        }
        if live.base == self.scene {
            return Ok(());
        }
        self.scene.validate()?;
        self.last_transform_step = Some(Box::new(crate::transform_repeat::TransformStep {
            tool: live.tool,
            base: live.base.clone(),
            params: live.params.clone(),
            action_frame: live.action_frame,
            identity_base: live.identity_base.clone(),
            applied: live.applied,
            after: self.scene.clone(),
        }));
        let mut next = live.clone();
        if matches!(live.tool, Tool::Move | Tool::Transform)
            && let Some(frame) = next.action_frame.as_mut()
        {
            frame.center += frame.basis * live.params.delta;
        }
        next.params.delta = if live.tool == Tool::Scale {
            DVec3::ONE
        } else {
            DVec3::ZERO
        };
        next.params.rotation_delta = DVec3::ZERO;
        next.params.scale_delta = DVec3::ONE;
        // Box Transform (UVT-43): the next drag starts from the moved box;
        // a set pivot moves with it.
        let b = live.params.uv_box;
        next.params.uv_box = crate::uv_box::BoxTransform {
            pivot: b.pivot.map(|p| p + b.offset),
            show_rotation_handle: b.show_rotation_handle,
            snap_angle: b.snap_angle,
            ..Default::default()
        };
        next.params.tweak = None;
        next.params.tweak_plan = None;
        // TK-30: the drag's edit keys when it ends, not only on Drop.
        let base = std::mem::take(&mut next.base);
        self.auto_key_from(&base);
        next.base = self.scene.clone();
        next.applied = false;
        // The drag stays a tweak of the session (Jason, 2026-11-04; this
        // replaces TWK-24's step per Tweak drag); Drop records the session
        // as one step.
        next.identity_base = Some(self.identities.current.clone());
        next.identity_preview = None;
        self.live = Some(next);
        Ok(())
    }

    /// After Undo/Redo with a transform tool live: start it again on the
    /// restored scene (its center comes from the restored selection). If
    /// the restored scene can't host the tool it stays dropped.
    fn resume_transform(&mut self, tool: Tool) {
        if self.live.is_none() {
            let _ = self.handle_activate(tool);
        }
    }

    /// Change the action center and axis.
    fn handle_action_context(&mut self, context: crate::ActionContext) -> Result<()> {
        if self.cone.is_some() {
            return Err("Numeric Cone uses world coordinates; context changes during this session remain uncharacterized".into());
        }
        // Resolve before publishing context so invalid Selection requests
        // preserve the live stage, previous context and history atomically.
        let frame = self.action_context_frame(context)?;
        let previous_context = self.action_context;
        let previous_frame = self.live.as_ref().and_then(|live| live.action_frame);
        let previous_bend_center = self.live.as_ref().map(|live| live.params.bend.base);
        let parameters = self
            .live
            .as_ref()
            .filter(|live| {
                live.applied
                    && matches!(
                        live.tool,
                        Tool::Move
                            | Tool::Rotate
                            | Tool::Scale
                            | Tool::Transform
                            | Tool::EdgeExtend
                    )
            })
            .map(|live| live.params.clone());
        self.action_context = context;
        if let Some(live) = &mut self.live {
            live.action_frame = frame;
            live.placed_centre = None;
        }
        // Bend: its centre is the action centre, so a new centre mode moves
        // it (and re-bends from there once applied).
        let parameters = parameters.or_else(|| {
            let live = self.live.as_mut().filter(|l| l.tool == Tool::Bend)?;
            let center = live.action_frame?.center;
            live.params.bend.base = center;
            live.applied.then(|| live.params.clone())
        });
        if let Some(parameters) = parameters
            && let Err(error) = self.preview(parameters, false)
        {
            // Transform preview validates its local candidate before
            // publishing scene/params. Restore only the frame/context
            // staged above; never clone or commit the undo history.
            self.action_context = previous_context;
            if let Some(live) = &mut self.live {
                live.action_frame = previous_frame;
                if let Some(center) = previous_bend_center {
                    live.params.bend.base = center;
                }
            }
            return Err(error);
        }
        Ok(())
    }

    /// Select the polygons of a named polygon set.
    fn handle_select_polygon_set(&mut self, name: String) -> Result<()> {
        self.require_idle()?;
        let item = self.scene.active().ok_or("Select a mesh")?;
        if !self.scene.shows(item) {
            return Err("Show the mesh before recalling components".into());
        }
        let members = item
            .mesh
            .attributes
            .polygon_sets
            .get(&name)
            .ok_or("Named polygon set not found")?;
        let ids = self
            .identities
            .current
            .meshes
            .get(&item.id)
            .ok_or("Missing identity view")?;
        let faces = ids
            .logical_faces
            .iter()
            .filter_map(|id| ids.faces.iter().position(|x| x == id))
            .filter(|i| members.contains(&(*i as u32)))
            .map(|i| i as u32)
            .collect();
        let before = self.scene.clone();
        self.scene.selection.mode = Mode::Polygon;
        self.scene.selection.faces = faces;
        self.scene.selection.vertices.clear();
        self.scene.selection.edges.clear();
        if before != self.scene {
            self.record_selection("Selection".into(), before);
        }
        Ok(())
    }

    /// A camera, light or Planar locator channel edited from the live
    /// item tool (CL-14, planar_texture.rs). Kept out of `dispatch` so its
    /// scene copies don't grow that recursive function's frame.
    #[inline(never)]
    fn tool_item_channel(
        &mut self,
        item: u64,
        edit: &crate::camera_light::ItemChannelEdit,
    ) -> Result<()> {
        use crate::camera_light::ItemChannelEdit;
        let refusal = || -> String {
            if matches!(edit, ItemChannelEdit::Transform(_)) {
                "Use Move, Rotate, Scale or Transform on the Texture Locator".into()
            } else {
                "Use Move, Rotate or Transform on the camera or light".into()
            }
        };
        let live = self.live.as_ref().ok_or_else(refusal)?;
        let tool_ok = match edit {
            ItemChannelEdit::Transform(_) => live.tool.transforms_items(),
            _ => crate::camera_light::tool_shows_item_channels(live.tool),
        };
        if !tool_ok
            || self.scene.selection.mode != Mode::Item
            || self.scene.selection.item != Some(item)
        {
            return Err(refusal());
        }
        // Both the scene and the tool's base, so the tool's next
        // preview (built from its base) keeps the change.
        let mut scene = Box::new(self.scene.clone());
        scene.apply_item_channel(item, edit)?;
        let mut base = Box::new(live.base.clone());
        match edit {
            ItemChannelEdit::Transform(t) => {
                // PD-Q06: one item only; several selected move about a
                // shared pivot, which this doesn't invert.
                if live.base.item_transform_targets().is_some() {
                    return Err("Select only the Texture Locator to drag its projection".into());
                }
                let b = crate::planar_texture::base_for_tool(t, live.tool, &live.params)?;
                base.apply_item_channel(item, &ItemChannelEdit::Transform(b))?;
            }
            _ => base.apply_item_channel(item, edit)?,
        }
        scene.validate()?;
        self.scene = *scene;
        if let Some(live) = self.live.as_mut() {
            live.base = *base;
        }
        Ok(())
    }

    /// Re-target a live transform tool to a new item selection (see the
    /// route in `dispatch`). Kept out of `dispatch` so the staged copy
    /// doesn't grow every frame of that recursive function.
    #[inline(never)]
    fn retarget_transform(&mut self, tool: Tool, cmd: Command) -> Result<()> {
        let live = self.live.as_ref().ok_or("No live tool")?;
        let negative_scale = live.params.negative_scale;
        let tweak_options = live.params.tweak_options;
        let mut staged = Box::new(self.clone());
        staged.dispatch(Command::Drop)?;
        if staged.has_active_task() || staged.i2_active() {
            return Err("Finish the current tool before changing the item selection".into());
        }
        staged.dispatch(cmd)?;
        if staged.dispatch(Command::Activate(tool)).is_ok()
            && let Some(live) = staged.live.as_mut()
        {
            live.params.negative_scale = negative_scale;
            live.params.tweak_options = tweak_options;
        }
        *self = *staged;
        Ok(())
    }

    /// Select a scene item (or none).
    fn handle_select_item(&mut self, id: Option<u64>) -> Result<()> {
        self.require_idle()?;
        if id.is_some_and(|id| !self.scene.item(id).is_some()) {
            return Err("Missing mesh item".into());
        }
        self.scene.select_item(id);
        Ok(())
    }

    /// Sketch Extrude along an explicit world path in one step.
    fn handle_sketch_extrude_once(&mut self, path: Vec<DVec3>) -> Result<()> {
        if self.live.is_some() {
            return Err("Finish or Cancel the active tool first".into());
        }
        if path.len() < 2 {
            return Err("Sketch Extrude requires a nonzero path".into());
        }
        let mut staged = self.clone();
        staged.dispatch(Command::Activate(Tool::SketchExtrude))?;
        let mut params = staged.live.as_ref().unwrap().params.clone();
        params.sketch_path = path;
        staged.dispatch(Command::Preview(params))?;
        staged.dispatch(Command::Drop)?;
        *self = staged;
        Ok(())
    }

    /// Change the live Extrude stage's numeric parameters.
    fn handle_configure_extrude(&mut self, p: Parameters) -> Result<()> {
        let live = self.live.as_ref().ok_or("Arm Extrude first")?;
        if live.tool != Tool::Extrude {
            return Err("Arm Extrude first".into());
        }
        if p.extrude_uvs != live.params.extrude_uvs
            && (!p.extrude_uv_selected
                || !p.extrude_uv_map.as_ref().is_some_and(|name| {
                    self.scene
                        .active()
                        .is_some_and(|i| i.mesh.uv_maps.contains_key(name))
                }))
        {
            return Err("Select a UV map before changing Make UVs".into());
        }
        if !p.extent.is_finite() || p.extrude_sides == 0 || p.extrude_sides > 1_000_000 {
            return Err("Invalid Extrude numeric configuration".into());
        }
        if live.applied {
            self.preview(p.clone(), false)?;
        } else {
            self.live.as_mut().unwrap().params = p.clone();
        }
        self.extrude_preset = p;
        Ok(())
    }
    /// Stage the drop in a clone. The live task, history and revision remain
    /// owned by this editor until the atomic filesystem write succeeds.
    pub fn save_committed(&mut self, path: &std::path::Path) -> Result<()> {
        if self.live.as_ref().is_some_and(|l| {
            l.params
                .radial
                .is_some_and(|p| p.radius.min_element() == 0.)
        }) {
            return Err("Flat Sphere/Cylinder draft has no committed topology; enter positive Radius values or Cancel before Save".into());
        }
        let mut staged = self.clone();
        staged.dispatch(Command::Drop)?;
        if let Err(e) = crate::persistence::save(path, &staged.scene) {
            return Err(format!("Save {}: {e}", path.display()));
        }
        *self = staged;
        Ok(())
    }
    pub fn require_idle(&self) -> Result<()> {
        if self.bridge.is_some() {
            return Err("Drop or Cancel Bridge first".into());
        }
        if self.reduction.is_some() {
            return Err("Drop or Cancel Polygon Reduction first".into());
        }
        if self.live.is_some()
            || self.cone.is_some()
            || self.ellipsoid.is_some()
            || self.shape.is_some()
            || self.spike.is_some()
        {
            Err("Drop or Cancel the active tool first".into())
        } else {
            Ok(())
        }
    }
    /// PM-UP01..06: stage geometry, destination and validation before publishing
    /// anything. Do not activate/drop a tool or overwrite remembered dimensions.
    fn insert_unit(&mut self, tool: Tool, destination: UnitDestination) -> Result<()> {
        self.require_idle()?;
        if self.selection_start.is_some() {
            return Err("Finish the selection gesture before unit insertion".into());
        }
        if !tool.is_primitive() {
            return Err("Unit insertion requires Cube, Sphere or Cylinder".into());
        }
        self.scene.validate()?;
        let mut scene = self.scene.clone();
        if destination == UnitDestination::NewMesh {
            let next = scene.next_id.max(self.next_item_id);
            if next >= u64::MAX - 1 || scene.items.len() >= crate::MAX_SCENE_ITEMS {
                return Err("Scene item/identity budget exhausted".into());
            }
            scene.next_id = next;
            scene.new_mesh()?;
            // The new item is named after the shape (unit-primitive rule 10).
            scene.active_mut().ok_or("Missing new mesh")?.name = tool.label().into();
        }
        let item = scene
            .active()
            .ok_or("Select an eligible mesh for unit insertion; Shift-click creates a new mesh")?;
        if item.backdrop.is_some() {
            return Err(BACKDROP_NO_GEOMETRY.into());
        }
        if !item.is_mesh() {
            return Err(LOCATOR_NO_GEOMETRY.into());
        }
        if !scene.shows(item) {
            return Err("Unit insertion requires one visible mesh".into());
        }
        let item_id = item.id;
        let make_uvs = !self.primitive_uvs_off[primitive_slot(tool)];
        // Unit primitives ignore the tools' last-used settings and use their
        // reset shapes (unit-primitive-2026-11-05 rules 9 and 11).
        let geometry = if tool == Tool::Box {
            let mut mesh = crate::primitive::segmented_cube(DVec3::ZERO, DVec3::ONE, [1, 1, 1])?;
            if make_uvs {
                crate::add_cube_uvs(&mut mesh, DVec3::ZERO, DVec3::ONE)?;
            }
            mesh
        } else {
            let mut radial = tool
                .initial_radial()
                .ok_or("Missing unit primitive profile")?;
            // The unit Cylinder is 1.5 m tall like the tool's reset size.
            radial.radius = if tool == Tool::Cylinder {
                DVec3::new(0.5, 0.75, 0.5)
            } else {
                DVec3::splat(0.5)
            };
            radial_mesh(
                tool,
                DVec3::ZERO,
                radial,
                &Parameters {
                    make_uvs,
                    ..Default::default()
                },
            )?
        };
        geometry.validate()?;
        let geometry =
            crate::user_defaults::primitive_symmetry(self.active_symmetry()).doubled(geometry)?;
        // Made at the world origin like the drawn tools, then mapped into a
        // moved mesh's local space (provisional, PRIM-MOVED-Q01).
        let geometry = scene.mesh_world_to_local(item_id, geometry)?;
        crate::append_geometry(
            &mut scene.active_mut().ok_or("Select a mesh")?.mesh,
            geometry,
        )?;
        scene.validate()?;
        let before = std::mem::replace(&mut self.scene, scene);
        if destination == UnitDestination::NewMesh {
            self.next_item_id = self.scene.next_id;
        }
        self.record(format!("Unit {}", tool.label()), before);
        Ok(())
    }
    /// The Symmetry setting in effect: with Work Plane on, its plane runs
    /// through the work plane centre (SYM-B2-08).
    pub fn active_symmetry(&self) -> crate::Symmetry {
        self.symmetry.through(self.scene.workplane.center)
    }
    /// SYM3-30: the mirror counterpart of a picked component, when
    /// Symmetry is on and it differs from the pick.
    fn mirror_pick(&self, item: u64, hit: crate::Component) -> Option<crate::Component> {
        self.scene
            .symmetric_counterpart(item, hit, self.active_symmetry())
            .filter(|&m| m != hit)
    }
    /// The live tool's frame from the pivot service (pivot_service.rs):
    /// the rulebook's centre and axes, under Symmetry the + half's centre
    /// (ACS-01) with the axes of the whole selection (ACS-02).
    fn pivot_frame(&self, scene: &Scene, tool: Tool) -> Result<crate::ActionFrame> {
        self.pivot_request(scene, tool).frame()
    }
    /// The whole active falloff (the active one, the ones added on top and
    /// the selected weight map), or `None` with Falloff None.
    pub fn active_falloff(&self) -> Option<crate::ActiveFalloff> {
        Some(crate::ActiveFalloff {
            first: self.spatial_falloff?,
            added: self.added_falloffs.clone(),
            weight_map: self.selected_weight_map.clone(),
            element: self.falloff_element,
            symmetry: self.active_symmetry(),
            stamps: self.falloff_stamps.clone(),
            path: self.falloff_path.clone(),
        })
    }
    /// The action axes Bend leans along (World when the action axis can't
    /// be had for this selection, e.g. Selection axes without polygons).
    /// BN-45 (BH-Q12, provisional): the status bar text while Bend's axis
    /// runs along its spine, so the bend can't move anything.
    pub fn bend_notice(&self) -> Option<&'static str> {
        let live = self.live.as_ref().filter(|l| l.tool == Tool::Bend)?;
        live.params
            .bend
            .axis_along_spine(self.bend_axes(&self.scene))
            .then_some(crate::bend::AXIS_ALONG_SPINE)
    }
    pub fn bend_axes(&self, scene: &Scene) -> glam::DMat3 {
        self.live
            .as_ref()
            .and_then(|l| l.action_frame)
            .or_else(|| self.action_context.resolve(scene).ok())
            .map_or(glam::DMat3::IDENTITY, |f| f.basis)
    }
    /// Falloff weight of every point of item `id`; `None` with no falloff.
    /// An Element falloff is in the stack (FO-45).
    pub(crate) fn has_element_falloff(&self) -> bool {
        self.spatial_falloff
            .iter()
            .chain(self.added_falloffs.iter().map(|a| &a.0))
            .any(|f| matches!(f, crate::Falloff::Element(_)))
    }
    pub(crate) fn falloff_weights(&self, scene: &Scene, id: u64) -> Result<Option<Vec<f64>>> {
        let Some(field) = self.active_falloff() else {
            return Ok(None);
        };
        field.weights(scene, id).map(Some)
    }
    pub(crate) fn record(&mut self, label: String, before: Scene) {
        // TK-30/31: auto key, in the same step as the edit.
        self.auto_key_from(&before);
        // Mesh Instances share their sources' meshes again (an edit made
        // on an instance reaches its source and the other instances).
        self.scene.sync_instances(Some(&before));
        // An edit that removed edges takes them out of saved edge sets.
        crate::selection_sets::prune_edge_sets(&mut self.scene);
        // Other active meshes keep components only while selected and
        // unedited (multi_active.rs), or as a tool that ran on them left
        // them (multi_tools.rs).
        let fresh = std::mem::take(&mut self.multi_fresh);
        self.scene
            .prune_component_selections_keeping(Some(&before), &fresh);
        self.multi_fresh = fresh;
        // One gesture, one step: a later edit of the same gesture replaces
        // the previous step's result when nothing came in between.
        if let Some(gesture) = self.undo_fold
            && self.redo.is_empty()
            && let Some(last) = self.undo.last_mut().map(std::sync::Arc::make_mut)
            && last.fold == Some(gesture)
            && last.after == before
        {
            last.after = self.scene.clone();
            let pending = std::mem::take(&mut self.pending_records.0);
            last.records
                .extend_capped(pending, self.history.max_sub_commands as usize);
            self.history_serial = self.history_serial.wrapping_add(1);
            self.history_mark += 1;
            return;
        }
        self.i2_branch();
        self.history_mark += 1;
        self.undo.push(std::sync::Arc::new(Checkpoint {
            identity_before: self.identities.record_before.clone(),
            identity_after: None,
            label,
            before,
            after: self.scene.clone(),
            fold: self.undo_fold,
            records: std::mem::take(&mut self.pending_records),
            mark: self.history_mark,
        }));
        // Max Undo Level: the oldest steps go once a new one exceeds it.
        // A lowered limit trims here, on the next step (UND-Q2, provisional).
        let excess = self
            .undo
            .len()
            .saturating_sub(self.history.max_undo_level.max(1));
        self.undo.drain(..excess);
        self.trim_undo_memory();
        self.redo.clear();
        self.selection_run = 0;
        if let Some(n) = self.tool_steps.as_mut() {
            *n += 1;
        }
        self.history_serial = self.history_serial.wrapping_add(1);
    }
    /// Drops the oldest undo steps while the history's item copies pass
    /// `UNDO_MEMORY_BUDGET`, keeping at least the newest step. Each step
    /// keeps two whole copies of the item list; meshes are shared, so the
    /// items themselves are the cost that grows with the scene (100 steps
    /// of a 100,000-item scene were about 35 GB). Provisional
    /// (scene-budget, 2026-11-20).
    pub(crate) fn trim_undo_memory(&mut self) {
        if self.undo_trim_paused {
            return;
        }
        self.trim_undo_memory_to(self.undo_memory_budget.unwrap_or(UNDO_MEMORY_BUDGET));
    }
    /// `trim_undo_memory` with another budget, in bytes.
    /// The history counts the mesh data each step alone keeps alive
    /// (undo_memory.rs, limits sweep A1), and is held to at most half of
    /// the memory free beside it, when a free-memory source is set. The
    /// newest step is always kept.
    pub fn trim_undo_memory_to(&mut self, budget: usize) {
        let costs = crate::undo_memory::step_costs(&self.scene, &self.undo, &self.redo);
        let mut total: usize = costs.iter().sum();
        let budget = match self.free_memory {
            crate::FreeMemory(Some(free)) => {
                let free = usize::try_from(free()).unwrap_or(usize::MAX);
                budget.min(total.saturating_add(free) / 2)
            }
            crate::FreeMemory(None) => budget,
        };
        let mut excess = 0;
        while total > budget && excess + 1 < self.undo.len() {
            total -= costs[excess];
            excess += 1;
        }
        self.undo.drain(..excess);
    }
    /// The history's mark now; pass it to `recorded_since` or
    /// `steps_since` after running something (Astra P4-003: "did that
    /// record?" was guessed from `undo.len()` growing, which stops working
    /// once Max Undo Level trims a step for every new one).
    pub fn history_mark(&self) -> u64 {
        self.history_mark
    }
    /// Whether recording changed the undo history since `mark`: a new step,
    /// an edit folded into the newest one, or a joined selection change.
    pub fn recorded_since(&self, mark: u64) -> bool {
        self.history_mark != mark
    }
    /// How many undo steps are newer than `mark` (still in the history).
    pub fn steps_since(&self, mark: u64) -> usize {
        self.undo.iter().rev().take_while(|c| c.mark > mark).count()
    }
    /// For a command that sets its result selection after recording its
    /// step: the step recorded since `mark` ends with that selection, so
    /// Redo gives it back (Astra P4-002).
    pub(crate) fn amend_step_selection(&mut self, mark: u64) {
        if self.steps_since(mark) > 0
            && let Some(step) = self.undo.last_mut().map(std::sync::Arc::make_mut)
        {
            step.after.selection = self.scene.selection.clone();
        }
    }
    /// The bytes the undo history holds beyond the scene and redo list.
    pub fn undo_memory_bytes(&self) -> usize {
        crate::undo_memory::step_costs(&self.scene, &self.undo, &self.redo)
            .iter()
            .sum()
    }
    /// Action frame of a tool session, resolved against world-space positions
    /// for component editing (shared by the editor and viewport handles).
    pub fn extrude_frame(
        context: crate::ActionContext,
        tool: Tool,
        scene: &Scene,
    ) -> Result<crate::ActionFrame> {
        // Parent and Pivot centres come from the item hierarchy (ACS
        // table), before the world transform is baked in below.
        if scene.selection.mode != Mode::Item
            && let Some(frame) = crate::pivot_service::item_pivot_frame(context, scene)
        {
            return frame;
        }
        // Item-mode transforms use the item frame; every component tool
        // resolves its frame against world-space positions.
        let transform = tool.transforms_items();
        if scene.selection.mode == Mode::Item && tool != Tool::Extrude && !tool.is_smooth_shift() {
            // The handle sits where rigged items show (RG-01), not at
            // their own unconstrained place.
            let rigged = (!scene.animation.rig.pieces.is_empty())
                .then(|| crate::rig_eval::apply(scene, &crate::rig_eval::evaluate(scene)))
                .flatten();
            let scene = rigged.as_ref().unwrap_or(scene);
            let frame = context.resolve(scene)?;
            // Several items: the handle at their shared pivot.
            if transform
                && let Some(targets) = scene.item_transform_targets()
                && !targets.is_empty()
            {
                return Ok(crate::ActionFrame {
                    center: scene.item_pivots(&targets)?[0],
                    ..frame
                });
            }
            return Ok(frame);
        }
        // Several active meshes: one frame over all their selections.
        if (transform || crate::multi_tools::tool_runs_on_each_mesh(tool))
            && let Some(targets) = scene.component_transform_meshes()
        {
            return context.resolve(&scene.combined_component_probe(&targets)?);
        }
        let item = scene.active().ok_or("Select a mesh")?;
        let world = scene.world_matrix(item.id)?;
        let mut basis_scene = scene.clone();
        let item = basis_scene.active_mut().unwrap();
        for v in &mut item.mesh.vertices {
            *v = world.transform_point3(*v);
        }
        item.transform = Default::default();
        item.parent = None;
        let frame = context.resolve(&basis_scene)?;
        if !context.center.uses_selection_box() {
            return Ok(frame);
        }
        // The selection's box is taken in the item's own space and its
        // middle placed in the world, so a turned item's box turns with
        // it (AC-rotate-Selection; AC-Q20 provisional).
        let item = scene.active().ok_or("Select a mesh")?;
        let local = crate::box_centre(
            crate::transform_vertices(scene)?
                .iter()
                .map(|&v| item.mesh.vertices[v as usize]),
        );
        Ok(crate::ActionFrame {
            center: world.transform_point3(local),
            ..frame
        })
    }
    /// BN-30/32: put Bend's centre (the action centre, `bend.base`) at a
    /// world point. The ring and the bend start there; the spine keeps its
    /// vector, so its end moves too (BH-Q05, provisional). Works before and
    /// after the bend has been applied.
    /// Move Bend's centre to `point`. With `refit` (a click in a view on
    /// that work plane, BN-24/BN-30b revised 2026-11-16) the spine is
    /// refitted over the target and the Axis goes to the plane's normal;
    /// without it (scripts, `SetActionCenter`) only the centre moves.
    fn set_bend_center(&mut self, point: DVec3, refit: Option<u8>) -> Result<()> {
        if !point.is_finite() || point.abs().max_element() > 1e9 {
            return Err("The center must be a finite point".into());
        }
        let live = self.live.as_ref().ok_or("No active tool")?;
        let mut p = live.params.clone();
        if let Some(plane) = refit {
            // The target is the scene the tool started from, so the bend
            // in progress doesn't change the box; with Symmetry on, the
            // side the user works on (SYM3-12).
            let one_side =
                crate::symmetry_sides::primary_side_scene(&live.base, self.active_symmetry());
            let target = one_side.as_ref().unwrap_or(&live.base);
            let axes = live.action_frame.map_or(glam::DMat3::IDENTITY, |f| f.basis);
            let fitted = crate::BendOptions::fitted(target, point, plane, axes)?;
            p.bend.spine = fitted.spine;
            p.bend.axis = fitted.axis;
        }
        p.bend.base = point;
        let live = self.live.as_mut().ok_or("No active tool")?;
        if live.applied {
            self.preview(p, false)
        } else {
            live.params = p;
            if let Some(frame) = live.action_frame.as_mut() {
                frame.center = point;
            }
            Ok(())
        }
    }
    pub(crate) fn preview(&mut self, p: Parameters, apply: bool) -> Result<()> {
        // Bend's centre is the action centre: the frame follows `bend.base`
        // however it changed (panel fields, a click, undo inside the tool).
        let bend_center =
            (self.live.as_ref().is_some_and(|l| l.tool == Tool::Bend)).then_some(p.bend.base);
        let result = self.preview_tool(p, apply);
        if result.is_ok()
            && let Some(center) = bend_center
            && let Some(frame) = self.live.as_mut().and_then(|l| l.action_frame.as_mut())
        {
            frame.center = center;
        }
        // Back to the mode the selection was made in, with that selection
        // (SI-05/06), whether or not the preview worked.
        self.keep_inferred_source();
        result?;
        // Constrain to Background acts after the tool's own change.
        self.constrain_preview()
    }
    fn preview_tool(&mut self, p: Parameters, apply: bool) -> Result<()> {
        let live = self.live.as_ref().ok_or("No active tool")?;
        // Started from a UV view: the pipe runs on the map's UVs (UVT-130,
        // uv_pipe.rs).
        if p.uv.is_some() && crate::uv_pipe::runs_on_uvs(live.tool) {
            return self.preview_uv(p, apply);
        }
        // Several active meshes: the tool runs on each (multi_tools.rs).
        if let Some(targets) = Self::tool_mesh_targets(live.tool, &live.base) {
            return self.preview_on_meshes(&targets, p, apply);
        }
        // Tweak grabs an element of any active mesh and edits that mesh.
        if live.tool == Tool::Tweak
            && let Some(grab) = p.tweak
            && live.base.is_other_active_mesh(grab.item)
        {
            return self.preview_on_meshes(&[grab.item], p, apply);
        }
        self.preview_one(p, apply)
    }
    /// One live tool preview on the primary mesh.
    pub(crate) fn preview_one(&mut self, mut p: Parameters, apply: bool) -> Result<()> {
        let inferred = self.inferred_mode();
        let live = self.live.as_ref().ok_or("No active tool")?;
        let mut scene = live.base.clone();
        // An inferred start: the tool and every layer see its own mode.
        if let Some(mode) = inferred {
            scene.selection.mode = mode;
        }
        // Twist Bevel's note about a group across the symmetry plane.
        let mut twist_note: Option<String> = None;
        match live.tool {
            Tool::Sphere | Tool::Cylinder => {
                let radial = p.radial.ok_or("Missing primitive Radius/resolution")?;
                // The flat first-draw base is a real disc that Drop keeps
                // (drag-creation-2026-11-06 rules 1 and 4; `flat_disc`).
                if !radial.radius.is_finite() || radial.radius.min_element() < 0. {
                    return Err("Radius must be finite and nonnegative".into());
                }
                let mesh = crate::user_defaults::primitive_symmetry(self.symmetry)
                    .doubled(radial_mesh(live.tool, p.position, radial, &p)?)?;
                let item = scene.active().ok_or("Select a mesh")?;
                if !scene.shows(item) {
                    return Err(
                        "The selected mesh is hidden; show it before creating geometry".into(),
                    );
                }
                // Drawn in world space; the mesh is local.
                let mesh = scene.mesh_world_to_local(item.id, mesh)?;
                crate::append_geometry(&mut scene.active_mut().ok_or("Select a mesh")?.mesh, mesh)?;
            }
            Tool::Box => {
                if !apply
                    && !live.applied
                    && p.position == live.params.position
                    && p.size == live.params.size
                    && p.segments == live.params.segments
                    && p.make_uvs == live.params.make_uvs
                {
                    self.live.as_mut().ok_or("No active primitive")?.params = p;
                    return Ok(());
                }
                let mut mesh = crate::primitive::segmented_cube(p.position, p.size, p.segments)?;
                if p.make_uvs && !mesh.faces.is_empty() {
                    crate::add_cube_uvs(&mut mesh, p.position, p.size)?;
                }
                // SYM-PR-01/02.
                let mesh = crate::user_defaults::primitive_symmetry(self.active_symmetry())
                    .doubled(mesh)?;
                let item = scene.active().ok_or("Select an item")?;
                if !scene.shows(item) {
                    return Err(
                        "The selected mesh is hidden; show it before creating geometry".into(),
                    );
                }
                // Drawn in world space (UVs too); the mesh is local.
                let mesh = scene.mesh_world_to_local(item.id, mesh)?;
                let item = scene.active_mut().ok_or("Select an item")?;
                crate::primitive::append_geometry(&mut item.mesh, mesh)?;
            }
            Tool::EdgeExtend
                if self.active_symmetry().axis.is_some()
                    && (p.rotation_delta != DVec3::ZERO || p.scale_delta != DVec3::ONE) =>
            {
                // N27 (Jason, 2026-11-17): with Symmetry, Rotate and Scale
                // turn each half about its own centre, the far half
                // mirrored, instead of both about the point between them.
                let frame = live.action_frame.ok_or("Missing action frame")?;
                let at_zero = self.extend_from_click;
                let symmetry = self.active_symmetry();
                crate::symmetry_sides::two_pass(&mut scene, symmetry, |scene, mirrored| {
                    let edges = scene.selection.edges.clone();
                    let item = scene.active().ok_or("Select a mesh for Extend")?;
                    let world = scene.world_matrix(item.id)?;
                    let mut points: Vec<u32> = edges.iter().flat_map(|&(a, b)| [a, b]).collect();
                    points.sort_unstable();
                    points.dedup();
                    let centre = points
                        .iter()
                        .filter_map(|&v| item.mesh.vertices.get(v as usize))
                        .map(|&q| world.transform_point3(q))
                        .sum::<DVec3>()
                        / points.len().max(1) as f64;
                    let mirror = crate::symmetry_sides::WorldMirror::new(symmetry, world)
                        .filter(|_| mirrored);
                    let (basis, offset) = match &mirror {
                        // A mirrored basis gives the mirror image of the
                        // same rotate and scale (M·B·R·S·Bᵀ·M).
                        Some(m) => (
                            glam::DMat3::from_mat4(m.matrix) * frame.basis,
                            m.vector(frame.basis * p.delta),
                        ),
                        None => (frame.basis, frame.basis * p.delta),
                    };
                    let shape = crate::ExtendShape {
                        offset,
                        inset: p.inset,
                        shift: p.shift,
                        rotate: p.rotation_delta,
                        scale: p.scale_delta,
                        frame: crate::ActionFrame {
                            center: centre,
                            basis,
                        },
                        segments: p.segments[0],
                        make_uvs: p.edge_extend.make_uvs,
                    };
                    let (mesh, outer) = if at_zero {
                        crate::edge_extend_at_zero(&item.mesh, &edges, world, &shape)?
                    } else {
                        crate::edge_extend_shaped(&item.mesh, &edges, world, &shape)?
                    };
                    scene.active_mut().ok_or("Select a mesh")?.mesh = mesh.into();
                    // PF-12: the new edges' points are the point selection too, so
                    // Vertex mode shows them.
                    scene.selection.vertices = crate::topology::edge_points(&outer);
                    scene.selection.edges = outer;
                    Ok(())
                })?;
            }
            Tool::EdgeExtend => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Extend")?;
                // Offset is along the action axes (as Move's); the shape
                // works in world space and maps back to the mesh.
                let frame = live.action_frame.ok_or("Missing action frame")?;
                let shape = crate::ExtendShape {
                    offset: frame.basis * p.delta,
                    inset: p.inset,
                    shift: p.shift,
                    rotate: p.rotation_delta,
                    scale: p.scale_delta,
                    frame,
                    segments: p.segments[0],
                    make_uvs: p.edge_extend.make_uvs,
                };
                let world = scene.world_matrix(item.id)?;
                let (mut mesh, outer) = if self.extend_from_click {
                    crate::edge_extend_at_zero(&item.mesh, &edges, world, &shape)?
                } else {
                    crate::edge_extend_shaped(&item.mesh, &edges, world, &shape)?
                };
                // SYM3-12: the offset is mirrored for the partner side.
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() && shape.offset != DVec3::ZERO {
                    let primary = self.symmetry_primary(&scene);
                    let v = world.inverse().transform_vector3(shape.offset);
                    let moved: Vec<u32> = outer.iter().flat_map(|&(a, b)| [a, b]).collect();
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(&mut mesh.vertices, &moved, v, primary, tolerance);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                // PF-12: the new edges' points are the point selection too, so
                // Vertex mode shows them.
                scene.selection.vertices = crate::topology::edge_points(&outer);
                scene.selection.edges = outer;
            }
            Tool::EdgeCrease => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Weight")?;
                let mesh = crate::edge_crease_geometry(
                    &item.mesh,
                    &edges,
                    p.shift,
                    p.edge_weight_additive,
                )?;
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::EdgeSlide => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Slide")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let (mesh, after) = crate::edge_slide_extra(
                    &item.mesh,
                    &edges,
                    p.shift,
                    p.edge_slide_distance,
                    weights.as_deref(),
                    &p.slide_extras,
                )?;
                let merged = mesh.vertices.len() != item.mesh.vertices.len();
                let mut mesh = mesh;
                if !merged {
                    // SYM3-12: the slide direction is mirrored for the
                    // partner side.
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    symmetry.mirror_moved(&item.mesh, &mut mesh, primary);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                if merged {
                    // ES-04: merged points renumber the mesh.
                    scene.selection.edges = after;
                    scene.selection.vertices.clear();
                    scene.selection.faces.clear();
                }
            }
            Tool::EdgeExtrude => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Extrude")?;
                // Use Material tags the ridge quads (extrude-2026-11-05, EE08).
                let material = p.vertex_use_material.then_some(p.vertex_material.as_str());
                let (mesh, raised) =
                    crate::edge_extrude_with(&item.mesh, &edges, p.shift, p.inset, material)?;
                scene.active_mut().unwrap().mesh = mesh.into();
                // The ridge edges and their points are selected after.
                scene.selection.vertices = crate::topology::edge_points(&raised);
                scene.selection.edges = raised;
            }
            Tool::EdgeBevel | Tool::EdgeChamfer => {
                let item = scene.active().ok_or("Select a mesh for Edge Bevel")?;
                // TA-06: nothing selected bevels every edge (Edge Bevel only;
                // activation refuses Edge Chamfer without a selection).
                let edges = if live.tool == Tool::EdgeBevel && scene.selection.edges.is_empty() {
                    item.mesh.edges()
                } else {
                    scene.selection.edges.clone()
                };
                // Widths are world distances; the mesh is local (uniform
                // scale assumed for the basic profile).
                let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                let weights = self.falloff_weights(&scene, item.id)?;
                let settings = crate::EdgeBevelSettings {
                    width: p.chamfer.offset / scale,
                    rows: p.chamfer.segments,
                    depth: p.chamfer.depth,
                    chamfer: live.tool == Tool::EdgeChamfer,
                    options: p.edge_bevel.clone(),
                };
                let result =
                    crate::edge_bevel_with(&item.mesh, &edges, &settings, weights.as_deref())?;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                // EBC-J-01 / EBC-P-02: Edge Bevel selects the strips' edges,
                // Edge Chamfer nothing (the original edges with Square).
                // EBC-B3 oracles: the points selected are the ends of those
                // edges.
                scene.selection.vertices = crate::topology::edge_points(&result.selected_after);
                scene.selection.edges = result.selected_after;
            }
            Tool::VertexCreate => {
                let item = scene.active().ok_or("Select a mesh for Create")?;
                let local = scene.world_matrix(item.id)?.inverse();
                let mut points: Vec<DVec3> = p
                    .sketch_path
                    .iter()
                    .map(|&q| local.transform_point3(q))
                    .collect();
                // Class F, Weld seam (SYM3-22/23): each placed point gets a
                // mirror point; one on the plane (within the matching
                // tolerance) is made once, exactly on it.
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() {
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh).max(
                        crate::symmetry::match_tolerance(&crate::Mesh {
                            vertices: points.clone(),
                            ..Default::default()
                        }),
                    );
                    let mut mirrored = Vec::new();
                    for q in &mut points {
                        if symmetry.side(*q, tolerance) == Some(crate::Side::Plane) {
                            *q = symmetry.onto_plane(*q);
                        } else {
                            mirrored.push(symmetry.mirror(*q));
                        }
                    }
                    points.extend(mirrored);
                }
                let (mesh, ids) = crate::add_points(&item.mesh, &points)?;
                scene.active_mut().unwrap().mesh = mesh.into();
                if !ids.is_empty() {
                    scene.selection.vertices = ids;
                }
            }
            Tool::VertexBevel | Tool::VertexExtrude | Tool::VertexMerge | Tool::VertexTear => {
                let mut points = scene.selection.vertices.clone();
                let item = scene.active().ok_or("Select a mesh")?;
                // Vertex Extrude with nothing selected processes every point
                // (extrude-2026-11-05, VE08).
                if points.is_empty() && live.tool == Tool::VertexExtrude {
                    points = (0..item.mesh.vertices.len() as u32).collect();
                }
                let world = scene.world_matrix(item.id)?;
                // Distances are world lengths; the mesh is local (uniform
                // scale assumed for the basic profile).
                let scale = crate::scene::uniform_scale(world);
                let (mesh, selected) = match live.tool {
                    Tool::VertexBevel => crate::vertex_commands::vertex_bevel_round(
                        &item.mesh,
                        &points,
                        p.inset / scale,
                        p.vertex_round_level,
                        self.falloff_weights(&scene, item.id)?.as_deref(),
                    )?,
                    Tool::VertexExtrude => crate::vertex_commands::vertex_extrude_weighted(
                        &item.mesh,
                        &points,
                        p.shift / scale,
                        p.inset / scale,
                        self.falloff_weights(&scene, item.id)?.as_deref(),
                    )?,
                    Tool::VertexMerge => crate::vertex_merge_with(
                        &item.mesh,
                        &points,
                        p.shift / scale,
                        crate::MergeOptions {
                            keep_small: p.vertex_merge_keep_small,
                            test_morphs: p.vertex_merge_test_morphs,
                            ..Default::default()
                        },
                    )?,
                    _ => crate::vertex_tear_geometry(
                        &item.mesh,
                        &points,
                        &scene.selection.faces,
                        world.inverse().transform_vector3(p.delta),
                    )?,
                };
                let mut mesh = mesh;
                // SYM3-12: Tear's new points move by the drag; those torn
                // from the other side take the mirrored drag, seam points
                // keep only the part along the plane (SYM3-13).
                if live.tool == Tool::VertexTear && mesh.vertices.len() > item.mesh.vertices.len() {
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    let torn: Vec<u32> =
                        (item.mesh.vertices.len() as u32..mesh.vertices.len() as u32).collect();
                    let v = world.inverse().transform_vector3(p.delta);
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(&mut mesh.vertices, &torn, v, primary, tolerance);
                }
                // VTO-T02/T03 Use Material: tag the new polygons (every
                // polygon of a rounded cap, VX-B07).
                if p.vertex_use_material
                    && matches!(live.tool, Tool::VertexBevel | Tool::VertexExtrude)
                {
                    let new = crate::vertex_commands::new_vertex_tool_polygons(&item.mesh, &mesh);
                    crate::vertex_commands::assign_material(&mut mesh, new, &p.vertex_material);
                }
                let same_polygons = item.mesh.faces == mesh.faces;
                scene.active_mut().unwrap().mesh = mesh.into();
                // Tear keeps polygon order, so its polygon selection stays.
                if !same_polygons && live.tool != Tool::VertexTear {
                    scene.selection.edges.clear();
                    scene.selection.faces.clear();
                }
                scene.selection.vertices = selected;
            }
            Tool::LoopSlice
            | Tool::EdgeSlice
            | Tool::PolygonSlice
            | Tool::CurveSlice
            | Tool::PenSlice
            | Tool::PrimitiveSlice
            | Tool::AxisSlice => {
                crate::slice::preview(live.tool, &mut scene, &p.slice, self.active_symmetry())?
            }
            Tool::Contour => crate::contour::preview(&mut scene, &p.contour)?,
            Tool::Mirror
            | Tool::Lathe
            | Tool::Clone
            | Tool::Array
            | Tool::RadialArray
            | Tool::CurveExtrude
            | Tool::CurveClone
            | Tool::ScatterClone
            | Tool::Symmetrize => crate::duplicate::preview(
                live.tool,
                &mut scene,
                &p.duplicate,
                self.next_item_id,
                self.active_symmetry(),
            )?,
            Tool::VertexSlide => {
                let points = scene.selection.vertices.clone();
                let item = scene.active().ok_or("Select a mesh for Slide")?;
                let world = scene.world_matrix(item.id)?;
                let rail = world.inverse().transform_vector3(p.delta);
                // Distance is a world length; the mesh is local (uniform
                // scale assumed, as for the other vertex tools).
                let amount = if p.edge_slide_distance {
                    p.shift / crate::scene::uniform_scale(world)
                } else {
                    p.shift
                };
                let (mesh, selected) = crate::vertex_commands::vertex_slide_extra(
                    &item.mesh,
                    &points,
                    rail,
                    amount,
                    crate::vertex_commands::SlideOptions {
                        by_distance: p.edge_slide_distance,
                        merge: p.vertex_slide_merge,
                    },
                    &p.slide_extras,
                )?;
                let merged = mesh.vertices.len() != item.mesh.vertices.len();
                let mut mesh = mesh;
                if !merged {
                    // SYM3-12: the rail is mirrored for the partner side.
                    let symmetry = self.active_symmetry();
                    let primary = self.symmetry_primary(&scene);
                    symmetry.mirror_moved(&item.mesh, &mut mesh, primary);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
                // Merged points renumber the mesh; edge and polygon picks
                // would point at the wrong components.
                if merged {
                    scene.selection.edges.clear();
                    scene.selection.faces.clear();
                }
                scene.selection.vertices = selected;
            }
            Tool::Push | Tool::Smooth => {
                let points = crate::transform_vertices(&scene)?;
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let mesh = if live.tool == Tool::Push {
                    // Distance is a world length; the mesh is local (uniform
                    // scale assumed, as for Vertex Extrude).
                    let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                    let faces = (scene.selection.mode == Mode::Polygon)
                        .then_some(scene.selection.faces.as_slice());
                    // Element Push (hands-on 2026-11-17): with an Element
                    // falloff the picked element's normal is the one
                    // direction, so a polygon moves as one flat piece and
                    // its Range neighbours follow it.
                    let element = self
                        .falloff_element
                        .filter(|g| g.item == item.id)
                        .filter(|_| self.has_element_falloff())
                        .map(|g| crate::deform::element_normal(&item.mesh, g.element));
                    match element {
                        Some(dir) => crate::deform::push_along(
                            &item.mesh,
                            &points,
                            dir,
                            p.shift / scale,
                            weights.as_deref(),
                        )?,
                        None => crate::deform::push_geometry(
                            &item.mesh,
                            &points,
                            p.shift / scale,
                            p.push_normals,
                            faces,
                            weights.as_deref(),
                        )?,
                    }
                } else {
                    crate::deform::smooth_geometry(
                        &item.mesh,
                        &points,
                        &p.smooth,
                        weights.as_deref(),
                    )?
                };
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::UvProject
            | Tool::UvViewProject
            | Tool::UvUnwrap
            | Tool::UvRelax
            | Tool::UvPelt
            | Tool::UvPeeler => {
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                if let Some(mesh) =
                    crate::uv_relax::uv_tool_geometry(live.tool, &scene, &p, weights.as_deref())?
                {
                    scene.active_mut().unwrap().mesh = mesh.into();
                }
            }
            // UVT-43: on the UV proxy (the pipe swapped it in); selected UV
            // positions move by the box's edit, weighted by the falloff.
            Tool::UvBoxTransform => {
                if p.uv.is_none() {
                    return Err("Box Transform works in a UV view".into());
                }
                p.uv_box.validate()?;
                let item = scene.active().ok_or("Select a mesh")?;
                let id = item.id;
                let weights = self.falloff_weights(&scene, id)?;
                let points = crate::transform_vertices(&scene)?;
                let (lo, hi) = crate::uv_box::bounds(&item.mesh.vertices, points.iter().copied())
                    .ok_or("Box Transform: no UVs selected")?;
                let mut mesh: crate::Mesh = (*item.mesh).clone();
                for &v in &points {
                    let q = mesh.vertices[v as usize];
                    let to = p.uv_box.apply(q.truncate(), lo, hi);
                    let w = weights
                        .as_ref()
                        .map_or(1., |w| w.get(v as usize).copied().unwrap_or(0.));
                    let moved = q.truncate().lerp(to, w);
                    mesh.vertices[v as usize] = DVec3::new(moved.x, moved.y, q.z);
                }
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::Jitter | Tool::Quantize | Tool::RadialAlign | Tool::LinearAlign => {
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let mesh =
                    crate::deform_finish::tool_geometry(live.tool, &scene, &p, weights.as_deref())?;
                // Symmetry: the primary side's result, mirrored (SYM3-12).
                let mut mesh = mesh;
                let symmetry = self.active_symmetry();
                let primary = self.symmetry_primary(&scene);
                let base = &scene.active().unwrap().mesh;
                symmetry.mirror_moved(base, &mut mesh, primary);
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::SymmetryTool => {
                let item = scene
                    .active()
                    .ok_or("Select a mesh for the Symmetry Tool")?;
                // Threshold is a world distance; the mesh is local (uniform
                // scale assumed, as for the other tools).
                let scale = scene
                    .world_matrix(item.id)?
                    .transform_vector3(DVec3::X)
                    .length()
                    .max(1e-12);
                let o = p.symmetry_fix;
                let fixed = crate::symmetry_fix::symmetry_fix(
                    &item.mesh,
                    self.active_symmetry(),
                    o.threshold / scale,
                    o.topology,
                    o.reverse,
                )?;
                self.symmetry_unpaired = fixed.unpaired;
                scene.active_mut().unwrap().mesh = fixed.mesh.into();
            }
            Tool::Bend => {
                let points = crate::transform_vertices(&scene)?;
                let item = scene.active().ok_or("Select a mesh")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let world = scene.world_matrix(item.id)?;
                let axes = self.bend_axes(&scene);
                let direction = p.bend.direction(axes);
                // BN-45 (BH-Q12, provisional): an axis along the spine can't
                // turn it; the mesh stays as it was (see `bend_notice`).
                let bend = if p.bend.axis_along_spine(axes) {
                    crate::BendOptions {
                        angle: 0.,
                        ..p.bend
                    }
                } else {
                    p.bend
                };
                // SYM3-12: the side the user works on bends as entered, the
                // other side with the mirrored base, spine and lean; seam
                // points stay on the plane (SYM3-13).
                let symmetry = self.active_symmetry();
                let primary = self.symmetry_primary(&scene);
                let list: Vec<u32> = points.iter().copied().collect();
                let split =
                    crate::symmetry_sides::split_points(&item.mesh, symmetry, &list, primary);
                let mirror = crate::symmetry_sides::WorldMirror::new(symmetry, world);
                let mesh = match mirror.filter(|_| split.is_two_sided()) {
                    None => crate::bend::bend_geometry(
                        &item.mesh,
                        &points,
                        world,
                        &bend,
                        direction,
                        weights.as_deref(),
                    )?,
                    Some(m) => {
                        let first: std::collections::BTreeSet<u32> =
                            split.primary.iter().copied().collect();
                        let second: std::collections::BTreeSet<u32> =
                            split.other.iter().copied().collect();
                        let mut mesh = crate::bend::bend_geometry(
                            &item.mesh,
                            &first,
                            world,
                            &bend,
                            direction,
                            weights.as_deref(),
                        )?;
                        let mirrored = crate::BendOptions {
                            base: m.point(bend.base),
                            spine: m.vector(bend.spine),
                            ..bend
                        };
                        mesh = crate::bend::bend_geometry(
                            &mesh,
                            &second,
                            world,
                            &mirrored,
                            m.vector(direction),
                            weights.as_deref(),
                        )?;
                        let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                        for &v in &first {
                            if symmetry.side(item.mesh.vertices[v as usize], tolerance)
                                == Some(crate::Side::Plane)
                            {
                                let q = &mut mesh.vertices[v as usize];
                                *q = symmetry.onto_plane(*q);
                            }
                        }
                        mesh
                    }
                };
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::EdgeRelax => {
                let edges = scene.selection.edges.clone();
                let item = scene.active().ok_or("Select a mesh for Edge Relax")?;
                let mesh = crate::edge_relax_geometry(&item.mesh, &edges, p.shift)?;
                scene.active_mut().unwrap().mesh = mesh.into();
            }
            Tool::Chamfer => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh for Chamfer Edit")?;
                let output = crate::chamfer_geometry(&item.mesh, &selected, p.chamfer)?;
                // Rule 11: the selection stays when no polygons were added
                // or removed (CE02, CE10); a rebuilt strip clears it.
                let rebuilt = output.faces.len() != item.mesh.faces.len();
                scene.active_mut().unwrap().mesh = output.into();
                if rebuilt {
                    scene.selection.faces.clear();
                }
            }
            Tool::Inset | Tool::Shift => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh")?;
                // World distances on the item's own units (uniform scale
                // assumed, as for the vertex tools).
                let scale = crate::scene::uniform_scale(scene.world_matrix(item.id)?);
                let output = if live.tool == Tool::Inset {
                    crate::inset_geometry(&item.mesh, &selected, p.inset / scale)?
                } else {
                    crate::shift_geometry(&item.mesh, &selected, p.shift / scale)?
                };
                scene.active_mut().unwrap().mesh = output.mesh.into();
                scene.selection.faces = output.selected;
            }
            Tool::PolyHaul => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select a mesh for Poly Haul")?;
                let local = scene.world_matrix(item.id)?.inverse();
                let amount = crate::HaulAmount {
                    offset: p.poly_haul_offset,
                    vector: local.transform_vector3(p.poly_haul_amount.vector),
                    ..p.poly_haul_amount
                };
                let untouched = p.poly_haul_mode.uses_selection() && selected.is_empty()
                    || !p.poly_haul_mode.uses_selection()
                        && p.poly_haul_target == crate::HaulTarget::None;
                // Nothing to act on yet (no selection, or no drag started on
                // an element): the base scene stays.
                if !untouched {
                    let mut output = crate::poly_haul_mode_geometry(
                        &item.mesh,
                        &selected,
                        p.poly_haul_mode,
                        p.poly_haul_target,
                        amount,
                    )?;
                    // SYM3-10/12: a picked edge, point or polygon has its
                    // mirror partner hauled too, with the mirrored drag. The
                    // selection modes act on a symmetric selection already.
                    let symmetry = self.active_symmetry();
                    if !p.poly_haul_mode.uses_selection()
                        && let Some(target) = crate::symmetry_sides::mirrored_haul_target(
                            &item.mesh,
                            &output.mesh,
                            p.poly_haul_target,
                            symmetry,
                        )
                    {
                        let mirrored = crate::HaulAmount {
                            vector: symmetry.mirror_vector(amount.vector),
                            ..amount
                        };
                        let faces = output.faces.clone().unwrap_or_else(|| selected.clone());
                        output = crate::poly_haul_mode_geometry(
                            &output.mesh,
                            &faces,
                            p.poly_haul_mode,
                            target,
                            mirrored,
                        )?;
                    }
                    scene.active_mut().unwrap().mesh = output.mesh.into();
                    if let Some(faces) = output.faces {
                        scene.selection.faces = faces;
                    }
                }
            }
            Tool::SketchExtrude => {
                if !p.sketch_precision.is_finite() || !(1. ..=1000.).contains(&p.sketch_precision) {
                    return Err("Sketch Extrude Precision must be 1–1000 logical pixels".into());
                }
                if !p.sketch_path.is_empty() {
                    // SYM3-12: with Symmetry, the other side's polygons
                    // follow the mirrored path.
                    let symmetry = self.active_symmetry();
                    crate::symmetry_sides::two_pass(&mut scene, symmetry, |scene, mirrored| {
                        let selected = scene.selection.faces.clone();
                        let item = scene.active().ok_or("Select a mesh for Sketch Extrude")?;
                        // The drawn path is in world space; the mesh is local.
                        let local = scene.world_matrix(item.id)?.inverse();
                        let path: Vec<DVec3> = p
                            .sketch_path
                            .iter()
                            .map(|&q| local.transform_point3(q))
                            .map(|q| if mirrored { symmetry.mirror(q) } else { q })
                            .collect();
                        let (mesh, caps) =
                            crate::sketch_extrude_geometry(&item.mesh, &selected, &path)?;
                        scene.active_mut().unwrap().mesh = mesh.into();
                        scene.selection.faces = caps;
                        Ok(())
                    })?;
                }
            }
            Tool::SmoothShift | Tool::Thicken => {
                let selected = scene.selection.faces.clone();
                let item = scene.active().ok_or("Select an editable visible mesh")?;
                if !scene.shows(item) || scene.selection.mode != Mode::Polygon {
                    return Err("Select a visible mesh in Polygon mode".into());
                }
                // Q-SS-04 (Jason, 2026-11-18): Scale follows the selected
                // action centre; with none, the selection's centre. Origin
                // is the item's own origin (as the command oracles); the
                // world points of Element and Screen are taken into the item.
                let to_local = scene.world_matrix(item.id)?.inverse();
                let symmetry = self.active_symmetry();
                let point = |at: DVec3| crate::SmoothShiftPivot::Point {
                    at,
                    mirror: symmetry.axis.is_some().then_some(symmetry),
                };
                // ACS-20 (Jason, 2026-11-18): through the pivot service like
                // a transform tool: a placed centre (ACS-07) and, with
                // Symmetry, the service's primary pivot, the other side
                // scaling about its mirror image (ACS-04) and seam points
                // about its foot on the plane (ACS-05).
                use crate::CenterMode as C;
                let mirrored = symmetry.axis.is_some();
                let pivot = match (live.placed_centre, self.action_context.center) {
                    (Some(at), _) => point(to_local.transform_point3(at)),
                    (None, C::Local) => crate::SmoothShiftPivot::Local,
                    (None, C::None | C::Selection | C::SelectionAutoAxis | C::Automatic)
                        if mirrored =>
                    {
                        crate::SmoothShiftPivot::HalfBox(symmetry)
                    }
                    (None, C::SelectionBorder) if mirrored => {
                        crate::SmoothShiftPivot::HalfBorder(symmetry)
                    }
                    (None, C::None | C::Selection | C::SelectionAutoAxis | C::Automatic) => {
                        crate::SmoothShiftPivot::SelectionBox
                    }
                    (None, C::SelectionBorder) => crate::SmoothShiftPivot::SelectionBorder,
                    (None, C::Origin) => point(DVec3::ZERO),
                    (None, C::Element(at) | C::Screen(at)) => {
                        point(to_local.transform_point3(at.point()))
                    }
                    (None, C::Parent | C::Pivot) => point(
                        to_local
                            .transform_point3(live.action_frame.map_or(DVec3::ZERO, |f| f.center)),
                    ),
                };
                let result = crate::smooth_shift_geometry_about(
                    &item.mesh,
                    &selected,
                    p.smooth_shift,
                    None,
                    pivot,
                )?;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                scene.selection.faces = result.selected;
                scene.selection.vertices.clear();
                scene.selection.edges.clear();
            }
            Tool::Extrude => {
                let item = scene.active().ok_or("Select a mesh")?;
                let source_ids = live
                    .identity_base
                    .as_ref()
                    .and_then(|ids| ids.meshes.get(&item.id))
                    .ok_or("Missing extrusion identity basis")?;
                source_ids.validate_shared(item.mesh.shared())?;
                // Stable ordered targets are resolved once into this private
                // kernel view; neither a coordinate nor a stale dense offset
                // supplies correspondence after the edit.
                // PE-O-03: with nothing selected every polygon is extruded.
                let faces: Vec<u32> = if scene.selection.faces.is_empty() {
                    (0..item.mesh.faces.len() as u32).collect()
                } else {
                    scene.selection.faces.clone()
                };
                let targets = faces
                    .iter()
                    .map(|&f| {
                        source_ids
                            .faces
                            .get(f as usize)
                            .copied()
                            .map(|face| crate::FaceHandle {
                                scope: source_ids.scope,
                                face,
                            })
                            .ok_or_else(|| String::from("Invalid source face target"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let selected = targets
                    .into_iter()
                    .map(|h| source_ids.face_index(h))
                    .collect::<Result<Vec<_>>>()?;
                let world = scene.world_matrix(item.id)?;
                if !item.mesh.attributes.polygon_sets.is_empty()
                    && (world != glam::DMat4::IDENTITY || self.action_context != Default::default())
                {
                    return Err(
                        "I1-S named sets require identity world transform and neutral context"
                            .into(),
                    );
                }
                if !world.is_finite() || world.determinant().abs() < 1e-15 {
                    return Err("Extrude requires an invertible world transform".into());
                }
                if !apply && p.extent == DVec3::ZERO {
                    self.extrude_moved_caps = scene.selection.faces.clone();
                    self.scene = scene;
                    if let Some(ids) = &live.identity_base {
                        self.identities.current = ids.clone();
                        self.identities.observe(&self.scene)?;
                    }
                    let l = self.live.as_mut().unwrap();
                    l.identity_preview = None;
                    l.params = p;
                    l.applied = true;
                    l.preview_error = None;
                    return Ok(());
                }
                let mut source = item.mesh.clone();
                // Facing is evaluated in world coordinates, including mirrored
                // transforms; stored attributes remain authored local payloads.
                for v in &mut source.vertices {
                    *v = world.transform_point3(*v);
                }
                let map = p
                    .extrude_uv_map
                    .as_deref()
                    .filter(|n| source.uv_maps.contains_key(*n));
                let mut result = crate::extrude_geometry(
                    &source,
                    &selected,
                    p.extent,
                    p.extrude_sides,
                    p.extrude_mode,
                    p.extrude_uvs,
                    map,
                )?;
                let inverse = world.inverse();
                for (index, v) in result.mesh.vertices.iter_mut().enumerate() {
                    // Preserve untouched local coordinates exactly; a world/local
                    // roundtrip must not introduce drift in unselected geometry.
                    if index < source.vertices.len() && *v == source.vertices[index] {
                        *v = item.mesh.vertices[index];
                    } else {
                        *v = inverse.transform_point3(*v);
                    }
                }
                // SYM3-12: the extrude offset is mirrored for the caps on the
                // partner side (a normal-only extrude is symmetric anyway).
                let symmetry = self.active_symmetry();
                if symmetry.axis.is_some() && p.extent != DVec3::ZERO {
                    let primary = self.symmetry_primary(&scene);
                    let v = inverse.transform_vector3(p.extent);
                    let moved: Vec<u32> = result
                        .moved_caps
                        .iter()
                        .filter_map(|&f| result.mesh.faces.get(f as usize))
                        .flat_map(|f| f.vertices.iter().copied())
                        .collect();
                    let tolerance = crate::symmetry::match_tolerance(&item.mesh);
                    symmetry.mirror_offset(
                        &mut result.mesh.vertices,
                        &moved,
                        v,
                        primary,
                        tolerance,
                    );
                }
                let topology_changed = result.mesh != *item.mesh;
                let source_ids = live
                    .identity_base
                    .as_ref()
                    .and_then(|ids| ids.meshes.get(&item.id))
                    .ok_or("Missing extrusion stage identity basis")?;
                let ids = if let Some((old_plan, ids)) = &live.identity_preview
                    && old_plan == &result.correspondence
                {
                    ids.validate(&result.mesh)?;
                    ids.clone()
                } else {
                    self.identities.with_ids(|next| {
                        source_ids.transported(
                            &item.mesh,
                            &result.mesh,
                            &result.correspondence,
                            next,
                        )
                    })?
                };
                self.identities
                    .current
                    .meshes
                    .insert(item.id, ids.clone().into());
                self.live.as_mut().unwrap().identity_preview =
                    Some((result.correspondence.clone(), ids));
                scene.active_mut().unwrap().mesh = result.mesh.into();
                // While the tool is live the moving caps stay selected (a
                // continuation extrudes them again); Apply, and Drop
                // (`select_extrude_caps`), select both caps (PE-M-03).
                scene.selection.faces = if apply {
                    result.caps
                } else {
                    result.moved_caps.clone()
                };
                if topology_changed {
                    scene.selection.vertices.clear();
                    scene.selection.edges.clear();
                }
                scene.validate()?;
                self.extrude_moved_caps = result.moved_caps;
                self.identities.observe(&scene)?;
            }
            Tool::TwistBevel => twist_note = self.preview_twist_bevel(&mut scene, live, &p)?,
            Tool::Bevel => {
                // PB-O-01: nothing selected bevels every polygon and leaves
                // nothing selected. bevel-activation-2026-11-05 BA11 says the
                // opposite; kept as measured in polygon-bevel-2026-11-02 until
                // the spec side settles it (SPECIFICATION_QUESTIONS BA-Q01).
                let everything = scene.selection.faces.is_empty();
                let selected = if everything {
                    (0..scene.active().ok_or("Select an item")?.mesh.faces.len() as u32).collect()
                } else {
                    scene.selection.faces.clone()
                };
                let item = scene.active().ok_or("Select an item")?;
                let weights = self.falloff_weights(&scene, item.id)?;
                let bevel = if apply {
                    crate::bevel_geometry_applied
                } else {
                    crate::bevel_geometry_with_profile
                };
                let result = bevel(
                    &item.mesh,
                    &selected,
                    p.shift,
                    p.inset,
                    p.group_polygons,
                    &p.bevel,
                    weights.as_deref(),
                    self.bevel_profile,
                )
                .map_err(|e| format!("Bevel Shift/Inset: {e}"))?;
                let topology_changed = result.mesh != *item.mesh;
                scene.active_mut().unwrap().mesh = result.mesh.into();
                scene.selection.faces = if everything {
                    Vec::new()
                } else {
                    result.selected
                };
                if topology_changed {
                    scene.selection.vertices.clear();
                    scene.selection.edges.clear();
                }
            }
            Tool::Tweak => {
                // The grabbed element's points move by the world offset
                // `delta`, nearby points by their falloff weight;
                // selection, topology and IDs stay as they were (TWK-22).
                if !p.delta.is_finite() {
                    return Err("Tweak offset must be finite".into());
                }
                if let Some(grab) = p.tweak {
                    let item = scene.active().ok_or("Select a mesh")?;
                    if item.id != grab.item {
                        return Err("Tweak edits the active mesh only".into());
                    }
                    let world = live.base.world_matrix(grab.item)?;
                    let offset = world.inverse().transform_vector3(p.delta);
                    // TWK-12/15/16 and TWK-20 (provisional Symmetry): which
                    // points move and by how much comes from the press-time
                    // mesh, worked out once per drag.
                    let symmetry = self.active_symmetry();
                    let base_mesh = &live.base.active().ok_or("Select a mesh")?.mesh;
                    // In a UV view (UVT-131) Range may be in screen pixels
                    // (UVT-Q11) and connectivity read from the 3D mesh
                    // (UVT-Q12); see uv_pipe.rs.
                    let uv_space = p.uv.as_ref();
                    let mut options = p.tweak_options;
                    if let Some(space) = uv_space
                        && crate::uv_pipe::RANGE_UNITS == crate::uv_pipe::RangeUnits::Pixels
                        && space.pixels_per_uv > 0.
                    {
                        options.range /= space.pixels_per_uv;
                    }
                    let uv_proxy = uv_space.and_then(|s| s.proxy.clone()).filter(|_| {
                        crate::uv_pipe::CONNECTIVITY == crate::uv_pipe::UvConnectivity::Mesh
                    });
                    let pieces =
                        |seeds: &[u32]| uv_proxy.as_ref().expect("checked").connected_in_3d(seeds);
                    // The plan rides on the live parameters from frame to frame.
                    let plan = match p
                        .tweak_plan
                        .take()
                        .or_else(|| live.params.tweak_plan.clone())
                    {
                        Some(plan)
                            if plan.grab == grab
                                && plan.options == options
                                && plan.symmetry == symmetry =>
                        {
                            plan
                        }
                        _ => crate::tweak::tweak_plan_with(
                            base_mesh,
                            world,
                            grab,
                            options,
                            symmetry,
                            uv_proxy
                                .is_some()
                                .then_some(&pieces as crate::tweak::PieceFinder<'_>),
                        )?,
                    };
                    let item = scene.active_mut().ok_or("Select a mesh")?;
                    crate::tweak_apply(&mut item.mesh, base_mesh, &plan, offset)?;
                    p.tweak_plan = Some(plan);
                } else if p.delta != DVec3::ZERO {
                    return Err("Tweak: press on a vertex, edge or polygon and drag".into());
                }
            }
            Tool::Transform | Tool::Move | Tool::Rotate | Tool::Scale => {
                if !p.delta.is_finite()
                    || !p.rotation_delta.is_finite()
                    || !p.scale_delta.is_finite()
                {
                    return Err("Transform must be finite".into());
                }
                let factors = if live.tool == Tool::Transform {
                    Some(p.scale_delta)
                } else {
                    (live.tool == Tool::Scale).then_some(p.delta)
                };
                // IT-04: with Negative Scale on, an item scale may flip sign
                // (never zero). Components keep the positive-only rule.
                let negative_ok = p.negative_scale && scene.selection.mode == Mode::Item;
                if let Some(f) = factors
                    && (f.abs().min_element() <= 1e-8 || (!negative_ok && f.min_element() <= 1e-8))
                {
                    return Err(if negative_ok {
                        "Scale can't be zero".into()
                    } else {
                        "Scale must be positive; turn on Negative Scale for items".into()
                    });
                }
                if scene.selection.mode == Mode::Item
                    && let Some(targets) = live.base.item_transform_targets()
                {
                    // Several selected items move together (multi_transform.rs).
                    crate::multi_transform::transform_items(
                        &mut scene, &live.base, &targets, live.tool, &p,
                    )?;
                    // Planar spec: a Texture Locator's Size stops at 1 mm.
                    scene.clamp_locator_sizes();
                } else if scene.selection.mode == Mode::Item {
                    let item = scene
                        .active_mut()
                        .ok_or("Select an item; empty selection does not target all items")?;
                    // CP-A02: a locked item refuses item transforms.
                    if item.is_locked() {
                        return Err(format!("{} is locked (Assembly > Lock)", item.name));
                    }
                    // ENVIRONMENT-PLACEMENT §2: an environment has no
                    // transform; its sky turns on its layers' locators.
                    if item.environment.is_some() {
                        return Err(crate::environment::NO_TRANSFORM.into());
                    }
                    match live.tool {
                        Tool::Transform => {
                            item.transform.translation += p.delta;
                            item.transform.rotation += p.rotation_delta.map(f64::to_radians);
                            item.transform.scale *= p.scale_delta;
                        }
                        Tool::Move => item.transform.translation += p.delta,
                        Tool::Rotate => item.transform.rotation += p.delta.map(f64::to_radians),
                        Tool::Scale => item.transform.scale *= p.delta,
                        _ => {}
                    }
                    // Planar spec: a Texture Locator's Size stops at 1 mm.
                    if item.texture_locator().is_some() {
                        item.transform.scale =
                            crate::texture_locator::clamp_size(item.transform.scale);
                    }
                } else {
                    // Every active mesh with a selection moves about the one
                    // shared frame (multi_transform.rs); just the primary
                    // when it is the only one.
                    let targets: Vec<(u64, std::collections::BTreeSet<u32>)> =
                        match live.base.component_transform_meshes() {
                            Some(ids) => ids
                                .into_iter()
                                .map(|id| Ok((id, live.base.mesh_transform_vertices(id)?)))
                                .collect::<Result<_>>()?,
                            None => vec![(
                                scene.active().ok_or("Select a mesh")?.id,
                                crate::transform_vertices(&live.base)?,
                            )],
                        };
                    for (item_id, mut vertices) in targets {
                        // FO-47/48/45: Soft Selection, Coplanar and Element
                        // falloffs also move the points they reach outside
                        // the selection.
                        if let Some(f) = self.active_falloff().filter(|f| f.reaches()) {
                            vertices.extend(f.reach(&live.base, item_id)?);
                        }
                        // The action frame is in world space: move each point there,
                        // apply the transform, and map the result back to local.
                        let world = live.base.world_matrix(item_id)?;
                        if world.determinant() == 0. || !world.determinant().is_finite() {
                            return Err("Component tools need an invertible item transform".into());
                        }
                        let item = scene.item_mut(item_id).ok_or("Select a mesh")?;
                        let frame = live.action_frame.ok_or("Missing action frame")?;
                        let angles = if live.tool == Tool::Transform {
                            p.rotation_delta
                        } else {
                            p.delta
                        };
                        let rotation = glam::DQuat::from_euler(
                            glam::EulerRot::XYZ,
                            angles.x.to_radians(),
                            angles.y.to_radians(),
                            angles.z.to_radians(),
                        );
                        // The pivot service's mirror rules (ACS-03..06): the
                        // primary side is the side the pivot is on; selected
                        // points there take the values as entered (typed or
                        // dragged alike), the other side's are mirrored,
                        // edited and mirrored back, and seam points go back
                        // onto the plane. Several meshes share one frame and
                        // each mirrors in its own space (MAM-Q09).
                        let symmetry = self.active_symmetry();
                        let map = crate::pivot_service::MirrorMap::new(
                            &live.base,
                            item_id,
                            symmetry,
                            frame.center,
                        )?;
                        // FAL-M-02 / FAL-B2-02/03: with a falloff each point moves to
                        // p + w·(T(p) − p), its weight's share of the full
                        // transform. With Symmetry the weight is read at the
                        // point's positive-side position (FAL-B2-04).
                        let falloff = self.active_falloff();
                        // VMF-03: a Vertex Map falloff reads the point's own
                        // value (copied out, the mesh is edited below).
                        // The selection-driven falloffs read the drag's
                        // starting mesh and selection (once per drag).
                        let prepared = match falloff.as_ref() {
                            Some(f) => f.prepare(&live.base, item_id)?,
                            None => Default::default(),
                        };
                        // TW-Q1: Twist and Swirl turn each point by weight ×
                        // Angle instead of blending positions (deform.rs).
                        let angle_weighted = live.tool == Tool::Rotate
                            && self.preset.as_ref().is_some_and(|p| p.angle_by_weight);
                        // Local: each connected piece about its own centre (F7).
                        let pieces =
                            (self.action_context.center == crate::CenterMode::Local).then(|| {
                                let selection = if live.base.selection.item == Some(item_id) {
                                    live.base.selection.clone()
                                } else {
                                    live.base.selection.sole(item_id)
                                };
                                crate::local_piece_frames(&item.mesh, &vertices, &selection)
                            });
                        for i in vertices {
                            let stored = &mut item.mesh.vertices[i as usize];
                            let role = map.role(*stored);
                            // Local: a point from the other side is edited in
                            // its island's mirror image (it is mirrored into
                            // the partner island first, ACS-04, ACS-10/11).
                            let frame = match pieces.as_ref().and_then(|p| p.get(&i)) {
                                Some(&piece) if role == crate::pivot_service::Role::Other => {
                                    piece.mirrored(symmetry).frame(world, frame.basis)
                                }
                                Some(&piece) => piece.frame(world, frame.basis),
                                None => frame,
                            };
                            // ACS-13: the weight is read where the point's
                            // primary-side image is.
                            let sample = map.falloff_sample(*stored);
                            let w = falloff.as_ref().map_or(1., |f| {
                                f.weight_at_local(
                                    world.transform_point3(sample),
                                    sample,
                                    i,
                                    prepared.inputs(),
                                )
                            });
                            let edit = |start: DVec3| -> DVec3 {
                                let v = start;
                                let moved = match live.tool {
                                    Tool::Transform => {
                                        frame.center
                                            + frame.basis
                                                * (rotation
                                                    * ((frame.basis.transpose()
                                                        * (v - frame.center))
                                                        * p.scale_delta)
                                                    + p.delta)
                                    }
                                    Tool::Move => v + frame.basis * p.delta,
                                    Tool::Rotate if angle_weighted => {
                                        let turned = angles * w;
                                        let rotation = glam::DQuat::from_euler(
                                            glam::EulerRot::XYZ,
                                            turned.x.to_radians(),
                                            turned.y.to_radians(),
                                            turned.z.to_radians(),
                                        );
                                        frame.center
                                            + frame.basis
                                                * (rotation
                                                    * (frame.basis.transpose()
                                                        * (v - frame.center)))
                                    }
                                    Tool::Rotate => {
                                        frame.center
                                            + frame.basis
                                                * (rotation
                                                    * (frame.basis.transpose()
                                                        * (v - frame.center)))
                                    }
                                    Tool::Scale => {
                                        frame.center
                                            + frame.basis
                                                * ((frame.basis.transpose() * (v - frame.center))
                                                    * p.delta)
                                    }
                                    _ => v,
                                };
                                if angle_weighted {
                                    moved
                                } else {
                                    start + (moved - start) * w
                                }
                            };
                            *stored = map.apply(*stored, edit);
                        }
                    }
                }
            }
        }
        // A drag that only moves points skips re-checking every polygon.
        if let Some(live) = &self.live {
            for item in &scene.items {
                if let Some(before) = live.base.item(item.id) {
                    item.mesh.shared().prove_points_moved(before.mesh.shared());
                }
            }
        }
        scene.validate()?;
        // Item-mode copies take new ids (DS-12).
        self.next_item_id = self.next_item_id.max(scene.next_id);
        self.scene = scene;
        if twist_note.is_some() {
            self.symmetry_note = twist_note;
        }
        let l = self.live.as_mut().ok_or("No tool")?;
        l.params = p;
        l.preview_error = None;
        l.applied = true;
        Ok(())
    }

    /// Twist Bevel on the active mesh (twist-bevel-2026-11-15): the shared
    /// pipe's falloff (one weight per tube, read at its base centre),
    /// symmetry (mirror-image tubes) and the action axis (Rotation Axis =
    /// Action Axis) go to `twist_bevel_geometry`. Returns the symmetry
    /// note, if any.
    fn preview_twist_bevel(
        &self,
        scene: &mut Scene,
        live: &Live,
        p: &Parameters,
    ) -> Result<Option<String>> {
        // PB-O-01 as Bevel: nothing selected works on every polygon and
        // leaves nothing selected.
        let everything = scene.selection.faces.is_empty();
        let item = scene.active().ok_or("Select an item")?;
        let selected: Vec<u32> = if everything {
            (0..item.mesh.faces.len() as u32).collect()
        } else {
            scene.selection.faces.clone()
        };
        let world = scene.world_matrix(item.id)?;
        // Distances are entered in world units on the item's own scale
        // (uniform scale assumed, as for Inset and Shift).
        let scale = crate::scene::uniform_scale(world);
        let options = crate::TwistBevelOptions {
            shift: p.twist_bevel.shift / scale,
            inset: p.twist_bevel.inset / scale,
            ..p.twist_bevel
        };
        // FAL (spec section 5, TB-Q05): a position falloff is read at each
        // tube's base centre; one that reads points (a weight map, the
        // selection) gives the mean of the tube's point weights.
        let field = self.active_falloff();
        let point_weights = match &field {
            Some(f) if f.reads_map() || f.reads_selection() => Some(f.weights(scene, item.id)?),
            _ => None,
        };
        let weight = |centre: DVec3, points: &[u32]| -> f64 {
            let Some(field) = &field else { return 1. };
            match &point_weights {
                Some(w) => {
                    points.iter().map(|&v| w[v as usize]).sum::<f64>() / points.len().max(1) as f64
                }
                None => field.weight_at_local(
                    world.transform_point3(centre),
                    centre,
                    0,
                    Default::default(),
                ),
            }
        };
        let symmetry = self.active_symmetry();
        let primary = self.symmetry_primary(scene);
        let action_axis = live
            .action_frame
            .map(|f| world.inverse().transform_vector3(f.basis.y_axis))
            .unwrap_or(DVec3::Y);
        let pipe = crate::TwistBevelPipe {
            weight: field
                .is_some()
                .then_some(&weight as &dyn Fn(DVec3, &[u32]) -> f64),
            symmetry: symmetry.axis.map(|_| (symmetry, primary)),
            action_axis,
        };
        let result = crate::twist_bevel_geometry(&item.mesh, &selected, &options, &pipe)?;
        let topology_changed = result.mesh != *item.mesh;
        scene.active_mut().unwrap().mesh = result.mesh.into();
        scene.selection.faces = if everything {
            Vec::new()
        } else {
            result.selected
        };
        if topology_changed {
            scene.selection.vertices.clear();
            scene.selection.edges.clear();
        }
        Ok(result.note)
    }
}

/// What `dispatch_inner` does after a command handler. Most handlers end by
/// advancing `revision`; a few finish early and must leave it unchanged.
pub(crate) enum AfterCommand {
    BumpRevision,
    Done,
}

/// The polygons an applied Extrude selects on Drop: the moving caps in
/// `faces` plus each one's kept bottom cap (`select_extrude_caps`). `None`
/// when the session has no applied stage.
pub(crate) fn extrude_caps_on_drop(l: &Live, faces: &[u32]) -> Option<Vec<u32>> {
    let (plan, _) = l.identity_preview.as_ref().filter(|_| l.applied)?;
    let source_faces = plan.source_layout.len();
    let mut out = faces.to_vec();
    let bottoms: Vec<u32> = faces
        .iter()
        .filter(|&&f| f as usize >= source_faces)
        .filter_map(|&f| plan.face_sources.get(f as usize).map(|&s| s as u32))
        .filter(|s| !faces.contains(s))
        .collect();
    out.extend(bottoms);
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Index of a primitive tool in per-primitive settings (Cube, Sphere, Cylinder).
fn primitive_slot(tool: Tool) -> usize {
    match tool {
        Tool::Sphere => 1,
        Tool::Cylinder => 2,
        _ => 0,
    }
}

/// Sphere or Cylinder geometry from the live settings (`p`'s Make UVs,
/// axis, Cylinder caps and Sphere mode); `radial` gives size and resolution.
/// One flat `sides`-gon when exactly one radius is 0, facing that axis.
fn flat_disc(position: DVec3, radial: crate::RadialParameters) -> Result<Option<crate::Mesh>> {
    let zero: Vec<usize> = (0..3).filter(|&a| radial.radius[a] == 0.).collect();
    let [normal] = zero[..] else {
        return Ok(None);
    };
    crate::flat_ngon(
        position,
        normal,
        radial.radius[(normal + 1) % 3],
        radial.radius[(normal + 2) % 3],
        radial.sides,
    )
    .map(Some)
}

fn radial_mesh(
    tool: Tool,
    position: DVec3,
    radial: crate::RadialParameters,
    p: &Parameters,
) -> Result<crate::Mesh> {
    // Drag creation's flat base (spec drag-creation-2026-11-06 rule 1): with
    // one radius 0 the shape is one Sides-gon disc in the other two axes.
    if let Some(disc) = flat_disc(position, radial)? {
        return Ok(disc);
    }
    if tool != Tool::Sphere {
        crate::cylinder_geometry_with(position, radial, p.cylinder_caps, p.radial_axis, p.make_uvs)
    } else if let Some(level) = p.sphere_level {
        crate::tessellated_sphere(position, radial.radius, level, p.radial_axis, p.make_uvs)
    } else {
        crate::globe_geometry_with(position, radial, p.radial_axis, p.make_uvs)
    }
}
