//! Portable modeling state. No window, graphics, or platform API dependencies.
pub mod action_center;
pub mod bake;
pub mod brush_engine;
#[cfg(feature = "painting")]
pub mod paint;
#[cfg(feature = "painting")]
pub mod paint_geometry_brush;
#[cfg(feature = "painting")]
pub mod paint_inks;
#[cfg(feature = "painting")]
pub mod paint_lasso;
#[cfg(feature = "painting")]
pub mod paint_sculpt;
#[cfg(feature = "painting")]
pub mod paint_starters;
#[cfg(feature = "painting")]
pub mod paint_stroke;
#[cfg(feature = "painting")]
pub mod paint_tips;
pub mod selection_inference;
pub mod uv_atlas;
pub mod uv_box;
pub mod uv_charts;
pub use action_center::*;
pub mod align_tools;
pub mod angle_snap;
pub mod pen;
pub use align_tools::{
    GroundAlign, GroundMode, QuickAlign, QuickAlignKind, SelectionAxisMode, Tack, TackOptions,
};
pub mod clipboard;
pub mod curve_boolean;
pub mod curve_commands;
pub mod curve_convert;
pub mod curve_edit;
pub mod curve_fit;
pub mod curve_ops;
pub mod curves;
pub mod dicer;
pub mod generator_ops;
pub mod icon_studio;
pub mod item_merge;
pub mod patch;
pub mod patch_live;
pub mod path_tools;
pub mod sketch;
pub use clipboard::{Clipboard, PastePlacement};
pub mod text;
pub use curve_edit::{AddGeometry, CurveEdit};
pub use curves::CurveKind;
pub use text::{TextJustification, TextLocation, TextParameters};
pub mod unit_insert;
pub use pen::*;
pub use unit_insert::UnitShape;
pub mod background_constraint;
pub mod camera_light;
pub mod camera_light_defaults;
pub mod camera_props;
mod corner_uv;
pub mod drop_action;
pub mod environment;
mod environment_channels;
pub mod item_common;
pub mod item_common_channels;
pub mod item_delete;
pub mod item_extras;
pub mod item_groups;
pub mod item_list;
pub mod item_target;
pub mod item_transform_ops;
pub mod locator;
pub mod photometry;
pub mod physical_sun;
pub use drop_action::DropAction;
pub mod command_record;
pub mod editor;
pub use command_record::{CommandClass, CommandRecord, RecordLog, Source, StepRecords};
mod history_settings;
pub mod units;
pub mod user_defaults;
pub use history_settings::*;
mod flip;
mod flip_it;
pub mod freeze;
pub mod fxhash;
pub mod interchange;
pub mod lxo;
pub mod mesh;
pub mod polygon;
pub mod render_settings;
pub mod shading;
pub mod subdivision;
pub use polygon::*;
pub mod blender_principled;
pub mod cycles_noise;
pub mod cycles_voronoi;
pub mod dispersion;
pub mod gradient;
pub mod graph_view;
pub mod light_path_expressions;
pub mod material;
pub mod material_values;
pub mod materialx;
pub mod node_graph;
pub mod node_material;
pub mod node_view;
pub mod node_view_board;
pub mod node_view_commands;
pub mod node_view_settings;
pub mod openpbr;
pub mod per_light_outputs;
pub mod planar_texture;
pub mod procedural;
pub mod process_occlusion;
pub mod render_outputs;
pub mod shader_eval;
pub mod shader_func;
pub mod shader_instances;
pub mod shader_layer_menu;
pub mod shader_library;
pub mod shader_tree;
pub mod shader_wgsl;
pub mod texture_locator;
pub mod transform_channel;
pub mod vertex_map_texture;
pub use material::{Material, MaterialKind};
pub use shader_tree::ShaderTreeEdit;
pub mod polygon_material;
pub use polygon_material::*;
mod flatten_ngons;
mod polygon_align;
pub use flatten_ngons::*;
pub mod polygon_part;
pub mod polygon_tags;
pub use polygon_tags::*;
pub mod colour_math;
pub mod contour;
pub mod diagonal_redirect;
pub mod edge_zip;
pub mod editor_color;
pub mod fix_black_dots;
pub mod merge_instance;
pub mod mesh_paint;
pub mod plane_item;
pub mod scaffold;
pub mod teapot;
pub use editor_color::EditorColor;
pub mod tag_selection;
pub use tag_selection::{TagRegion, TagSelection};
mod polygon_quadruple;
mod polygon_scale_offset;
mod polygon_spin;
mod polygon_split;
pub mod polygon_unify;
pub use polygon_part::*;
pub use polygon_unify::*;
mod bridge_commands;
pub mod channels;
pub mod command_args;
pub mod command_exec;
pub mod command_line;
pub mod command_names;
pub mod command_registry;
pub mod command_runtime;
mod set_position;
pub use set_position::*;
mod absolute_scaling;
pub use absolute_scaling::*;
pub mod polygon_subdivide;
pub use polygon_subdivide::{
    SubdivideBoundary, SubdivideMethod, SubdivideOptions, SubdivideOutcome, UnsubdivideOptions,
};
pub mod polygon_triple;
pub use polygon_triple::*;
pub mod mesh_cleanup;
pub use mesh_cleanup::*;
pub mod polygon_remove;
pub use polygon_remove::*;
pub mod polygon_merge;
pub use polygon_merge::*;
pub mod polygon_collapse;
pub use polygon_collapse::*;
pub mod edge_extend;
pub use edge_extend::{
    EXTEND_UV_MAP, EdgeExtendOptions, ExtendHandles, ExtendHaul, ExtendPlane, ExtendShape,
    ExtendUvs, edge_extend_at_zero, edge_extend_geometry, edge_extend_shaped,
};
pub mod edge_extrude;
pub use edge_extrude::{
    edge_extrude_geometry, edge_extrude_start_width, edge_extrude_with, vertex_extrude_start_width,
};
pub mod edge_slide;
pub use edge_slide::{
    SlideExtras, edge_slide_extra, edge_slide_geometry, edge_slide_weighted, edge_slide_with,
};
pub mod edge_crease;
pub use edge_crease::edge_crease_geometry;
pub mod edge_bevel;
mod edge_bevel_junctions;
pub use edge_bevel::{
    EdgeBevelMode, EdgeBevelOptions, EdgeBevelPreset, EdgeBevelResult, EdgeBevelSettings,
    EdgeBevelShape, EdgeEnd, edge_bevel_geometry, edge_bevel_with,
};
pub mod edge_commands;
pub mod edge_relax;
pub use edge_commands::{
    EdgeCommand, edge_bridge, edge_bridge_geometry, edge_command_geometry, edge_delete_geometry,
    edge_join_symmetric_geometry, edge_subdivide_geometry,
};
pub use edge_relax::edge_relax_geometry;
pub mod axis_slice;
pub mod detriangulate;
pub mod uv_edit;
pub mod uv_export;
pub mod uv_maps;
pub mod uv_ops;
pub mod uv_peel;
pub mod uv_pipe;
pub mod uv_project;
pub mod uv_relax;
pub mod uv_seams;
pub mod uv_symmetry;
pub mod uv_tools;
pub use detriangulate::DetriangulateOptions;
pub mod anim_camera_channels;
pub mod anim_commands;
pub mod anim_curve;
pub mod anim_edit;
pub mod anim_graph;
pub mod animation;
pub mod boolean;
pub mod click_select;
pub mod drill;
pub mod eval_build;
pub mod eval_graph;
pub mod eval_state;
pub mod evaluate;
pub mod kit_item;
pub mod meshop_selection;
pub mod meshop_types;
pub mod meshops;
mod meshops_builtin;
pub mod meshops_editor;
pub mod meshops_items;
pub mod meshops_tools;
pub mod rig;
pub mod rig_commands;
pub mod rig_eval;
pub mod scene_time;
pub mod time_tick;
pub use boolean::{BooleanOp, BooleanOptions, BooleanProblem, DriveMesh};
pub use drill::{DrillAxis, DrillKind, DrillOp, DrillOptions};
pub mod duplicate;
pub mod duplicate_copier;
mod duplicate_hierarchy;
pub mod duplicate_items;
pub mod duplicate_sweep;
pub mod duplicate_transform;
pub use duplicate::{
    Array, Circle, CloneLine, Copies, DuplicateParameters, Lathe, Mirror, MirrorMode, RadialArray,
    Sweep, UvsAround,
};
pub mod edge_insert;
pub mod path_slice;
pub use axis_slice::{AxisSlice, AxisSliceMode, axis_slice_bounds, axis_slice_planes};
pub mod profile;
pub use profile::{ProfilePick, ProfilePoint, ProfileShape};
pub mod slice;
pub(crate) mod slice_cut;
pub use edge_insert::{
    EdgeInsertOptions, add_loop, add_loop_ring, add_loop_with, add_point, add_point_with,
};
pub use path_slice::{
    CurveSlice, CutPath, PathCutResult, PathCutter, PenSlice, PrimitiveShape, PrimitiveSlice,
    ShapePlace, SliceAxis, SliceOperation, path_cut,
};
pub use slice::{
    EdgeSlice, LoopSlice, LoopSliceEdit, LoopSliceMode, OffsetSide, PlaneCut, PolygonSlice,
    SliceParameters, SlicePoint, SliceShow,
};
pub mod polygon_make;
pub mod vertex_commands;
pub use polygon_make::*;
pub use vertex_commands::{
    MakeType, MergeOptions, ReorderMode, VERTEX_BEVEL_MAX_ROUND_LEVEL, VertexCommand, WeldDrop,
    add_points, make_polygon_from_points, make_polygon_typed, sync_index_order,
    vertex_bevel_geometry, vertex_bevel_round, vertex_command_geometry, vertex_extrude_geometry,
    vertex_extrude_with, vertex_join_symmetric_geometry, vertex_merge_geometry, vertex_merge_with,
    vertex_slide_direction, vertex_slide_geometry, vertex_tear_geometry, weld_drag_geometry,
    weld_points,
};
pub mod numeric;
pub mod tweak;
pub use tweak::{
    TweakConnect, TweakElement, TweakGrab, TweakMirror, TweakMode, TweakOptions, TweakPlan,
    edge_loop_points, tweak_apply, tweak_mirror, tweak_mirror_points, tweak_plan, tweak_points,
    tweak_points_mirrored,
};
pub mod mesh_pack;
pub mod mesh_slot;
pub mod node_graph_reader;
pub mod packed_mesh;
pub use mesh_slot::{MeshSlot, MeshView};
pub mod mesh_instance;
pub use packed_mesh::{MeshRead, PackedMesh};
mod document_state;
pub mod persistence;
pub mod preset;
pub mod primitive;
pub mod transform_repeat;
mod undo_memory;
pub use primitive::*;
pub mod primitive_uv;
pub mod project;
pub mod project_paths;
pub use primitive_uv::*;
pub use project::Project;
pub mod radial;
pub use radial::*;
pub mod active_task;
pub mod backdrop;
pub mod last_mesh;
pub mod multi_active;
pub mod multi_tools;
pub mod multi_transform;
pub use multi_active::ComponentSelection;
pub mod scene;
pub use active_task::ActiveTask;
pub mod selection;
pub mod selection_sets;
pub use selection_sets::{SetKind, SetRef, UseSetMode};
mod selection_continuation;
mod selection_neighbour_loop;
mod selection_symmetry;
pub use editor::*;
pub use mesh::*;
pub use scene::*;
pub use selection::*;
pub mod bend;
pub mod click_start;
pub mod deform;
pub mod deform_finish;
pub mod falloff;
pub mod meshops_deform;
pub mod select_tools;
pub mod topology_cleanup;
pub mod topology_pen;
pub use bend::BendOptions;
pub use deform::{DeformPreset, PushNormals, SmoothOptions};
pub use deform_finish::{
    JitterOptions, LinearAlignOptions, LinearMode, QuantizeOptions, RadialAlignOptions, RadialMode,
    RelaxMeshOptions,
};
pub use topology_pen::{PenEdit, PenMode, TopologyPenOptions};
pub mod haul;
pub mod pivot_service;
pub mod tool_preset;
mod tool_reset;
pub use tool_preset::{PresetRow, PresetSession};
pub mod scene_index;
pub mod shared;
pub mod topology;
pub mod topology_cache;
pub use falloff::{
    ActiveFalloff, CylinderFalloff, Falloff, FalloffShape, FalloffSymmetric, LinearFalloff,
    MixMode, RadialFalloff, SelectionFalloff, VertexMapFalloff, VertexMapMode,
};
pub mod falloff_path;
pub mod falloff_pattern;
pub use falloff_path::{CurvatureFalloff, PathFalloff};
pub mod falloff_reach;
pub mod falloff_screen;
pub use falloff_pattern::{GrayImage, ImageFalloff, NoiseFalloff};
pub use falloff_reach::{CoplanarFalloff, ElementFalloff, SoftSelectionFalloff};
pub use falloff_screen::{AirbrushFalloff, LassoFalloff, LassoStyle, ScreenFalloff};
pub mod sculpt;
/// Blender-derived brush maths (GPL-2.0-or-later, see the module header).
pub mod sculpt_blender;
// The rest stays under `sculpt::` (its `Stroke` would clash with egui's in
// the app's glob imports).
pub use sculpt::{SculptBrush, SculptEdit, SculptItemEdit, SculptSettings};
pub mod symmetrize_tool;
pub mod symmetry;
pub mod symmetry_fix;
pub mod symmetry_sides;
pub mod symmetry_support;
pub mod tool_card;
pub use shared::Shared;
pub use symmetry::{Side, SymmetrizeSide, Symmetry};
pub use tool_card::ToolCard;
pub type Result<T> = std::result::Result<T, String>;

pub mod cone;
pub use cone::*;
pub mod inset;
pub mod mesh_cleanup_ops;
pub use inset::{InsetResult, inset_geometry, shift_geometry};
pub use mesh_cleanup_ops::{CleanupOption, CleanupResult, mesh_cleanup_geometry};
pub mod mesh_attributes;
pub use mesh_attributes::*;

pub mod cone_editor;
mod numeric_session;
pub use numeric_session::*;

mod cone_grid;

pub mod ellipsoid;
pub use ellipsoid::*;
mod ellipsoid_editor;

pub mod shapes;
pub use shapes::*;
pub mod element_state;
pub use element_state::{ElementKind, ElementMask, ElementOp, ElementState, Protection};
mod shape_editor;

pub mod spike;
pub use spike::*;
mod spike_editor;

pub mod extrude;
pub use extrude::*;

pub mod smooth_shift;
pub use smooth_shift::*;

mod smooth_shift_editor;

pub mod bevel;
pub use bevel::*;

pub mod twist_bevel;
pub use twist_bevel::{
    TubeFrame, TwistAxis, TwistBevelOptions, TwistBevelPipe, TwistBevelResult, twist_bevel_frames,
    twist_bevel_geometry, twist_bevel_rings,
};

mod bevel_editor;
pub use bevel_editor::*;

mod bevel_profile;
pub use bevel_profile::*;

mod bevel_weld;

pub mod correspondence;
pub use correspondence::*;
pub mod identity;
pub use identity::*;

mod identity_editor;

pub mod live_extrude;
pub use live_extrude::*;

mod history_policy;
pub(crate) mod mid_tool_undo;
pub use history_policy::*;
pub use mid_tool_undo::{ToolReopen, ToolTweaks};

mod session_ownership;

mod history_completion;
pub use history_completion::*;

mod mode_eligibility;
pub use mode_eligibility::*;

pub mod bridge;
pub use bridge::*;
mod bridge_editor;

pub mod pen_interactive;
pub use pen_interactive::*;
pub mod guides;
pub mod reduction;
pub mod remesh;
pub mod remesh_voxel;
pub mod replicator;
pub mod vertex_normals;
pub use remesh::{CountMode, OctreeMode, OctreeOptions, RemeshOp, RetopoOptions, VoxelOptions};
mod reduction_qem;
pub use reduction::*;
mod reduction_editor;
pub use reduction_editor::*;

pub mod chamfer;
pub use chamfer::*;

pub mod poly_haul;
pub use poly_haul::*;
pub mod poly_haul_modes;
pub use poly_haul_modes::{HaulAmount, HaulTarget, PolyHaulMode, poly_haul_mode_geometry};

pub mod sketch_extrude;
pub mod solid_sketch;
pub mod solid_sketch_editor;
pub use solid_sketch_editor::{SketchEdit, SketchSession};
pub mod solid_sketch_store;
pub mod solid_sketch_symmetry;
pub use sketch_extrude::*;
