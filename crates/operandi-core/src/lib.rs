//! Portable modeling state. No window, graphics, or platform API dependencies.
#[path = "transforms/action_center.rs"]
pub mod action_center;
pub mod bake;
#[path = "sculpting/brush_engine.rs"]
pub mod brush_engine;
#[cfg(feature = "painting")]
#[path = "painting/paint.rs"]
pub mod paint;
#[cfg(feature = "painting")]
#[path = "painting/paint_geometry_brush.rs"]
pub mod paint_geometry_brush;
#[cfg(feature = "painting")]
#[path = "painting/paint_inks.rs"]
pub mod paint_inks;
#[cfg(feature = "painting")]
#[path = "painting/paint_lasso.rs"]
pub mod paint_lasso;
#[cfg(feature = "painting")]
#[path = "painting/paint_sculpt.rs"]
pub mod paint_sculpt;
#[cfg(feature = "painting")]
#[path = "painting/paint_starters.rs"]
pub mod paint_starters;
#[cfg(feature = "painting")]
#[path = "painting/paint_stroke.rs"]
pub mod paint_stroke;
#[cfg(feature = "painting")]
#[path = "painting/paint_tips.rs"]
pub mod paint_tips;
#[path = "selections/selection_inference.rs"]
pub mod selection_inference;
#[path = "uv/uv_atlas.rs"]
pub mod uv_atlas;
#[path = "uv/uv_box.rs"]
pub mod uv_box;
#[path = "uv/uv_charts.rs"]
pub mod uv_charts;
pub use action_center::*;
#[path = "transforms/align_tools.rs"]
pub mod align_tools;
#[path = "transforms/angle_snap.rs"]
pub mod angle_snap;
#[path = "curve_tools/pen.rs"]
pub mod pen;
pub use align_tools::{
    GroundAlign, GroundMode, QuickAlign, QuickAlignKind, SelectionAxisMode, Tack, TackOptions,
};
#[path = "commands/clipboard.rs"]
pub mod clipboard;
#[path = "curve_tools/curve_boolean.rs"]
pub mod curve_boolean;
#[path = "curve_tools/curve_commands.rs"]
pub mod curve_commands;
#[path = "curve_tools/curve_convert.rs"]
pub mod curve_convert;
#[path = "curve_tools/curve_edit.rs"]
pub mod curve_edit;
#[path = "curve_tools/curve_fit.rs"]
pub mod curve_fit;
#[path = "curve_tools/curve_ops.rs"]
pub mod curve_ops;
#[path = "curve_tools/curves.rs"]
pub mod curves;
#[path = "slicing/dicer.rs"]
pub mod dicer;
#[path = "meshops/generator_ops.rs"]
pub mod generator_ops;
#[path = "scene_items/item_merge.rs"]
pub mod item_merge;
#[path = "curve_tools/patch.rs"]
pub mod patch;
#[path = "curve_tools/patch_live.rs"]
pub mod patch_live;
#[path = "curve_tools/path_tools.rs"]
pub mod path_tools;
#[path = "curve_tools/sketch.rs"]
pub mod sketch;
pub use clipboard::{Clipboard, PastePlacement};
#[path = "curve_tools/text.rs"]
pub mod text;
pub use curve_edit::{AddGeometry, CurveEdit};
pub use text::{TextJustification, TextLocation, TextParameters};
#[path = "primitives/unit_insert.rs"]
pub mod unit_insert;
pub use pen::*;
pub use unit_insert::UnitShape;
#[path = "scene_items/background_constraint.rs"]
pub mod background_constraint;
#[path = "render/camera_light.rs"]
pub mod camera_light;
#[path = "render/camera_light_defaults.rs"]
pub mod camera_light_defaults;
#[path = "render/camera_props.rs"]
pub mod camera_props;
#[path = "uv/corner_uv.rs"]
mod corner_uv;
#[path = "tools/drop_action.rs"]
pub mod drop_action;
#[path = "render/environment.rs"]
pub mod environment;
#[path = "render/environment_channels.rs"]
mod environment_channels;
#[path = "scene_items/item_common.rs"]
pub mod item_common;
#[path = "scene_items/item_common_channels.rs"]
pub mod item_common_channels;
#[path = "scene_items/item_delete.rs"]
pub mod item_delete;
#[path = "scene_items/item_extras.rs"]
pub mod item_extras;
#[path = "scene_items/item_groups.rs"]
pub mod item_groups;
#[path = "scene_items/item_list.rs"]
pub mod item_list;
#[path = "scene_items/item_target.rs"]
pub mod item_target;
#[path = "scene_items/item_transform_ops.rs"]
pub mod item_transform_ops;
#[path = "render/locator.rs"]
pub mod locator;
#[path = "render/photometry.rs"]
pub mod photometry;
#[path = "render/physical_sun.rs"]
pub mod physical_sun;
pub use drop_action::DropAction;
#[path = "commands/command_record.rs"]
pub mod command_record;
pub mod editor;
pub use command_record::{CommandClass, CommandRecord, RecordLog, Source, StepRecords};
#[path = "commands/history_settings.rs"]
mod history_settings;
#[path = "transforms/units.rs"]
pub mod units;
#[path = "commands/user_defaults.rs"]
pub mod user_defaults;
pub use history_settings::*;
#[path = "geometry/component_modes.rs"]
pub mod component_modes;
#[path = "polygons/flip.rs"]
mod flip;
#[path = "polygons/flip_it.rs"]
mod flip_it;
#[path = "transforms/freeze.rs"]
pub mod freeze;
#[path = "geometry/fxhash.rs"]
pub mod fxhash;
pub mod interchange;
pub mod lxo;
pub use component_modes::{Mode, SetKind, SetRef};
#[path = "geometry/mesh.rs"]
pub mod mesh;
#[path = "geometry/polygon_kind.rs"]
pub mod polygon_kind;
pub use polygon_kind::{CurveKind, PolygonRepresentation};
#[path = "sculpting/multires.rs"]
pub mod multires;
#[path = "polygons/polygon.rs"]
pub mod polygon;
#[path = "render/post_effects.rs"]
pub mod post_effects;
#[path = "render/render_settings.rs"]
pub mod render_settings;
#[path = "materials/shading.rs"]
pub mod shading;
pub mod subdivision;
pub use polygon::*;
#[path = "materials/blender_principled.rs"]
pub mod blender_principled;
#[path = "materials/cycles_noise.rs"]
pub mod cycles_noise;
#[path = "materials/cycles_voronoi.rs"]
pub mod cycles_voronoi;
#[path = "materials/dispersion.rs"]
pub mod dispersion;
#[path = "materials/gradient.rs"]
pub mod gradient;
#[path = "nodes/graph_view.rs"]
pub mod graph_view;
#[path = "render/light_path_expressions.rs"]
pub mod light_path_expressions;
#[path = "materials/material.rs"]
pub mod material;
#[path = "materials/material_values.rs"]
pub mod material_values;
pub mod materialx;
#[path = "nodes/node_graph.rs"]
pub mod node_graph;
#[path = "nodes/node_material.rs"]
pub mod node_material;
#[path = "nodes/node_view.rs"]
pub mod node_view;
#[path = "nodes/node_view_board.rs"]
pub mod node_view_board;
#[path = "nodes/node_view_commands.rs"]
pub mod node_view_commands;
#[path = "nodes/node_view_settings.rs"]
pub mod node_view_settings;
pub mod openpbr;
#[path = "render/per_light_outputs.rs"]
pub mod per_light_outputs;
#[path = "uv/planar_texture.rs"]
pub mod planar_texture;
#[path = "meshops/procedural.rs"]
pub mod procedural;
#[path = "materials/process_occlusion.rs"]
pub mod process_occlusion;
#[path = "render/render_outputs.rs"]
pub mod render_outputs;
#[path = "materials/shader_eval.rs"]
pub mod shader_eval;
#[path = "materials/shader_func.rs"]
pub mod shader_func;
#[path = "materials/shader_instances.rs"]
pub mod shader_instances;
#[path = "materials/shader_layer_menu.rs"]
pub mod shader_layer_menu;
#[path = "materials/shader_library.rs"]
pub mod shader_library;
#[path = "materials/shader_tree.rs"]
pub mod shader_tree;
#[path = "materials/shader_wgsl.rs"]
pub mod shader_wgsl;
#[path = "materials/texture_locator.rs"]
pub mod texture_locator;
#[path = "anim/transform_channel.rs"]
pub mod transform_channel;
#[path = "polygons/vertex_map_texture.rs"]
pub mod vertex_map_texture;
pub use material::{Material, MaterialKind};
pub use shader_tree::ShaderTreeEdit;
#[path = "polygons/polygon_material.rs"]
pub mod polygon_material;
pub use polygon_material::*;
#[path = "polygons/flatten_ngons.rs"]
mod flatten_ngons;
#[path = "polygons/polygon_align.rs"]
mod polygon_align;
pub use flatten_ngons::*;
#[path = "polygons/polygon_part.rs"]
pub mod polygon_part;
#[path = "polygons/polygon_tags.rs"]
pub mod polygon_tags;
pub use polygon_tags::*;
#[path = "materials/colour_math.rs"]
pub mod colour_math;
#[path = "curve_tools/contour.rs"]
pub mod contour;
#[path = "edges/diagonal_redirect.rs"]
pub mod diagonal_redirect;
#[path = "edges/edge_zip.rs"]
pub mod edge_zip;
#[path = "materials/editor_color.rs"]
pub mod editor_color;
#[path = "polygons/fix_black_dots.rs"]
pub mod fix_black_dots;
#[path = "duplication/merge_instance.rs"]
pub mod merge_instance;
#[path = "painting/mesh_paint.rs"]
pub mod mesh_paint;
#[path = "primitives/plane_item.rs"]
pub mod plane_item;
#[path = "tools/scaffold.rs"]
pub mod scaffold;
#[path = "primitives/teapot.rs"]
pub mod teapot;
pub use editor_color::EditorColor;
#[path = "selections/tag_selection.rs"]
pub mod tag_selection;
pub use tag_selection::{TagRegion, TagSelection};
#[path = "polygons/polygon_quadruple.rs"]
mod polygon_quadruple;
#[path = "polygons/polygon_scale_offset.rs"]
mod polygon_scale_offset;
#[path = "polygons/polygon_spin.rs"]
mod polygon_spin;
#[path = "polygons/polygon_split.rs"]
mod polygon_split;
#[path = "polygons/polygon_unify.rs"]
pub mod polygon_unify;
pub use polygon_part::*;
pub use polygon_unify::*;
#[path = "edges/bridge_commands.rs"]
mod bridge_commands;
#[path = "anim/channels.rs"]
pub mod channels;
#[path = "rigging/character_rig.rs"]
pub mod character_rig;
#[path = "rigging/character_rig_commands.rs"]
pub mod character_rig_commands;
#[path = "commands/command_args.rs"]
pub mod command_args;
#[path = "commands/command_exec.rs"]
pub mod command_exec;
#[path = "commands/command_line.rs"]
pub mod command_line;
#[path = "commands/command_names.rs"]
pub mod command_names;
pub mod command_registry;
#[path = "commands/command_runtime.rs"]
pub mod command_runtime;
#[path = "transforms/set_position.rs"]
mod set_position;
pub use set_position::*;
#[path = "transforms/absolute_scaling.rs"]
mod absolute_scaling;
pub use absolute_scaling::*;
#[path = "polygons/polygon_subdivide.rs"]
pub mod polygon_subdivide;
pub use polygon_subdivide::{
    SubdivideBoundary, SubdivideMethod, SubdivideOptions, SubdivideOutcome, SubdividePreset,
    UnsubdivideOptions, UnsubdividePreset,
};
#[path = "polygons/polygon_triple.rs"]
pub mod polygon_triple;
pub use polygon_triple::*;
#[path = "polygons/mesh_cleanup.rs"]
pub mod mesh_cleanup;
pub use mesh_cleanup::*;
#[path = "polygons/polygon_remove.rs"]
pub mod polygon_remove;
pub use polygon_remove::*;
#[path = "polygons/polygon_merge.rs"]
pub mod polygon_merge;
pub use polygon_merge::*;
#[path = "polygons/polygon_collapse.rs"]
pub mod polygon_collapse;
pub use polygon_collapse::*;
#[path = "edges/edge_extend.rs"]
pub mod edge_extend;
pub use edge_extend::{
    EXTEND_UV_MAP, EdgeExtendOptions, ExtendHandles, ExtendHaul, ExtendPlane, ExtendShape,
    ExtendUvs, edge_extend_at_zero, edge_extend_geometry, edge_extend_shaped,
};
#[path = "edges/edge_extrude.rs"]
pub mod edge_extrude;
pub use edge_extrude::{
    edge_extrude_geometry, edge_extrude_start_width, edge_extrude_with, vertex_extrude_start_width,
};
#[path = "edges/edge_slide.rs"]
pub mod edge_slide;
pub use edge_slide::{
    SlideExtras, edge_slide_extra, edge_slide_geometry, edge_slide_weighted, edge_slide_with,
};
#[path = "edges/edge_crease.rs"]
pub mod edge_crease;
pub use edge_crease::edge_crease_geometry;
#[path = "edges/edge_bevel.rs"]
pub mod edge_bevel;
#[path = "edges/edge_bevel_junctions.rs"]
mod edge_bevel_junctions;
pub use edge_bevel::{
    CornerPattern, EdgeBevelMode, EdgeBevelOptions, EdgeBevelPreset, EdgeBevelResult,
    EdgeBevelSettings, EdgeBevelShape, EdgeBevelToolPreset, EdgeChamferToolPreset, EdgeEnd,
    edge_bevel_geometry, edge_bevel_with,
};
#[path = "edges/edge_commands.rs"]
pub mod edge_commands;
#[path = "edges/edge_relax.rs"]
pub mod edge_relax;
pub use edge_commands::{
    EdgeCommand, edge_bridge, edge_bridge_geometry, edge_command_geometry, edge_delete_geometry,
    edge_join_symmetric_geometry, edge_subdivide_geometry,
};
pub use edge_relax::edge_relax_geometry;
#[path = "slicing/axis_slice.rs"]
pub mod axis_slice;
#[path = "polygons/detriangulate.rs"]
pub mod detriangulate;
#[path = "uv/uv_edit.rs"]
pub mod uv_edit;
#[path = "uv/uv_export.rs"]
pub mod uv_export;
#[path = "uv/uv_maps.rs"]
pub mod uv_maps;
#[path = "uv/uv_ops.rs"]
pub mod uv_ops;
#[path = "uv/uv_peel.rs"]
pub mod uv_peel;
#[path = "uv/uv_pipe.rs"]
pub mod uv_pipe;
#[path = "uv/uv_project.rs"]
pub mod uv_project;
#[path = "uv/uv_relax.rs"]
pub mod uv_relax;
#[path = "uv/uv_seams.rs"]
pub mod uv_seams;
#[path = "uv/uv_symmetry.rs"]
pub mod uv_symmetry;
#[path = "uv/uv_tools.rs"]
pub mod uv_tools;
pub use detriangulate::{DetriangulateOptions, DetriangulatePreset};
#[path = "anim/anim_camera_channels.rs"]
pub mod anim_camera_channels;
#[path = "anim/anim_commands.rs"]
pub mod anim_commands;
#[path = "anim/anim_curve.rs"]
pub mod anim_curve;
#[path = "anim/anim_edit.rs"]
pub mod anim_edit;
#[path = "anim/anim_graph.rs"]
pub mod anim_graph;
#[path = "anim/animation.rs"]
pub mod animation;
#[path = "slicing/boolean.rs"]
pub mod boolean;
#[path = "anim/channel_sets.rs"]
pub mod channel_sets;
#[path = "selections/click_select.rs"]
pub mod click_select;
#[path = "slicing/drill.rs"]
pub mod drill;
#[path = "meshops/eval_build.rs"]
pub mod eval_build;
#[path = "meshops/eval_graph.rs"]
pub mod eval_graph;
#[path = "meshops/eval_state.rs"]
pub mod eval_state;
#[path = "meshops/evaluate.rs"]
pub mod evaluate;
#[path = "rigging/ik.rs"]
pub mod ik;
#[path = "rigging/ik_body.rs"]
pub mod ik_body;
#[path = "rigging/ik_commands.rs"]
pub mod ik_commands;
#[path = "rigging/ik_joint.rs"]
pub mod ik_joint;
#[path = "rigging/ik_switch.rs"]
pub mod ik_switch;
#[path = "kits/kit_folder.rs"]
pub mod kit_folder;
#[path = "kits/kit_import.rs"]
pub mod kit_import;
#[path = "scene_items/kit_item.rs"]
pub mod kit_item;
#[path = "kits/kit_manifest.rs"]
pub mod kit_manifest;
#[path = "meshops/mesh_stack.rs"]
pub mod mesh_stack;
#[path = "meshops/mesh_stack_builtin.rs"]
mod mesh_stack_builtin;
#[path = "meshops/mesh_stack_editor.rs"]
pub mod mesh_stack_editor;
#[path = "meshops/mesh_stack_items.rs"]
pub mod mesh_stack_items;
#[path = "meshops/mesh_stack_tools.rs"]
pub mod mesh_stack_tools;
pub mod meshblend;
pub mod meshblend_spike;
#[path = "meshops/meshop_arc.rs"]
mod meshop_arc;
#[path = "meshops/meshop_assign_selection_set.rs"]
mod meshop_assign_selection_set;
#[path = "meshops/meshop_bend.rs"]
mod meshop_bend;
#[path = "meshops/meshop_bezier_deformer.rs"]
mod meshop_bezier_deformer;
#[path = "meshops/meshop_bridge.rs"]
mod meshop_bridge;
#[path = "meshops/meshop_chamfer_edit.rs"]
mod meshop_chamfer_edit;
#[path = "meshops/meshop_conditional_loop.rs"]
pub mod meshop_conditional_loop;
#[path = "meshops/meshop_create_uv_map.rs"]
mod meshop_create_uv_map;
#[path = "meshops/meshop_create_vertex_map.rs"]
mod meshop_create_vertex_map;
#[path = "meshops/meshop_curve_boolean.rs"]
mod meshop_curve_boolean;
#[path = "meshops/meshop_curve_common.rs"]
mod meshop_curve_common;
#[path = "meshops/meshop_curve_constraint.rs"]
mod meshop_curve_constraint;
#[path = "meshops/meshop_curve_convert.rs"]
mod meshop_curve_convert;
#[path = "meshops/meshop_curve_deform_common.rs"]
mod meshop_curve_deform_common;
#[path = "meshops/meshop_curve_fill.rs"]
mod meshop_curve_fill;
#[path = "meshops/meshop_curve_rebuild.rs"]
mod meshop_curve_rebuild;
#[path = "meshops/meshop_curve_rebuild_spans.rs"]
mod meshop_curve_rebuild_spans;
#[path = "meshops/meshop_curve_slice.rs"]
mod meshop_curve_slice;
#[path = "meshops/meshop_curve_sweep.rs"]
mod meshop_curve_sweep;
#[path = "meshops/meshop_curve_winding.rs"]
mod meshop_curve_winding;
#[path = "meshops/meshop_cutter_common.rs"]
mod meshop_cutter_common;
#[path = "meshops/meshop_deformer_common.rs"]
mod meshop_deformer_common;
#[path = "meshops/meshop_edge_slide.rs"]
mod meshop_edge_slide;
#[path = "meshops/meshop_edges_to_curves.rs"]
mod meshop_edges_to_curves;
#[path = "meshops/meshop_flood_weight.rs"]
mod meshop_flood_weight;
#[path = "meshops/meshop_geodesic_weight.rs"]
mod meshop_geodesic_weight;
#[path = "meshops/meshop_hard_edge.rs"]
mod meshop_hard_edge;
#[path = "meshops/meshop_lace.rs"]
mod meshop_lace;
#[path = "meshops/meshop_lattice.rs"]
pub mod meshop_lattice;
#[path = "meshops/meshop_loop_slice.rs"]
mod meshop_loop_slice;
#[path = "meshops/meshop_magnet.rs"]
mod meshop_magnet;
#[path = "meshops/meshop_morph_influence.rs"]
mod meshop_morph_influence;
#[path = "meshops/meshop_part_tag.rs"]
mod meshop_part_tag;
#[path = "meshops/meshop_pen_slice.rs"]
mod meshop_pen_slice;
#[path = "meshops/meshop_polygon_type.rs"]
mod meshop_polygon_type;
#[path = "meshops/meshop_primitive_slice.rs"]
mod meshop_primitive_slice;
#[path = "meshops/meshop_push_influence.rs"]
mod meshop_push_influence;
#[path = "meshops/meshop_radial_sweep.rs"]
mod meshop_radial_sweep;
#[path = "meshops/meshop_rotate_effector.rs"]
mod meshop_rotate_effector;
#[path = "meshops/meshop_scene.rs"]
mod meshop_scene;
#[path = "meshops/meshop_selection.rs"]
pub mod meshop_selection;
#[path = "meshops/meshop_set_edge_weight.rs"]
mod meshop_set_edge_weight;
#[path = "meshops/meshop_set_rgb.rs"]
mod meshop_set_rgb;
#[path = "meshops/meshop_set_seam.rs"]
mod meshop_set_seam;
#[path = "meshops/meshop_set_vertex_normals.rs"]
mod meshop_set_vertex_normals;
#[path = "meshops/meshop_set_weight.rs"]
mod meshop_set_weight;
#[path = "meshops/meshop_spline_deformer.rs"]
mod meshop_spline_deformer;
#[path = "meshops/meshop_text.rs"]
mod meshop_text;
#[path = "meshops/meshop_thicken.rs"]
mod meshop_thicken;
#[path = "meshops/meshop_tool_pipe.rs"]
pub mod meshop_tool_pipe;
#[path = "meshops/meshop_transform_effector.rs"]
mod meshop_transform_effector;
#[path = "meshops/meshop_twist.rs"]
mod meshop_twist;
#[path = "meshops/meshop_types.rs"]
pub mod meshop_types;
#[path = "meshops/meshop_uv_flip.rs"]
mod meshop_uv_flip;
#[path = "meshops/meshop_uv_relax.rs"]
mod meshop_uv_relax;
#[path = "meshops/meshop_uv_to_mesh.rs"]
mod meshop_uv_to_mesh;
#[path = "meshops/meshop_uv_transform.rs"]
pub mod meshop_uv_transform;
#[path = "meshops/meshop_uv_unwrap_relax.rs"]
mod meshop_uv_unwrap_relax;
#[path = "meshops/meshop_vertex_map_common.rs"]
mod meshop_vertex_map_common;
#[path = "meshops/meshop_vortex.rs"]
mod meshop_vortex;
#[path = "meshops/meshop_weight_influence.rs"]
mod meshop_weight_influence;
#[path = "meshops/meshop_wrap.rs"]
pub mod meshop_wrap;
#[path = "rigging/pose_tool.rs"]
pub mod pose_tool;
#[path = "rigging/rig.rs"]
pub mod rig;
#[path = "rigging/rig_commands.rs"]
pub mod rig_commands;
#[path = "rigging/rig_eval.rs"]
pub mod rig_eval;
#[path = "rigging/rig_expression.rs"]
pub mod rig_expression;
#[path = "rigging/rig_list.rs"]
pub mod rig_list;
#[path = "rigging/rig_schematic.rs"]
pub mod rig_schematic;
#[path = "rigging/rig_user_set.rs"]
pub mod rig_user_set;
#[path = "scene_items/scene_time.rs"]
pub mod scene_time;
#[path = "rigging/schematic_layout.rs"]
pub mod schematic_layout;
#[path = "rigging/schematic_pages.rs"]
pub mod schematic_pages;
#[path = "meshops/script_mesh.rs"]
pub mod script_mesh;
#[path = "rigging/skeleton.rs"]
pub mod skeleton;
#[path = "rigging/skin.rs"]
pub mod skin;
#[path = "rigging/skin_bind.rs"]
pub mod skin_bind;
#[path = "rigging/skin_commands.rs"]
pub mod skin_commands;
#[path = "rigging/skin_weights.rs"]
pub mod skin_weights;
#[path = "anim/time_tick.rs"]
pub mod time_tick;
pub use boolean::{BooleanOp, BooleanOptions, BooleanProblem, DriveMesh};
pub use drill::{DrillAxis, DrillKind, DrillOp, DrillOptions};
#[path = "duplication/duplicate.rs"]
pub mod duplicate;
#[path = "duplication/duplicate_copier.rs"]
pub mod duplicate_copier;
#[path = "duplication/duplicate_hierarchy.rs"]
mod duplicate_hierarchy;
#[path = "duplication/duplicate_items.rs"]
pub mod duplicate_items;
#[path = "duplication/duplicate_sweep.rs"]
pub mod duplicate_sweep;
#[path = "duplication/duplicate_transform.rs"]
pub mod duplicate_transform;
pub use duplicate::{
    Array, Circle, CloneLine, Copies, DuplicateParameters, Lathe, Mirror, MirrorMode, RadialArray,
    Sweep, UvsAround,
};
#[path = "edges/edge_insert.rs"]
pub mod edge_insert;
#[path = "slicing/path_slice.rs"]
pub mod path_slice;
pub use axis_slice::{AxisSlice, AxisSliceMode, axis_slice_bounds, axis_slice_planes};
#[path = "scene_items/profile.rs"]
pub mod profile;
pub use profile::{ProfilePick, ProfilePoint, ProfileShape};
#[path = "slicing/slice.rs"]
pub mod slice;
#[path = "geometry/slice_cut.rs"]
pub(crate) mod slice_cut;
#[path = "slicing/support_edges.rs"]
pub(crate) mod support_edges;
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
#[path = "polygons/polygon_make.rs"]
pub mod polygon_make;
#[path = "polygons/vertex_commands.rs"]
pub mod vertex_commands;
pub use polygon_make::*;
pub use vertex_commands::{
    MakeType, MergeOptions, ReorderMode, VERTEX_BEVEL_MAX_ROUND_LEVEL, VertexCommand,
    VertexMergePreset, WeldDrop, add_points, make_polygon_from_points, make_polygon_typed,
    sync_index_order, vertex_bevel_geometry, vertex_bevel_round, vertex_command_geometry,
    vertex_extrude_geometry, vertex_extrude_with, vertex_join_symmetric_geometry,
    vertex_merge_geometry, vertex_merge_with, vertex_slide_direction, vertex_slide_geometry,
    vertex_tear_geometry, weld_drag_geometry, weld_points,
};
#[path = "tools/numeric.rs"]
pub mod numeric;
#[path = "deformers/tweak.rs"]
pub mod tweak;
pub use tweak::{
    TweakConnect, TweakElement, TweakGrab, TweakMirror, TweakMode, TweakOptions, TweakPlan,
    edge_loop_points, tweak_apply, tweak_mirror, tweak_mirror_points, tweak_plan, tweak_points,
    tweak_points_mirrored,
};
#[path = "geometry/mesh_pack.rs"]
pub mod mesh_pack;
#[path = "geometry/mesh_slot.rs"]
pub mod mesh_slot;
#[path = "nodes/node_graph_reader.rs"]
pub mod node_graph_reader;
#[path = "geometry/packed_mesh.rs"]
pub mod packed_mesh;
pub use mesh_slot::{MeshSlot, MeshView};
#[path = "duplication/mesh_instance.rs"]
pub mod mesh_instance;
pub use packed_mesh::{MeshRead, PackedMesh};
#[path = "commands/document_state.rs"]
mod document_state;
#[path = "commands/persistence.rs"]
pub mod persistence;
pub mod preset;
#[path = "primitives/primitive.rs"]
pub mod primitive;
#[path = "meshops/primitive_ops.rs"]
mod primitive_ops;
#[path = "primitives/primitive_shapes.rs"]
pub mod primitive_shapes;
#[path = "primitives/shape_falloff.rs"]
pub mod shape_falloff;
#[path = "transforms/transform_repeat.rs"]
pub mod transform_repeat;
#[path = "commands/undo_history.rs"]
mod undo_history;
pub use undo_history::History;
#[path = "commands/undo_memory.rs"]
mod undo_memory;
pub use primitive::*;
pub use primitive_shapes::*;
pub use shape_falloff::{FalloffSample, FalloffSpacing, ShapeFalloff, ShapeWeights};
#[path = "uv/primitive_uv.rs"]
pub mod primitive_uv;
#[path = "scene_items/project.rs"]
pub mod project;
#[path = "scene_items/project_paths.rs"]
pub mod project_paths;
pub use primitive_uv::*;
pub use project::Project;
#[path = "duplication/radial.rs"]
pub mod radial;
pub use radial::*;
#[path = "tools/active_task.rs"]
pub mod active_task;
#[path = "render/backdrop.rs"]
pub mod backdrop;
#[path = "scene_items/last_mesh.rs"]
pub mod last_mesh;
#[path = "selections/multi_active.rs"]
pub mod multi_active;
#[path = "transforms/multi_tools.rs"]
pub mod multi_tools;
#[path = "transforms/multi_transform.rs"]
pub mod multi_transform;
#[path = "tools/tool_yield.rs"]
mod tool_yield;
pub use multi_active::ComponentSelection;
#[path = "scene_items/scene.rs"]
pub mod scene;
pub use active_task::ActiveTask;
#[path = "selections/selection.rs"]
pub mod selection;
#[path = "selections/selection_sets.rs"]
pub mod selection_sets;
pub use selection_sets::UseSetMode;
#[path = "selections/selection_continuation.rs"]
mod selection_continuation;
#[path = "selections/selection_neighbour_loop.rs"]
mod selection_neighbour_loop;
#[path = "selections/selection_symmetry.rs"]
mod selection_symmetry;
pub use editor::*;
pub use mesh::*;
pub use scene::*;
pub use selection::*;
#[path = "deformers/bend.rs"]
pub mod bend;
#[path = "tools/click_start.rs"]
pub mod click_start;
#[path = "deformers/deform.rs"]
pub mod deform;
#[path = "deformers/deform_finish.rs"]
pub mod deform_finish;
#[path = "deformers/falloff.rs"]
pub mod falloff;
#[path = "meshops/mesh_stack_deform.rs"]
pub mod mesh_stack_deform;
#[path = "selections/select_tools.rs"]
pub mod select_tools;
#[path = "polygons/topology_cleanup.rs"]
pub mod topology_cleanup;
#[path = "curve_tools/topology_pen.rs"]
pub mod topology_pen;
pub use bend::BendOptions;
pub use deform::{DeformPreset, PushNormals, SmoothOptions};
pub use deform_finish::{
    JitterOptions, LinearAlignOptions, LinearMode, QuantizeOptions, RadialAlignOptions, RadialMode,
    RelaxMeshOptions,
};
pub use topology_pen::{PenEdit, PenMode, TopologyPenOptions};
#[path = "transforms/element_center.rs"]
pub mod element_center;
#[path = "tools/haul.rs"]
pub mod haul;
pub use element_center::{CenterPress, element_axes, element_frame};
#[path = "transforms/pivot_service.rs"]
pub mod pivot_service;
#[path = "tools/preset_store.rs"]
mod preset_store;
#[path = "transforms/selection_axes.rs"]
pub mod selection_axes;
#[path = "tools/tool_preset.rs"]
pub mod tool_preset;
#[path = "tools/tool_reset.rs"]
mod tool_reset;
#[path = "tools/tool_session.rs"]
mod tool_session;
pub use preset_store::{PresetKey, ToolPresets};
pub use tool_preset::{PresetRow, PresetSession};
pub use tool_session::ToolSession;
#[path = "scene_items/scene_index.rs"]
pub mod scene_index;
#[path = "geometry/shared.rs"]
pub mod shared;
#[path = "geometry/topology.rs"]
pub mod topology;
#[path = "geometry/topology_cache.rs"]
pub mod topology_cache;
pub use falloff::{
    ActiveFalloff, CylinderFalloff, Falloff, FalloffShape, FalloffSymmetric, LinearFalloff,
    MixMode, RadialFalloff, SelectionFalloff, VertexMapFalloff, VertexMapMode,
};
#[path = "deformers/falloff_path.rs"]
pub mod falloff_path;
#[path = "deformers/falloff_pattern.rs"]
pub mod falloff_pattern;
pub use falloff_path::{CurvatureFalloff, PathFalloff};
#[path = "deformers/falloff_reach.rs"]
pub mod falloff_reach;
#[path = "deformers/falloff_screen.rs"]
pub mod falloff_screen;
pub use falloff_pattern::{GrayImage, ImageFalloff, NoiseFalloff};
pub use falloff_reach::{CoplanarFalloff, ElementFalloff, SoftSelectionFalloff};
pub use falloff_screen::{AirbrushFalloff, LassoFalloff, LassoStyle, ScreenFalloff};
#[path = "sculpting/sculpt.rs"]
pub mod sculpt;
/// Blender-derived brush maths (GPL-2.0-or-later, see the module header).
#[path = "sculpting/sculpt_blender/mod.rs"]
pub mod sculpt_blender;
// The rest stays under `sculpt::` (its `Stroke` would clash with egui's in
// the app's glob imports).
pub use sculpt::{SculptBrush, SculptEdit, SculptItemEdit, SculptSettings};
#[path = "transforms/symmetrize_tool.rs"]
pub mod symmetrize_tool;
#[path = "transforms/symmetry.rs"]
pub mod symmetry;
#[path = "transforms/symmetry_fix.rs"]
pub mod symmetry_fix;
#[path = "transforms/symmetry_sides.rs"]
pub mod symmetry_sides;
#[path = "transforms/symmetry_support.rs"]
pub mod symmetry_support;
#[path = "tools/tool_card.rs"]
pub mod tool_card;
pub use shared::Shared;
pub use symmetry::{Side, SymmetrizeSide, Symmetry};
pub use tool_card::ToolCard;
pub type Result<T> = std::result::Result<T, String>;

#[path = "primitives/cone.rs"]
pub mod cone;
pub use cone::*;
#[path = "polygons/inset.rs"]
pub mod inset;
#[path = "polygons/mesh_cleanup_ops.rs"]
pub mod mesh_cleanup_ops;
pub use inset::{InsetResult, inset_geometry, shift_geometry};
pub use mesh_cleanup_ops::{CleanupOption, CleanupResult, mesh_cleanup_geometry};
#[path = "geometry/mesh_attributes.rs"]
pub mod mesh_attributes;
pub use mesh_attributes::*;

#[path = "primitives/cone_editor.rs"]
pub mod cone_editor;
#[path = "tools/live_apply.rs"]
mod live_apply;
#[path = "tools/numeric_session.rs"]
mod numeric_session;
pub use numeric_session::*;

#[path = "primitives/cone_grid.rs"]
mod cone_grid;

#[path = "primitives/ellipsoid.rs"]
pub mod ellipsoid;
pub use ellipsoid::*;
#[path = "primitives/ellipsoid_editor.rs"]
mod ellipsoid_editor;

#[path = "primitives/shapes.rs"]
pub mod shapes;
pub use shapes::*;
#[path = "selections/element_state.rs"]
pub mod element_state;
pub use element_state::{ElementKind, ElementMask, ElementOp, ElementState, Protection};
#[path = "primitives/shape_editor.rs"]
mod shape_editor;

#[path = "polygons/spike.rs"]
pub mod spike;
pub use spike::*;
#[path = "polygons/spike_editor.rs"]
mod spike_editor;

#[path = "polygons/extrude.rs"]
pub mod extrude;
pub use extrude::*;

#[path = "polygons/smooth_shift.rs"]
pub mod smooth_shift;
pub use smooth_shift::*;

#[path = "polygons/smooth_shift_editor.rs"]
mod smooth_shift_editor;

#[path = "polygons/bevel.rs"]
pub mod bevel;
pub use bevel::*;

#[path = "polygons/twist_bevel.rs"]
pub mod twist_bevel;
pub use twist_bevel::{
    TubeFrame, TwistAxis, TwistBevelOptions, TwistBevelPipe, TwistBevelResult, twist_bevel_frames,
    twist_bevel_geometry, twist_bevel_rings,
};

#[path = "polygons/bevel_editor.rs"]
mod bevel_editor;
pub use bevel_editor::*;

#[path = "polygons/bevel_profile.rs"]
mod bevel_profile;
pub use bevel_profile::*;

#[path = "polygons/bevel_weld.rs"]
mod bevel_weld;

#[path = "polygons/correspondence.rs"]
pub mod correspondence;
pub use correspondence::*;
#[path = "scene_items/identity.rs"]
pub mod identity;
pub use identity::*;

#[path = "scene_items/identity_editor.rs"]
mod identity_editor;

#[path = "polygons/live_extrude.rs"]
pub mod live_extrude;
pub use live_extrude::*;

#[path = "commands/history_policy.rs"]
mod history_policy;
#[path = "tools/mid_tool_undo.rs"]
pub(crate) mod mid_tool_undo;
pub use history_policy::*;
pub use mid_tool_undo::{ToolReopen, ToolTweaks};

#[path = "commands/session_ownership.rs"]
mod session_ownership;

#[path = "commands/history_completion.rs"]
mod history_completion;
pub use history_completion::*;

#[path = "selections/mode_eligibility.rs"]
mod mode_eligibility;
pub use mode_eligibility::*;

#[path = "edges/bridge.rs"]
pub mod bridge;
pub use bridge::*;
#[path = "edges/bridge_editor.rs"]
mod bridge_editor;

#[path = "curve_tools/pen_interactive.rs"]
pub mod pen_interactive;
pub use pen_interactive::*;
#[path = "scene_items/guides.rs"]
pub mod guides;
#[path = "polygons/reduction.rs"]
pub mod reduction;
#[path = "sculpting/remesh.rs"]
pub mod remesh;
#[path = "sculpting/remesh_voxel.rs"]
pub mod remesh_voxel;
#[path = "duplication/replicator.rs"]
pub mod replicator;
#[path = "polygons/vertex_normals.rs"]
pub mod vertex_normals;
pub use remesh::{CountMode, OctreeMode, OctreeOptions, RemeshOp, RetopoOptions, VoxelOptions};
#[path = "polygons/reduction_qem.rs"]
mod reduction_qem;
pub use reduction::*;
#[path = "polygons/reduction_editor.rs"]
mod reduction_editor;
pub use reduction_editor::*;

#[path = "edges/chamfer.rs"]
pub mod chamfer;
pub use chamfer::*;

#[path = "deformers/poly_haul.rs"]
pub mod poly_haul;
pub use poly_haul::*;
#[path = "deformers/poly_haul_modes.rs"]
pub mod poly_haul_modes;
pub use poly_haul_modes::{HaulAmount, HaulTarget, PolyHaulMode, poly_haul_mode_geometry};

#[path = "curve_tools/sketch_extrude.rs"]
pub mod sketch_extrude;
pub mod solid_sketch;
#[path = "curve_tools/solid_sketch_editor.rs"]
pub mod solid_sketch_editor;
pub use solid_sketch_editor::{SketchEdit, SketchSession};
#[path = "curve_tools/solid_sketch_store.rs"]
pub mod solid_sketch_store;
#[path = "curve_tools/solid_sketch_symmetry.rs"]
pub mod solid_sketch_symmetry;
#[path = "scene_items/statistics.rs"]
pub mod statistics;
#[path = "scene_items/statistics_edit.rs"]
pub mod statistics_edit;
pub use sketch_extrude::*;
