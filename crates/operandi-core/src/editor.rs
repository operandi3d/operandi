use crate::{Mode, Result, Scene};
use glam::DVec3;

mod activate;
mod command;
mod context;
mod dispatch;
mod finish;
mod history;
mod live_tool;
mod parameters;
mod preview;
mod tool;

pub use command::Command;
pub use parameters::Parameters;
pub use tool::Tool;

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
    /// The pivot service's answer for this tool on `base`, kept so a drag
    /// resolves it once (`Editor::live_pivots`). Derived; not compared.
    pub pivots: crate::pivot_service::PivotCache,
}
#[derive(Clone, PartialEq)]
pub struct Checkpoint {
    pub identity_before: Option<crate::IdentityScene>,
    pub identity_after: Option<crate::IdentityScene>,
    pub label: String,
    pub before: Scene,
    pub after: Scene,
    /// The gesture this step belongs to (`History::undo_fold`).
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
    /// Undo and redo steps, their marks and limits (undo_history.rs).
    pub history: crate::History,
    /// The tools' last-used values, one per tool (preset_store.rs).
    pub presets: crate::ToolPresets,
    /// The running tool session: arming, tweaks, the tool Redo reopens and
    /// the command and gesture running now (tool_session.rs).
    pub session: crate::ToolSession,
    /// Meshes a tool or command has just run on as one of several active
    /// meshes (multi_tools.rs): their components were written by that
    /// tool for the new geometry, so they survive the topology check in
    /// `prune_component_selections`. Kept while a live tool is open;
    /// cleared after every other command.
    pub multi_fresh: std::collections::BTreeSet<u64>,
    /// Select > Polygon Tag Type: what tag selection modes select by.
    pub polygon_tag_kind: crate::PolygonTagKind,
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
    /// Interactive Pen options, kept between Pen sessions.
    pub pen_options: crate::PenOptions,
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
    /// Last Linear and Radial settings, offered on reactivation (FAL-M-03).
    pub linear_falloff_preset: crate::LinearFalloff,
    pub radial_falloff_preset: crate::RadialFalloff,
    /// CS-01 (click_start.rs): honour the tools' Auto-Activate flags, so a
    /// tool flagged off only arms when picked. The app turns it on; off,
    /// every tool starts at once as before (scripts, staged one-shots).
    pub auto_activate_flags: bool,
    /// 3D painting's saved-image record and last-made image and layer.
    #[cfg(feature = "painting")]
    pub paint: crate::paint::PaintSession,
    pub surface_states: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, crate::SmoothShiftParameters>,
    >,
    pub surface_state: std::collections::BTreeMap<String, String>,

    pub spike_strength: f64,
    pub scene: Scene,
    pub action_context: crate::ActionContext,
    /// Shared Symmetry setting (SYM-ST-01); tool context, not scene data.
    pub symmetry: crate::Symmetry,
    /// The document's key time, channel/key selections and playback
    /// (animation.rs; per scene tab).
    pub anim: crate::animation::AnimSession,
    /// RG-11/RG-30 Compensation: rig edits keep items where they are in
    /// world space (off by default; the app keeps it in its preferences).
    pub rig_compensation: bool,
    pub revision: u64,
    pub next_item_id: u64,
    /// Boxed to keep `Editor` small: the app state is near the stack limit.
    pub selection_start: Option<Box<Scene>>,
    pub extrude_moved_caps: Vec<u32>,
    /// Extrude on several meshes: each mesh's moving caps and the caps it
    /// selects on Drop (multi_tools.rs). Boxed like `selection_start`.
    pub multi_extrude: Option<Box<crate::multi_tools::MultiExtrude>>,
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
    /// Memory free for the app now, in bytes; Subdivide refuses a result
    /// that needs more (`Editor::subdivide_estimate`). The app sets it;
    /// without it (scripts, tests) nothing is checked.
    pub free_memory: FreeMemory,
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
    /// A remesh the app worked out on a worker thread, used by the next
    /// remesh command when it matches (`remesh::RemeshPrepared`).
    pub remesh_prepared: Option<Box<crate::remesh::RemeshPrepared>>,
    /// What the last successful Boolean wants on the status line.
    pub boolean_note: Option<String>,
    /// Mesh Stack results (mesh_stack.rs): a cache shared by clones, never part
    /// of the scene (SAFE-03).
    pub mesh_stack: crate::mesh_stack::Runtime,
    /// The scene's evaluation graph (eval_state.rs): a cache, never part
    /// of the scene or undo (AR-04, AR-15).
    pub eval: crate::eval_state::EvalState,
    /// The mesh in Edit Base Mesh (MO-27): direct tools edit its Base Mesh.
    pub base_edit: Option<u64>,
    /// The selected row of the Mesh Operations list.
    pub meshop_row: Option<crate::mesh_stack_editor::OpRow>,
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

/// Sphere or Cylinder geometry from the live settings, through the shape's
/// one generator (primitive_shapes.rs, shared with its MeshOp); `radial`
/// gives size and resolution.
fn radial_mesh(
    tool: Tool,
    position: DVec3,
    radial: crate::RadialParameters,
    p: &Parameters,
    weight: crate::primitive_shapes::Weight,
) -> Result<crate::Mesh> {
    if tool == Tool::Sphere {
        p.sphere_shape(position, radial).mesh_weighted(weight)
    } else {
        p.cylinder_shape(position, radial).mesh_weighted(weight)
    }
}
