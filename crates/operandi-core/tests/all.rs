//! Every core integration test in one test program.
//!
//! Each `tests/*.rs` file used to build as its own program: about 240 links
//! of a ~100 MB binary, and on macOS a first-launch security scan of each
//! one, on every core change. Here they are modules of one program instead,
//! so a core change links once and launches once (Jason, 2026-11-15).
//!
//! The files stay where they were, so their relative paths (`#[path]`,
//! `include_str!`) still work. `autotests = false` in Cargo.toml stops Cargo
//! from also building each file alone, so a new file must be listed below:
//! `every_test_file_is_listed` fails, naming the line to add, until it is.
//! Run one file's tests with `cargo test --locked --workspace -- <file>::`
//! (a name filter; `--test all` or `-p operandi-core` alone would pick other
//! dependency features and rebuild).
//!
//! Several files include the same helper (`support/clean_spec.rs`) with
//! `#[path]`, each as its own copy, as when they were separate programs;
//! hence the `duplicate_mod` allowance.
#![allow(clippy::duplicate_mod)]

#[path = "a31_combined.rs"]
mod a31_combined;
#[path = "a37_combined.rs"]
mod a37_combined;
#[path = "absolute_scaling_spec.rs"]
mod absolute_scaling_spec;
#[path = "ac_symmetry_rulebook.rs"]
mod ac_symmetry_rulebook;
#[path = "action_center_islands.rs"]
mod action_center_islands;
#[path = "action_center_local_axes.rs"]
mod action_center_local_axes;
#[path = "action_center_local_edges.rs"]
mod action_center_local_edges;
#[path = "action_center_rulebook.rs"]
mod action_center_rulebook;
#[path = "action_center_scale_clean_spec.rs"]
mod action_center_scale_clean_spec;
#[path = "action_centers.rs"]
mod action_centers;
#[path = "active_task.rs"]
mod active_task;
#[path = "add_loop_slide_options.rs"]
mod add_loop_slide_options;
#[path = "animation_graph.rs"]
mod animation_graph;
#[path = "animation_time_keys.rs"]
mod animation_time_keys;
#[path = "animation_timeline.rs"]
mod animation_timeline;
#[path = "axis_slice_cases.rs"]
mod axis_slice_cases;
#[path = "backdrop.rs"]
mod backdrop;
#[path = "backdrop_gaps.rs"]
mod backdrop_gaps;
#[path = "background_constraint.rs"]
mod background_constraint;
#[path = "bend.rs"]
mod bend;
#[path = "bend_spine_fit.rs"]
mod bend_spine_fit;
#[path = "bend_v21.rs"]
mod bend_v21;
#[path = "bevel_activation_clean_spec.rs"]
mod bevel_activation_clean_spec;
#[path = "bevel_amendment.rs"]
mod bevel_amendment;
#[path = "bevel_drag_clean_spec.rs"]
mod bevel_drag_clean_spec;
#[path = "big_scene_save.rs"]
mod big_scene_save;
#[path = "booleans.rs"]
mod booleans;
#[path = "box_haul.rs"]
mod box_haul;
#[path = "bridge_clean_spec.rs"]
mod bridge_clean_spec;
#[path = "bridge_edges.rs"]
mod bridge_edges;
#[path = "bridge_s1.rs"]
mod bridge_s1;
#[path = "camera_light.rs"]
mod camera_light;
#[path = "camera_properties.rs"]
mod camera_properties;
#[path = "capsule_drawn_height.rs"]
mod capsule_drawn_height;
#[path = "chamfer_basic.rs"]
mod chamfer_basic;
#[path = "chamfer_edit_clean_spec.rs"]
mod chamfer_edit_clean_spec;
#[path = "checkpoint_boundary.rs"]
mod checkpoint_boundary;
#[path = "checks_ac_meshops_replay.rs"]
mod checks_ac_meshops_replay;
#[path = "click_start.rs"]
mod click_start;
#[path = "clipboard.rs"]
mod clipboard;
#[path = "command_conventions_clean_spec.rs"]
mod command_conventions_clean_spec;
#[path = "command_runtime.rs"]
mod command_runtime;
#[path = "cone_clean_spec.rs"]
mod cone_clean_spec;
#[path = "cone_grid.rs"]
mod cone_grid;
#[path = "cone_numeric.rs"]
mod cone_numeric;
#[path = "contour_spec.rs"]
mod contour_spec;
#[path = "curve_boolean.rs"]
mod curve_boolean;
#[path = "curve_commands.rs"]
mod curve_commands;
#[path = "curve_convert.rs"]
mod curve_convert;
#[path = "curve_edit.rs"]
mod curve_edit;
#[path = "deform_finish.rs"]
mod deform_finish;
#[path = "deform_presets.rs"]
mod deform_presets;
#[path = "delete_orphan_points.rs"]
mod delete_orphan_points;
#[path = "diagonal_redirect_spec.rs"]
mod diagonal_redirect_spec;
#[path = "dicer.rs"]
mod dicer;
#[path = "doors.rs"]
mod doors;
#[path = "drag_cost_probe.rs"]
mod drag_cost_probe;
#[path = "drag_creation_clean_spec.rs"]
mod drag_creation_clean_spec;
#[path = "drills.rs"]
mod drills;
#[path = "drop_action.rs"]
mod drop_action;
#[path = "duplicate_hierarchy.rs"]
mod duplicate_hierarchy;
#[path = "duplicate_instances.rs"]
mod duplicate_instances;
#[path = "duplicate_tools.rs"]
mod duplicate_tools;
#[path = "duplicate_transform.rs"]
mod duplicate_transform;
#[path = "edge_bevel.rs"]
mod edge_bevel;
#[path = "edge_bevel_measured.rs"]
mod edge_bevel_measured;
#[path = "edge_bevel_normal_map.rs"]
mod edge_bevel_normal_map;
#[path = "edge_bevel_options.rs"]
mod edge_bevel_options;
#[path = "edge_commands.rs"]
mod edge_commands;
#[path = "edge_delete_border_points.rs"]
mod edge_delete_border_points;
#[path = "edge_extend_extrude.rs"]
mod edge_extend_extrude;
#[path = "edge_extend_rotate_scale.rs"]
mod edge_extend_rotate_scale;
#[path = "edge_insert.rs"]
mod edge_insert;
#[path = "edge_loop_delete.rs"]
mod edge_loop_delete;
#[path = "edge_loop_double_click.rs"]
mod edge_loop_double_click;
#[path = "edge_slide_measured.rs"]
mod edge_slide_measured;
#[path = "edge_zip_spec.rs"]
mod edge_zip_spec;
#[path = "editor_color.rs"]
mod editor_color;
#[path = "ellipsoid_clean_spec.rs"]
mod ellipsoid_clean_spec;
#[path = "ellipsoid_numeric.rs"]
mod ellipsoid_numeric;
#[path = "empty_mesh_item_transform.rs"]
mod empty_mesh_item_transform;
#[path = "env_texture_locator.rs"]
mod env_texture_locator;
#[path = "environments.rs"]
mod environments;
#[path = "eval_build.rs"]
mod eval_build;
#[path = "eval_graph.rs"]
mod eval_graph;
#[path = "eval_probe.rs"]
mod eval_probe;
#[path = "extrude_clean_spec.rs"]
mod extrude_clean_spec;
#[path = "eye_hides_children.rs"]
mod eye_hides_children;
#[path = "falloff.rs"]
mod falloff;
#[path = "falloff_batch3.rs"]
mod falloff_batch3;
#[path = "falloff_path.rs"]
mod falloff_path;
#[path = "falloff_pattern.rs"]
mod falloff_pattern;
#[path = "falloff_reach.rs"]
mod falloff_reach;
#[path = "falloff_screen.rs"]
mod falloff_screen;
#[path = "fix_black_dots_spec.rs"]
mod fix_black_dots_spec;
#[path = "flip_basic.rs"]
mod flip_basic;
#[path = "flip_triple_integration.rs"]
mod flip_triple_integration;
#[path = "freeze_curves.rs"]
mod freeze_curves;
#[path = "freeze_transforms.rs"]
mod freeze_transforms;
#[path = "generator_ops.rs"]
mod generator_ops;
#[path = "gradient.rs"]
mod gradient;
#[path = "group_locators.rs"]
mod group_locators;
#[path = "group_material_extras.rs"]
mod group_material_extras;
#[path = "haul.rs"]
mod haul;
#[path = "history_completion.rs"]
mod history_completion;
#[path = "i1_completion.rs"]
mod i1_completion;
#[path = "i1_identity.rs"]
mod i1_identity;
#[path = "i1_review_corrections.rs"]
mod i1_review_corrections;
#[path = "i2_fastpath_guard.rs"]
mod i2_fastpath_guard;
#[path = "i2_gap_confirmation.rs"]
mod i2_gap_confirmation;
#[path = "i2_live_history.rs"]
mod i2_live_history;
#[path = "i2_origin_active_save_confirmation.rs"]
mod i2_origin_active_save_confirmation;
#[path = "i2_review_fixes.rs"]
mod i2_review_fixes;
#[path = "icon_studio.rs"]
mod icon_studio;
#[path = "identity_positions.rs"]
mod identity_positions;
#[path = "ik_body.rs"]
mod ik_body;
#[path = "ik_chain.rs"]
mod ik_chain;
#[path = "ik_limits.rs"]
mod ik_limits;
#[path = "ik_switch.rs"]
mod ik_switch;
#[path = "ik_ties.rs"]
mod ik_ties;
#[path = "ik_two_bone.rs"]
mod ik_two_bone;
#[path = "incidence_effects.rs"]
mod incidence_effects;
#[path = "inset_clean_spec.rs"]
mod inset_clean_spec;
#[path = "inset_shift_tools.rs"]
mod inset_shift_tools;
#[path = "item_groups.rs"]
mod item_groups;
#[path = "item_list_header.rs"]
mod item_list_header;
#[path = "item_reorder.rs"]
mod item_reorder;
#[path = "item_tabs_tier3.rs"]
mod item_tabs_tier3;
#[path = "item_transforms_measured.rs"]
mod item_transforms_measured;
#[path = "join_symmetry.rs"]
mod join_symmetry;
#[path = "last_mesh.rs"]
mod last_mesh;
#[path = "last_mesh_delete.rs"]
mod last_mesh_delete;
#[path = "light_properties.rs"]
mod light_properties;
#[path = "light_properties_tier2.rs"]
mod light_properties_tier2;
#[path = "light_properties_tier3.rs"]
mod light_properties_tier3;
#[path = "light_type_convert.rs"]
mod light_type_convert;
#[path = "local_tunnel_rings.rs"]
mod local_tunnel_rings;
#[path = "lock_hide.rs"]
mod lock_hide;
#[path = "loop_ngon_rim.rs"]
mod loop_ngon_rim;
#[path = "loop_slice_options.rs"]
mod loop_slice_options;
#[path = "loop_slice_selection.rs"]
mod loop_slice_selection;
#[path = "loop_slice_surface_probe.rs"]
mod loop_slice_surface_probe;
#[path = "loop_symmetry.rs"]
mod loop_symmetry;
#[path = "lxo.rs"]
mod lxo;
#[path = "lxo_budget_preflight.rs"]
mod lxo_budget_preflight;
#[path = "lxo_legacy.rs"]
mod lxo_legacy;
#[path = "lxo_local_corpus.rs"]
mod lxo_local_corpus;
#[path = "lxo_mesh_preset.rs"]
mod lxo_mesh_preset;
#[path = "lxo_old_formats.rs"]
mod lxo_old_formats;
#[path = "lxo_phase1_clean_spec.rs"]
mod lxo_phase1_clean_spec;
#[path = "lxo_resource_safety.rs"]
mod lxo_resource_safety;
#[path = "lxp_local_presets.rs"]
mod lxp_local_presets;
#[path = "m2_demo_scenes.rs"]
mod m2_demo_scenes;
#[path = "m3_vertex_tools.rs"]
mod m3_vertex_tools;
#[path = "make_edges_clean_spec.rs"]
mod make_edges_clean_spec;
#[path = "make_polygon_symmetry.rs"]
mod make_polygon_symmetry;
#[path = "make_split_clean_spec.rs"]
mod make_split_clean_spec;
#[path = "material_distance_fields.rs"]
mod material_distance_fields;
#[path = "merge_align.rs"]
mod merge_align;
#[path = "merge_holes.rs"]
mod merge_holes;
#[path = "merge_instance_spec.rs"]
mod merge_instance_spec;
#[path = "mesh_cleanup_basic.rs"]
mod mesh_cleanup_basic;
#[path = "mesh_cleanup_clean_spec.rs"]
mod mesh_cleanup_clean_spec;
#[path = "mesh_commands_measured.rs"]
mod mesh_commands_measured;
#[path = "mesh_paint_spec.rs"]
mod mesh_paint_spec;
#[path = "meshops_boolean_merge.rs"]
mod meshops_boolean_merge;
#[path = "meshops_links.rs"]
mod meshops_links;
#[path = "meshops_selops.rs"]
mod meshops_selops;
#[path = "meshops_stress.rs"]
mod meshops_stress;
#[path = "meshops_tools.rs"]
mod meshops_tools;
#[path = "mid_tool_undo.rs"]
mod mid_tool_undo;
#[path = "mode_availability.rs"]
mod mode_availability;
#[path = "multi_active_meshes.rs"]
mod multi_active_meshes;
#[path = "multi_active_tools.rs"]
mod multi_active_tools;
#[path = "multi_active_transforms.rs"]
mod multi_active_transforms;
#[path = "multi_mesh_undo_trim.rs"]
mod multi_mesh_undo_trim;
#[path = "native_wire_contract.rs"]
mod native_wire_contract;
#[path = "new_mesh_clean_spec.rs"]
mod new_mesh_clean_spec;
#[path = "new_point_uvs_measured.rs"]
mod new_point_uvs_measured;
#[path = "new_primitives_oracles.rs"]
mod new_primitives_oracles;
#[path = "new_primitives_session.rs"]
mod new_primitives_session;
#[path = "new_scene_clean_spec.rs"]
mod new_scene_clean_spec;
#[path = "nits_batch_3.rs"]
mod nits_batch_3;
#[path = "nits_batch_4.rs"]
mod nits_batch_4;
#[path = "nits_batch_5.rs"]
mod nits_batch_5;
#[path = "numeric_session_guard.rs"]
mod numeric_session_guard;
#[path = "offset_flatten_clean_spec.rs"]
mod offset_flatten_clean_spec;
#[path = "one_point_polygon.rs"]
mod one_point_polygon;
#[path = "packed_mesh.rs"]
mod packed_mesh;
#[path = "paint_images.rs"]
mod paint_images;
#[path = "paint_inks.rs"]
mod paint_inks;
#[path = "paint_sculpt.rs"]
mod paint_sculpt;
#[path = "paint_strokes.rs"]
mod paint_strokes;
#[path = "paint_tips_strokes.rs"]
mod paint_tips_strokes;
#[path = "paint_tools.rs"]
mod paint_tools;
#[path = "paint_uv.rs"]
mod paint_uv;
#[path = "parity_mesh_tools.rs"]
mod parity_mesh_tools;
#[path = "parity_selection.rs"]
mod parity_selection;
#[path = "patch_live.rs"]
mod patch_live;
#[path = "path_slice_cases.rs"]
mod path_slice_cases;
#[path = "path_slice_tools.rs"]
mod path_slice_tools;
#[path = "path_tools.rs"]
mod path_tools;
#[path = "pen_clean_spec.rs"]
mod pen_clean_spec;
#[path = "pen_interactive_basic.rs"]
mod pen_interactive_basic;
#[path = "pen_interactive_general.rs"]
mod pen_interactive_general;
#[path = "pen_interactive_options.rs"]
mod pen_interactive_options;
#[path = "pen_make_quads.rs"]
mod pen_make_quads;
#[path = "pen_numeric.rs"]
mod pen_numeric;
#[path = "pen_walls.rs"]
mod pen_walls;
#[path = "perf_snapshot.rs"]
mod perf_snapshot;
#[path = "performance_invariants.rs"]
mod performance_invariants;
#[path = "poly_flip_clean_spec.rs"]
mod poly_flip_clean_spec;
#[path = "poly_haul_clean_spec.rs"]
mod poly_haul_clean_spec;
#[path = "poly_haul_modes_clean_spec.rs"]
mod poly_haul_modes_clean_spec;
#[path = "poly_reduce_clean_spec.rs"]
mod poly_reduce_clean_spec;
#[path = "polygon_align_basic.rs"]
mod polygon_align_basic;
#[path = "polygon_bevel_measured.rs"]
mod polygon_bevel_measured;
#[path = "polygon_collapse_basic.rs"]
mod polygon_collapse_basic;
#[path = "polygon_connected_double_click.rs"]
mod polygon_connected_double_click;
#[path = "polygon_extrude_measured.rs"]
mod polygon_extrude_measured;
#[path = "polygon_make_basic.rs"]
mod polygon_make_basic;
#[path = "polygon_merge.rs"]
mod polygon_merge;
#[path = "polygon_quadruple_basic.rs"]
mod polygon_quadruple_basic;
#[path = "polygon_reduction.rs"]
mod polygon_reduction;
#[path = "polygon_remove.rs"]
mod polygon_remove;
#[path = "polygon_scale_offset_affine.rs"]
mod polygon_scale_offset_affine;
#[path = "polygon_scale_offset_core.rs"]
mod polygon_scale_offset_core;
#[path = "polygon_scale_offset_eligibility.rs"]
mod polygon_scale_offset_eligibility;
#[path = "polygon_set_part_upgrade.rs"]
mod polygon_set_part_upgrade;
#[path = "polygon_spin_basic.rs"]
mod polygon_spin_basic;
#[path = "polygon_subdivide.rs"]
mod polygon_subdivide;
#[path = "polygon_triple.rs"]
mod polygon_triple;
#[path = "polygon_unify_basic.rs"]
mod polygon_unify_basic;
#[path = "polygons_m17.rs"]
mod polygons_m17;
#[path = "polyscale_offset_clean_spec.rs"]
mod polyscale_offset_clean_spec;
#[path = "pose_tool.rs"]
mod pose_tool;
#[path = "primitive_moved_mesh.rs"]
mod primitive_moved_mesh;
#[path = "primitive_settings.rs"]
mod primitive_settings;
#[path = "primitive_uvs.rs"]
mod primitive_uvs;
#[path = "primitives_m16.rs"]
mod primitives_m16;
#[path = "primitives_measured.rs"]
mod primitives_measured;
#[path = "process_occlusion.rs"]
mod process_occlusion;
#[path = "project.rs"]
mod project;
#[path = "project_paths.rs"]
mod project_paths;
#[path = "quadruple_clean_spec.rs"]
mod quadruple_clean_spec;
#[path = "render_item.rs"]
mod render_item;
#[path = "render_outputs.rs"]
mod render_outputs;
#[path = "render_outputs_d.rs"]
mod render_outputs_d;
#[path = "replicator.rs"]
mod replicator;
#[path = "rig_bake.rs"]
mod rig_bake;
#[path = "rig_expression.rs"]
mod rig_expression;
#[path = "rig_links.rs"]
mod rig_links;
#[path = "rig_list.rs"]
mod rig_list;
#[path = "rig_milestones.rs"]
mod rig_milestones;
#[path = "rig_modifiers.rs"]
mod rig_modifiers;
#[path = "rig_phase4.rs"]
mod rig_phase4;
#[path = "rig_phase5_milestones.rs"]
mod rig_phase5_milestones;
#[path = "rig_phase6_milestones.rs"]
mod rig_phase6_milestones;
#[path = "rig_schematic.rs"]
mod rig_schematic;
#[path = "rig_setup.rs"]
mod rig_setup;
#[path = "scene_budget.rs"]
mod scene_budget;
#[path = "scene_transfer.rs"]
mod scene_transfer;
#[path = "scripts_gate.rs"]
mod scripts_gate;
#[path = "sculpt_brush_engine.rs"]
mod sculpt_brush_engine;
#[path = "sculpt_selection.rs"]
mod sculpt_selection;
#[path = "select_tools.rs"]
mod select_tools;
#[path = "selection_continuation_m15.rs"]
mod selection_continuation_m15;
#[path = "selection_dense_mesh.rs"]
mod selection_dense_mesh;
#[path = "selection_falloff.rs"]
mod selection_falloff;
#[path = "selection_inference.rs"]
mod selection_inference;
#[path = "selection_m15.rs"]
mod selection_m15;
#[path = "selection_measured.rs"]
mod selection_measured;
#[path = "selection_mixed_topology.rs"]
mod selection_mixed_topology;
#[path = "selection_next_loop.rs"]
mod selection_next_loop;
#[path = "selection_sets.rs"]
mod selection_sets;
#[path = "set_position_basic.rs"]
mod set_position_basic;
#[path = "set_position_clean_spec.rs"]
mod set_position_clean_spec;
#[path = "set_tags_clean_spec.rs"]
mod set_tags_clean_spec;
#[path = "shader_layer_image.rs"]
mod shader_layer_image;
#[path = "shader_layer_menu.rs"]
mod shader_layer_menu;
#[path = "shader_settings.rs"]
mod shader_settings;
#[path = "shader_tree_stale_selection.rs"]
mod shader_tree_stale_selection;
#[path = "shared_payloads.rs"]
mod shared_payloads;
#[path = "skeleton.rs"]
mod skeleton;
#[path = "sketch_extrude_clean_spec.rs"]
mod sketch_extrude_clean_spec;
#[path = "sketch_extrude_drag_clean_spec.rs"]
mod sketch_extrude_drag_clean_spec;
#[path = "skin.rs"]
mod skin;
#[path = "skin_bind.rs"]
mod skin_bind;
#[path = "skin_weights.rs"]
mod skin_weights;
#[path = "slice_tools.rs"]
mod slice_tools;
#[path = "smooth_shading.rs"]
mod smooth_shading;
#[path = "smooth_shift.rs"]
mod smooth_shift;
#[path = "smooth_shift_clean_spec.rs"]
mod smooth_shift_clean_spec;
#[path = "solid_sketch_symmetry.rs"]
mod solid_sketch_symmetry;
#[path = "solid_sketch_tool.rs"]
mod solid_sketch_tool;
#[path = "spike_numeric.rs"]
mod spike_numeric;
#[path = "spikey_clean_spec.rs"]
mod spikey_clean_spec;
#[path = "stab01_history_policy.rs"]
mod stab01_history_policy;
#[path = "stab02_ownership.rs"]
mod stab02_ownership;
#[path = "stab02c_activation.rs"]
mod stab02c_activation;
#[path = "statistics.rs"]
mod statistics;
#[path = "subdivision_commands.rs"]
mod subdivision_commands;
#[path = "symmetrize_tool.rs"]
mod symmetrize_tool;
#[path = "symmetry.rs"]
mod symmetry;
#[path = "symmetry_creation.rs"]
mod symmetry_creation;
#[path = "symmetry_movement.rs"]
mod symmetry_movement;
#[path = "symmetry_phase2.rs"]
mod symmetry_phase2;
#[path = "symmetry_phase3.rs"]
mod symmetry_phase3;
#[path = "symmetry_selection.rs"]
mod symmetry_selection;
#[path = "teapot_spec.rs"]
mod teapot_spec;
#[path = "tool_activation_live.rs"]
mod tool_activation_live;
#[path = "tool_card.rs"]
mod tool_card;
#[path = "tool_list.rs"]
mod tool_list;
#[path = "tool_preset_golden.rs"]
mod tool_preset_golden;
#[path = "tool_presets.rs"]
mod tool_presets;
#[path = "tool_speed_probe.rs"]
mod tool_speed_probe;
#[path = "tool_switch.rs"]
mod tool_switch;
#[path = "tool_switch_drop.rs"]
mod tool_switch_drop;
#[path = "tool_uvs.rs"]
mod tool_uvs;
#[path = "topology_cleanup.rs"]
mod topology_cleanup;
#[path = "topology_pen.rs"]
mod topology_pen;
#[path = "topology_tools.rs"]
mod topology_tools;
#[path = "transform_steps.rs"]
mod transform_steps;
#[path = "transformed_component_tools.rs"]
mod transformed_component_tools;
#[path = "tree_list_commands.rs"]
mod tree_list_commands;
#[path = "tree_select_keeps_tool.rs"]
mod tree_select_keeps_tool;
#[path = "tweak_oracles.rs"]
mod tweak_oracles;
#[path = "twist_bevel.rs"]
mod twist_bevel;
#[path = "undo_fold.rs"]
mod undo_fold;
#[path = "undo_marks.rs"]
mod undo_marks;
#[path = "undo_memory_budget.rs"]
mod undo_memory_budget;
#[path = "unit_primitive_clean_spec.rs"]
mod unit_primitive_clean_spec;
#[path = "unit_primitives.rs"]
mod unit_primitives;
#[path = "unit_shapes.rs"]
mod unit_shapes;
#[path = "uv_atlas_peel_export.rs"]
mod uv_atlas_peel_export;
#[path = "uv_editing.rs"]
mod uv_editing;
#[path = "uv_ops.rs"]
mod uv_ops;
#[path = "uv_project_relax.rs"]
mod uv_project_relax;
#[path = "uv_seams_symmetry.rs"]
mod uv_seams_symmetry;
#[path = "uv_tools_spec.rs"]
mod uv_tools_spec;
#[path = "uv_transform_pipe.rs"]
mod uv_transform_pipe;
#[path = "vertex_bevel_round.rs"]
mod vertex_bevel_round;
#[path = "vertex_map_falloff.rs"]
mod vertex_map_falloff;
#[path = "vertex_map_tools.rs"]
mod vertex_map_tools;
#[path = "vertex_merge.rs"]
mod vertex_merge;
#[path = "vertex_normals.rs"]
mod vertex_normals;
#[path = "vertex_tool_options.rs"]
mod vertex_tool_options;
#[path = "vertex_tool_options_clean_spec.rs"]
mod vertex_tool_options_clean_spec;
#[path = "vertex_variation.rs"]
mod vertex_variation;
#[path = "weld_drag.rs"]
mod weld_drag;
#[path = "workflow.rs"]
mod workflow;

/// Test files that build as their own program (`[[test]]` in Cargo.toml)
/// because they count something process-wide, which every other test here
/// would add to: two install a counting global allocator, and `mesh_slot`
/// reads the global count of packed-mesh opens.
const OWN_PROGRAM: &[&str] = &[
    "lxo_allocation_measurement",
    "mesh_slot",
    "packed_mesh_probe",
];

#[test]
fn every_test_file_is_listed() {
    let listed: std::collections::BTreeSet<&str> = include_str!("all.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("mod ")?.strip_suffix(';'))
        .collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        if name != "all" && !OWN_PROGRAM.contains(&name.as_str()) && !listed.contains(name.as_str())
        {
            missing.push(name);
        }
    }
    missing.sort();
    let lines: Vec<String> = missing
        .iter()
        .map(|n| format!("#[path = \"{n}.rs\"]\nmod {n};"))
        .collect();
    assert!(
        missing.is_empty(),
        "these tests/*.rs files are not in tests/all.rs, so they never run; add:\n{}",
        lines.join("\n")
    );
    for name in &listed {
        assert!(
            dir.join(format!("{name}.rs")).exists(),
            "tests/all.rs lists {name}.rs, which is gone"
        );
    }
}

/// Contributor rule (Jason, 2026-11-19): shared registry files take one
/// added line per new file, in sorted order, so parallel branches add
/// lines in different places and merge without clashes.
#[test]
fn test_files_are_listed_in_sorted_order() {
    let listed: Vec<&str> = include_str!("all.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("mod ")?.strip_suffix(';'))
        .collect();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    if let Some((at, (have, want))) = listed
        .iter()
        .zip(&sorted)
        .enumerate()
        .find(|(_, (a, b))| a != b)
    {
        panic!(
            "tests/all.rs: mod lines must stay in sorted order; line {} is `{have}`, \
             expected `{want}` there",
            at + 1
        );
    }
}
