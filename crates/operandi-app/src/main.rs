#[path = "files/about.rs"]
mod about;
#[path = "transforms/absolute_scaling_ui.rs"]
mod absolute_scaling_ui;
#[path = "presets/add_browser.rs"]
mod add_browser;
#[path = "anim/anim_keys.rs"]
mod anim_keys;
#[path = "anim/anim_switch.rs"]
mod anim_switch;
#[path = "anim/anim_ui.rs"]
mod anim_ui;
#[cfg(test)]
#[path = "anim/anim_ui_tests.rs"]
mod anim_ui_tests;
#[cfg(test)]
#[path = "anim/animated_scene_frames_tests.rs"]
mod animated_scene_frames_tests;
#[cfg(test)]
#[path = "anim/channel_panel_tests.rs"]
mod channel_panel_tests;
#[path = "controls/collapse_line.rs"]
mod collapse_line;
#[path = "commands/command_history.rs"]
mod command_history;
#[path = "commands/command_history_ui.rs"]
mod command_history_ui;
#[cfg(test)]
#[path = "create_tools/contour_tests.rs"]
mod contour_tests;
#[path = "items/item_transform_ui.rs"]
mod item_transform_ui;
#[cfg(test)]
#[path = "painting_ui/mesh_paint_tests.rs"]
mod mesh_paint_tests;
#[path = "items/mesh_stack_ui.rs"]
mod mesh_stack_ui;
#[path = "items/meshblend_ui.rs"]
mod meshblend_ui;
#[path = "nodes/node_edit_ui.rs"]
mod node_edit_ui;
#[cfg(test)]
#[path = "nodes/node_edit_ui_tests.rs"]
mod node_edit_ui_tests;
#[path = "nodes/node_material_ui.rs"]
mod node_material_ui;
#[cfg(test)]
#[path = "nodes/node_material_ui_tests.rs"]
mod node_material_ui_tests;
#[cfg(test)]
#[path = "nodes/node_view_close_tests.rs"]
mod node_view_close_tests;
#[path = "nodes/node_view_open.rs"]
mod node_view_open;
#[cfg(test)]
#[path = "nodes/node_view_open_tests.rs"]
mod node_view_open_tests;
#[cfg(test)]
#[path = "nodes/node_view_panel_tests.rs"]
mod node_view_panel_tests;
#[path = "controls/popup_title.rs"]
mod popup_title;
#[path = "view/snaps_panel.rs"]
mod snaps_panel;
#[path = "transforms/symmetrize_panel.rs"]
mod symmetrize_panel;
#[path = "transforms/symmetry_ui.rs"]
mod symmetry_ui;
use crate::prefs_file::PrefsFile;
use snaps_panel::SnapPreset;
#[cfg(test)]
#[path = "view/ac_press_tests.rs"]
mod ac_press_tests;
#[cfg(test)]
#[path = "view/ac_rules_step1_tests.rs"]
mod ac_rules_step1_tests;
#[path = "view/action_center_click.rs"]
mod action_center_click;
#[path = "files/app_environment.rs"]
mod app_environment;
#[path = "menus/app_menu.rs"]
mod app_menu;
#[path = "graphics/backdrop_gpu.rs"]
mod backdrop_gpu;
#[path = "render/backdrop_ui.rs"]
mod backdrop_ui;
#[path = "render/background_ui.rs"]
mod background_ui;
#[path = "polygons/bevel_panel.rs"]
mod bevel_panel;
#[path = "rigging/bind_dialog_ui.rs"]
mod bind_dialog_ui;
#[path = "slicing/boolean_dialog.rs"]
mod boolean_dialog;
#[path = "controls/branding.rs"]
mod branding;
#[path = "edges/bridge_panel.rs"]
mod bridge_panel;
#[path = "files/bug_report.rs"]
mod bug_report;
#[path = "files/bug_report_capture.rs"]
mod bug_report_capture;
#[path = "window_layout/button_variants.rs"]
mod button_variants;
#[path = "view/camera.rs"]
mod camera;
#[path = "render/camera_light_handles.rs"]
mod camera_light_handles;
#[path = "render/camera_light_ui.rs"]
mod camera_light_ui;
#[path = "render/camera_props_ui.rs"]
mod camera_props_ui;
#[path = "anim/channel_links_ui.rs"]
mod channel_links_ui;
#[path = "anim/channel_panel_ui.rs"]
mod channel_panel_ui;
#[path = "anim/channel_row.rs"]
mod channel_row;
#[path = "rigging/character_rig_ui.rs"]
mod character_rig_ui;
#[path = "tools/click_finish.rs"]
mod click_finish;
#[path = "anim/clip_picker.rs"]
mod clip_picker;
#[path = "settings/color_theme.rs"]
mod color_theme;
#[path = "controls/colour_field.rs"]
mod colour_field;
#[path = "controls/colour_picker.rs"]
mod colour_picker;
#[path = "commands/command.rs"]
mod command;
#[cfg(test)]
#[path = "commands/command_contract_tests.rs"]
mod command_contract_tests;
#[path = "commands/command_handlers.rs"]
mod command_handlers;
#[cfg(test)]
#[path = "commands/command_registry_gaps.rs"]
mod command_registry_gaps;
#[cfg(test)]
#[path = "commands/command_registry_tests.rs"]
mod command_registry_tests;
#[path = "create_tools/cone_panel.rs"]
mod cone_panel;
#[path = "create_tools/contour_ui.rs"]
mod contour_ui;
#[path = "items/create_utility.rs"]
mod create_utility;
#[cfg(test)]
#[path = "tools/ctrl_constrain_tests.rs"]
mod ctrl_constrain_tests;
#[path = "view/cursor_wrap.rs"]
mod cursor_wrap;
#[path = "create_tools/curve_hands_on.rs"]
mod curve_hands_on;
#[path = "create_tools/curve_tab.rs"]
mod curve_tab;
#[path = "slicing/cut_dialogs.rs"]
mod cut_dialogs;
#[path = "settings/defaults_preferences.rs"]
mod defaults_preferences;
#[path = "deformers/deform_panel.rs"]
mod deform_panel;
#[path = "controls/detents.rs"]
mod detents;
#[path = "polygons/detriangulate_dialog.rs"]
mod detriangulate_dialog;
#[path = "window_layout/dialog_place.rs"]
mod dialog_place;
#[path = "materials/display_shading.rs"]
mod display_shading;
#[cfg(test)]
#[path = "tools/drag_undo_guard_tests.rs"]
mod drag_undo_guard_tests;
#[path = "create_tools/draw_tools.rs"]
mod draw_tools;
#[path = "slicing/drill_dialog.rs"]
mod drill_dialog;
#[path = "tools/drop_action_ui.rs"]
mod drop_action_ui;
#[path = "input_devices/drop_keys.rs"]
mod drop_keys;
#[path = "items/duplicate_item_rows.rs"]
mod duplicate_item_rows;
#[path = "duplication/duplicate_panel.rs"]
mod duplicate_panel;
#[path = "duplication/duplicate_transform_panel.rs"]
mod duplicate_transform_panel;
#[path = "view/duplicate_view.rs"]
mod duplicate_view;
#[path = "edges/edge_panel.rs"]
mod edge_panel;
#[path = "polygons/edit_panel.rs"]
mod edit_panel;
#[path = "materials/editor_color_ui.rs"]
mod editor_color_ui;
#[path = "create_tools/ellipsoid_panel.rs"]
mod ellipsoid_panel;
#[path = "tools/empty_expand.rs"]
mod empty_expand;
#[path = "graphics/environment_gpu.rs"]
mod environment_gpu;
#[path = "render/environment_preset_ui.rs"]
mod environment_preset_ui;
#[path = "render/environment_ui.rs"]
mod environment_ui;
#[path = "render/exr_multilayer.rs"]
mod exr_multilayer;
#[path = "deformers/falloff_panel.rs"]
mod falloff_panel;
#[path = "tools/field_handoff.rs"]
mod field_handoff;
#[path = "files/file_chooser.rs"]
mod file_chooser;
#[path = "files/file_commands.rs"]
mod file_commands;
#[path = "settings/file_io_preferences.rs"]
mod file_io_preferences;
#[path = "polygons/flip_it_dialog.rs"]
mod flip_it_dialog;
#[path = "window_layout/form/mod.rs"]
mod form;
#[path = "window_layout/form_app.rs"]
mod form_app;
#[path = "graphics/frame_stats.rs"]
mod frame_stats;
#[path = "graphics/free_memory.rs"]
mod free_memory;
#[path = "graphics/gpu.rs"]
mod gpu;
#[path = "graphics/gpu_arena.rs"]
mod gpu_arena;
#[path = "graphics/gpu_backend.rs"]
mod gpu_backend;
#[path = "graphics/gpu_chunks.rs"]
mod gpu_chunks;
#[path = "graphics/gpu_memory.rs"]
mod gpu_memory;
#[path = "graphics/gpu_scene.rs"]
mod gpu_scene;
#[path = "materials/gradient_ui.rs"]
mod gradient_ui;
#[path = "graphics/grid.rs"]
mod grid;
#[path = "items/groups_ui.rs"]
mod groups_ui;
#[cfg(test)]
#[path = "commands/guide_menus_tests.rs"]
mod guide_menus_tests;
#[cfg(test)]
#[path = "commands/guide_python_tests.rs"]
mod guide_python_tests;
#[cfg(test)]
#[path = "commands/guide_shortcuts_tests.rs"]
mod guide_shortcuts_tests;
#[path = "view/handle_ids.rs"]
mod handle_ids;
#[path = "settings/handle_preferences.rs"]
mod handle_preferences;
#[path = "view/handle_tips.rs"]
mod handle_tips;
#[path = "view/haul_view.rs"]
mod haul_view;
#[path = "menus/help_menu.rs"]
mod help_menu;
#[path = "tools/help_mode.rs"]
mod help_mode;
#[cfg(test)]
#[path = "tools/help_mode_tests.rs"]
mod help_mode_tests;
#[path = "commands/i2_route.rs"]
mod i2_route;
#[path = "controls/icons.rs"]
mod icons;
#[path = "files/image_decode.rs"]
mod image_decode;
#[path = "files/image_viewer.rs"]
mod image_viewer;
#[path = "files/image_viewer_ui.rs"]
mod image_viewer_ui;
#[path = "files/import.rs"]
mod import;
#[path = "files/import_into.rs"]
mod import_into;
#[path = "input_devices/input.rs"]
mod input;
#[path = "settings/input_map_editor.rs"]
mod input_map_editor;
#[path = "files/interchange_ui.rs"]
mod interchange_ui;
#[path = "items/item_common_ui.rs"]
mod item_common_ui;
#[path = "items/item_extras_ui.rs"]
mod item_extras_ui;
#[path = "items/item_list_ui.rs"]
mod item_list_ui;
#[path = "items/items_tree.rs"]
mod items_tree;
#[path = "input_devices/key_toggles.rs"]
mod key_toggles;
#[path = "kits/kit_commands.rs"]
mod kit_commands;
#[path = "selections/lasso.rs"]
mod lasso;
#[path = "window_layout/layout.rs"]
mod layout;
#[path = "window_layout/layout_keys.rs"]
mod layout_keys;
mod layouts;
#[path = "window_layout/layouts_edit.rs"]
mod layouts_edit;
#[path = "render/light_tier2_ui.rs"]
mod light_tier2_ui;
#[path = "render/light_tier3_ui.rs"]
mod light_tier3_ui;
#[path = "tools/link_mode.rs"]
mod link_mode;
#[path = "tools/live_fields.rs"]
mod live_fields;
#[path = "controls/log_ring.rs"]
mod log_ring;
#[path = "render/lpe_rows.rs"]
mod lpe_rows;
#[path = "graphics/matcap.rs"]
mod matcap;
#[path = "graphics/material_gpu.rs"]
mod material_gpu;
#[cfg(test)]
#[path = "graphics/material_gpu_tests.rs"]
mod material_gpu_tests;
#[path = "materials/material_layout.rs"]
mod material_layout;
#[path = "commands/mcp.rs"]
mod mcp;
#[path = "commands/mcp_pointer.rs"]
mod mcp_pointer;
#[cfg(test)]
#[path = "commands/mcp_pointer_tests.rs"]
mod mcp_pointer_tests;
#[path = "menus/menu_forms.rs"]
mod menu_forms;
#[path = "polygons/mesh_cleanup_dialog.rs"]
mod mesh_cleanup_dialog;
#[path = "settings/mesh_items_preferences.rs"]
mod mesh_items_preferences;
#[path = "painting_ui/mesh_paint_ui.rs"]
mod mesh_paint_ui;
#[path = "tools/mini_properties.rs"]
mod mini_properties;
#[path = "menus/modeling_menu.rs"]
mod modeling_menu;
#[cfg(test)]
#[path = "commands/monkey_tests.rs"]
mod monkey_tests;
#[cfg(test)]
#[path = "input_devices/mouse_rule_tests.rs"]
mod mouse_rule_tests;
#[cfg(test)]
#[path = "selections/multi_active_tests.rs"]
mod multi_active_tests;
#[path = "sculpting/multires_ui.rs"]
mod multires_ui;
#[cfg(test)]
#[path = "sculpting/multires_ui_tests.rs"]
mod multires_ui_tests;
#[path = "input_devices/native.rs"]
mod native;
#[path = "menus/native_menu.rs"]
mod native_menu;
#[path = "settings/navigation_preferences.rs"]
mod navigation_preferences;
#[path = "tools/numeric_draft.rs"]
mod numeric_draft;
#[path = "controls/numeric_tab.rs"]
mod numeric_tab;
#[path = "presets/online_library.rs"]
mod online_library;
#[path = "presets/online_library_ui.rs"]
mod online_library_ui;
#[path = "files/open_documents.rs"]
mod open_documents;
#[path = "window_layout/option_row.rs"]
mod option_row;
#[path = "view/orbit_selected.rs"]
mod orbit_selected;
#[path = "controls/original_compatible.rs"]
mod original_compatible;
#[path = "controls/os_color.rs"]
mod os_color;
#[path = "controls/os_mouse.rs"]
mod os_mouse;
#[path = "graphics/outline.rs"]
mod outline;
#[path = "window_layout/panel_close.rs"]
mod panel_close;
#[path = "window_layout/panel_dot.rs"]
mod panel_dot;
#[cfg(test)]
#[path = "window_layout/panel_dot_tests.rs"]
mod panel_dot_tests;
#[path = "window_layout/panel_maximize.rs"]
mod panel_maximize;
#[cfg(test)]
#[path = "window_layout/panel_maximize_tests.rs"]
mod panel_maximize_tests;
#[path = "window_layout/panel_registry.rs"]
mod panel_registry;
#[path = "window_layout/panels.rs"]
mod panels;
#[cfg(test)]
#[path = "create_tools/patch_safety_tests.rs"]
mod patch_safety_tests;
#[path = "slicing/path_slice_panel.rs"]
mod path_slice_panel;
#[path = "view/path_slice_view.rs"]
mod path_slice_view;
#[path = "slicing/path_tool_panel.rs"]
mod path_tool_panel;
#[path = "view/path_tool_view.rs"]
mod path_tool_view;
#[path = "create_tools/pen_interactive_ui.rs"]
mod pen_interactive_ui;
#[path = "view/picking.rs"]
mod picking;
#[path = "menus/pie_forms.rs"]
mod pie_forms;
#[path = "menus/pie_menu.rs"]
mod pie_menu;
#[path = "uv/planar_texture_ui.rs"]
mod planar_texture_ui;
#[cfg(test)]
#[path = "view/plane_handles_tests.rs"]
mod plane_handles_tests;
#[cfg(test)]
#[path = "create_tools/plane_teapot_tests.rs"]
mod plane_teapot_tests;
#[path = "tools/poly_haul_drag.rs"]
mod poly_haul_drag;
#[path = "polygons/polygon_align.rs"]
mod polygon_align;
#[path = "materials/polygon_material_dialog.rs"]
mod polygon_material_dialog;
#[path = "polygons/polygon_part_dialog.rs"]
mod polygon_part_dialog;
#[path = "polygons/polygon_unify_dialog.rs"]
mod polygon_unify_dialog;
#[path = "rigging/pose_tool_ui.rs"]
mod pose_tool_ui;
#[path = "settings/preferences.rs"]
mod preferences;
#[cfg(test)]
#[path = "settings/preferences_tier1_tests.rs"]
mod preferences_tier1_tests;
#[cfg(test)]
#[path = "settings/preferences_tier2_tests.rs"]
mod preferences_tier2_tests;
#[cfg(test)]
#[path = "settings/prefs_checkbox_tests.rs"]
mod prefs_checkbox_tests;
#[path = "settings/prefs_commands.rs"]
mod prefs_commands;
#[path = "settings/prefs_file.rs"]
mod prefs_file;
#[path = "create_tools/primitive_input.rs"]
mod primitive_input;
#[path = "presets/profile_browser.rs"]
mod profile_browser;
#[path = "files/project_files.rs"]
mod project_files;
#[path = "presets/project_presets.rs"]
mod project_presets;
#[path = "files/project_ui.rs"]
mod project_ui;
#[path = "commands/python_switch.rs"]
mod python_switch;
#[path = "commands/python_ui.rs"]
mod python_ui;
#[cfg(test)]
#[path = "commands/python_ui_tests.rs"]
mod python_ui_tests;
#[cfg(feature = "qa")]
#[path = "commands/qa.rs"]
mod qa;
#[path = "controls/range_labels.rs"]
mod range_labels;
#[path = "polygons/reduction_panel.rs"]
mod reduction_panel;
#[path = "polygons/relax_mesh_dialog.rs"]
mod relax_mesh_dialog;
#[path = "sculpting/remesh_dialog.rs"]
mod remesh_dialog;
#[path = "render/render_item_ui.rs"]
mod render_item_ui;
#[path = "settings/render_preferences.rs"]
mod render_preferences;
#[path = "controls/rendered_icons.rs"]
mod rendered_icons;
#[path = "items/replica_rows.rs"]
mod replica_rows;
#[cfg(test)]
#[path = "items/replica_safety_tests.rs"]
mod replica_safety_tests;
#[path = "items/replicator_ui.rs"]
mod replicator_ui;
#[path = "files/report_bug_dialog.rs"]
mod report_bug_dialog;
#[path = "files/resource_write.rs"]
mod resource_write;
#[cfg(test)]
#[path = "rigging/rig_tab_tests.rs"]
mod rig_tab_tests;
#[cfg(test)]
#[path = "rigging/rig_ui_tests.rs"]
mod rig_ui_tests;
#[path = "rigging/rig_user_channels_ui.rs"]
mod rig_user_channels_ui;
#[path = "transforms/scale_offset_dialog.rs"]
mod scale_offset_dialog;
#[path = "files/scenes.rs"]
mod scenes;
#[path = "rigging/schematic_ui.rs"]
mod schematic_ui;
#[path = "controls/scrub.rs"]
mod scrub;
#[path = "sculpting/sculpt_ui.rs"]
mod sculpt_ui;
#[cfg(test)]
#[path = "sculpting/sculpt_ui_tests.rs"]
mod sculpt_ui_tests;
#[path = "selections/select_panel.rs"]
mod select_panel;
#[path = "selections/select_tools_ui.rs"]
mod select_tools_ui;
#[path = "view/selection_normals.rs"]
mod selection_normals;
#[path = "settings/selection_preferences.rs"]
mod selection_preferences;
#[path = "selections/selection_set_dialog.rs"]
mod selection_set_dialog;
#[path = "graphics/session_draw.rs"]
mod session_draw;
#[cfg(test)]
#[path = "graphics/session_draw_tests.rs"]
mod session_draw_tests;
#[path = "graphics/session_handles.rs"]
mod session_handles;
#[cfg(test)]
#[path = "graphics/session_handles_tests.rs"]
mod session_handles_tests;
#[path = "transforms/set_position_dialog.rs"]
mod set_position_dialog;
#[path = "materials/shading_panel.rs"]
mod shading_panel;
#[path = "create_tools/shape_panel.rs"]
mod shape_panel;
#[cfg(test)]
#[path = "input_devices/shifted_key_tests.rs"]
mod shifted_key_tests;
#[path = "window_layout/side_tabs.rs"]
mod side_tabs;
#[cfg(test)]
#[path = "rigging/skeleton_tool_tests.rs"]
mod skeleton_tool_tests;
#[path = "rigging/skeleton_tool_ui.rs"]
mod skeleton_tool_ui;
#[path = "view/slice_view.rs"]
mod slice_view;
#[cfg(test)]
#[path = "polygons/small_commands_tab_tests.rs"]
mod small_commands_tab_tests;
#[path = "polygons/smooth_shift_panel.rs"]
mod smooth_shift_panel;
#[path = "view/snap_kinds.rs"]
mod snap_kinds;
#[path = "view/snap_service.rs"]
mod snap_service;
#[path = "view/snapping.rs"]
mod snapping;
#[path = "create_tools/solid_sketch_ui.rs"]
mod solid_sketch_ui;
#[path = "input_devices/space_mouse/mod.rs"]
mod space_mouse;
#[path = "polygons/spike_panel.rs"]
mod spike_panel;
#[path = "items/statistics_ui.rs"]
mod statistics_ui;
#[path = "graphics/subd_gpu.rs"]
mod subd_gpu;
#[cfg(test)]
#[path = "graphics/subd_gpu_tests.rs"]
mod subd_gpu_tests;
#[cfg(test)]
#[path = "polygons/subdivide_tab_tests.rs"]
mod subdivide_tab_tests;
#[path = "polygons/subdivide_ui.rs"]
mod subdivide_ui;
#[path = "view/subdivision_view.rs"]
mod subdivision_view;
#[path = "topology_tools/topology_mode.rs"]
mod topology_mode;
#[path = "topology_tools/topology_pen_ui.rs"]
mod topology_pen_ui;
#[path = "topology_tools/topology_tab.rs"]
mod topology_tab;
#[path = "topology_tools/topology_tools_ui.rs"]
mod topology_tools_ui;
#[path = "files/usd_import_dialog.rs"]
mod usd_import_dialog;
#[cfg(all(test, feature = "cycles", feature = "usd"))]
#[path = "render/usd_render_tests.rs"]
mod usd_render_tests;
// Pen-tablet input is read only on macOS; elsewhere the code is unused.
#[path = "render/bake_dialogs.rs"]
mod bake_dialogs;
#[cfg(all(test, not(feature = "painting")))]
#[path = "painting_ui/no_painting_tests.rs"]
mod no_painting_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting.rs"]
mod painting;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_autocreate_tests.rs"]
mod painting_autocreate_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_brush.rs"]
mod painting_brush;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_brush_tests.rs"]
mod painting_brush_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_drive_tests.rs"]
mod painting_drive_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_highlight_tests.rs"]
mod painting_highlight_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_inks.rs"]
mod painting_inks;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_inks_tests.rs"]
mod painting_inks_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_layout.rs"]
mod painting_layout;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_layout_tests.rs"]
mod painting_layout_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_polish_tests.rs"]
mod painting_polish_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_prefs.rs"]
mod painting_prefs;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_prefs_tests.rs"]
mod painting_prefs_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_sculpt.rs"]
mod painting_sculpt;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_sculpt_tests.rs"]
mod painting_sculpt_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_speed_tests.rs"]
mod painting_speed_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_starter_tests.rs"]
mod painting_starter_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_subd_tests.rs"]
mod painting_subd_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_tabs_tests.rs"]
mod painting_tabs_tests;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_tests.rs"]
mod painting_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_uv.rs"]
mod painting_uv;
#[cfg(all(test, feature = "painting"))]
#[path = "painting_ui/painting_uv_tests.rs"]
mod painting_uv_tests;
#[cfg(feature = "painting")]
#[path = "painting_ui/painting_viewer.rs"]
mod painting_viewer;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[path = "input_devices/tablet.rs"]
mod tablet;
#[path = "tools/tag_mode_ui.rs"]
mod tag_mode_ui;
mod theme;
#[path = "tools/tool_exit_hint.rs"]
mod tool_exit_hint;
#[path = "window_layout/tool_form.rs"]
mod tool_form;
#[path = "tools/tool_framing.rs"]
mod tool_framing;
#[path = "window_layout/tool_split.rs"]
mod tool_split;
#[path = "window_layout/tool_tab_cycle.rs"]
mod tool_tab_cycle;
#[path = "window_layout/toolbox.rs"]
mod toolbox;
#[path = "controls/tree_list.rs"]
mod tree_list;
#[path = "controls/typography.rs"]
mod typography;
#[cfg(test)]
#[path = "commands/ui_inventory.rs"]
mod ui_inventory;
#[path = "settings/ui_preferences.rs"]
mod ui_preferences;
#[path = "controls/ui_text.rs"]
mod ui_text;
#[path = "controls/unavailable.rs"]
mod unavailable;
#[path = "settings/undo_autosave.rs"]
mod undo_autosave;
#[path = "settings/units_preferences.rs"]
mod units_preferences;
#[path = "files/update_check.rs"]
mod update_check;
#[path = "files/update_check_ui.rs"]
mod update_check_ui;
#[path = "uv/uv_checker.rs"]
mod uv_checker;
#[path = "uv/uv_dialogs.rs"]
mod uv_dialogs;
#[path = "uv/uv_edit_ui.rs"]
mod uv_edit_ui;
#[path = "uv/uv_editor.rs"]
mod uv_editor;
#[path = "uv/uv_ops_ui.rs"]
mod uv_ops_ui;
#[path = "uv/uv_tool_forms.rs"]
mod uv_tool_forms;
mod uv_view;
#[path = "controls/value_display.rs"]
mod value_display;
#[path = "presets/value_presets.rs"]
mod value_presets;
#[path = "edges/vertex_panel.rs"]
mod vertex_panel;
#[path = "settings/viewport_preferences.rs"]
mod viewport_preferences;
#[path = "tools/weld_drag.rs"]
mod weld_drag;
#[path = "window_layout/window_state.rs"]
mod window_state;
#[path = "window_layout/workspace.rs"]
mod workspace;
use camera::{Camera, View};
use eframe::egui::{self, *};
use glam::DVec3;
use operandi_core::Scene;
use operandi_core::*;
use std::{collections::BTreeMap, path::PathBuf};

/// Bevel: does a handle drag after a released one stack a new bevel (true,
/// the old bevel-drag G2) or keep adjusting the same one (false: Jason's
/// hands-on check, 2026-11-09)? Panel edits always adjust the current bevel;
/// Shift+click or Shift+drag starts a new one either way.
pub(crate) const BEVEL_REDRAG_STACKS: bool = false;

#[derive(Clone, Copy, Debug)]
enum FileAction {
    New,
    Open,
    /// Open restricted to interchange formats (OBJ, STL, glTF, GEO).
    Import,
    SaveAs,
    /// Not a document change: never queued behind the unsaved-work prompt.
    Export(operandi_core::interchange::ExportFormat),
    /// Quit the app (every open scene).
    Close,
    /// Close the scene being edited (File > Close Scene, Cmd+W).
    CloseScene,
}
struct FileDialog {
    action: FileAction,
    path: String,
    overwrite: bool,
}
/// Each mesh's edge list for edge picking, by item.
type EdgeCache = std::collections::HashMap<
    u64,
    (
        operandi_core::Shared<operandi_core::Mesh>,
        std::sync::Arc<Vec<(u32, u32)>>,
    ),
>;
struct Operandi {
    /// Every command and its handler (command_handlers.rs).
    commands: std::rc::Rc<command_handlers::Commands>,
    surface_cache: picking::SurfaceCache,
    /// Decoded Backdrop pictures, by file path (backdrop_ui.rs).
    backdrop_images: backdrop_ui::Images,
    /// Environment pictures decoded for the view's sky and lighting.
    sky_cache: environment_gpu::SkyCache,
    /// A camera/light Properties number being dragged or typed: item, field
    /// key and shown value (camera_light_ui.rs).
    locator_draft: Option<(u64, &'static str, f64)>,
    /// Saved Film Back presets and a "(new preset)" name being typed
    /// (camera_props_ui.rs).
    film_backs: camera_props_ui::FilmBackPresets,
    /// Saved values behind numeric fields' preset arrows (UIC-55).
    value_presets: value_presets::ValuePresets,
    film_back_naming: Option<String>,
    subdivision_cache: subdivision_view::SubdivisionCache,
    /// Item geometry kept on the graphics card (CPU side).
    resident_cache: gpu_scene::Resident,
    /// Each mesh's edge list for edge picking, by item.
    edge_cache: std::cell::RefCell<EdgeCache>,
    /// Tweak's hover pick, reused while the pointer, view and mesh stay put.
    tweak_hover_cache: std::cell::RefCell<Option<viewport::tweak_view::HoverCache>>,
    /// Live tool handle frames, computed once per drag (see `BaseMemo`).
    edge_slide_frame_memo: viewport::SymmetryMemo<Option<(DVec3, DVec3, f64)>>,
    vertex_frame_memo: viewport::SymmetryMemo<Option<(DVec3, DVec3, DVec3, f64)>>,
    edge_frame_memo: viewport::SymmetryMemo<Option<(DVec3, DVec3, DVec3)>>,
    /// The Bevels' gizmo direction: the selection's average normal, once
    /// per drag (`operandi_core::selection_normal`).
    selection_normal_memo: viewport::BaseMemo<(
        (operandi_core::Symmetry, operandi_core::NormalOf),
        Option<DVec3>,
    )>,
    /// Twist Bevel's first tube frame, Group off and on (twist_bevel_handles.rs).
    twist_bevel_frame_memo: viewport::BaseMemo<[Option<operandi_core::TubeFrame>; 2]>,
    /// A twist or Bend Direction ring drag's turn so far, in degrees, so
    /// it carries on past half a circle.
    twist_bevel_turn: Option<f64>,
    /// The base mesh's spanning points for `whole_mesh_drag`, once per drag.
    whole_mesh_basis_memo: viewport::BaseMemo<Option<[usize; 3]>>,
    /// AC-75: the Local handle's axes for the drag's base and context.
    local_handle_memo: viewport::BaseMemo<(
        operandi_core::ActionContext,
        Option<operandi_core::ActionFrame>,
    )>,
    /// Take the resident path without a GPU (benchmarks and tests).
    force_resident: bool,
    environment: app_environment::AppEnvironment,
    bevel_ui: bevel_panel::BevelUi,
    profile_browser: profile_browser::ProfileBrowser,
    import_job: Option<import::ImportJob>,
    import_ready: Option<import::PreparedImport>,
    import_message: Option<String>,
    queued_open: Option<PathBuf>,
    /// Files Finder, the Dock or the command line asked us to open
    /// (open_documents.rs), opened one at a time.
    document_opens: open_documents::Queue,
    editor: Editor,
    /// CS-01 (click-to-start): honour the tools' Auto-Activate flags, so a
    /// tool flagged off arms and waits for a click in a view. On in the
    /// real app; off in the test app, so older tests see tools start at
    /// once (click_start_tests.rs turns it on).
    click_to_start: bool,
    /// Window > MCP Bridge (mcp.rs).
    mcp: mcp::McpState,
    /// File > Render Viewport (Cycles) (render_ui.rs).
    render: render_ui::RenderState,
    /// The Preview window (F8; preview_ui.rs).
    preview: preview_ui::PreviewState,
    camera: Camera,
    /// The pointer's last position, for keys that act at it (NAV-12).
    nav_pointer: Option<eframe::egui::Pos2>,
    /// A turning view (view_nav.rs).
    view_motion: Option<view_nav::Motion>,
    /// LCK-25 Show Locked Mesh Items: locked geometry in its own colour.
    show_locked: bool,
    /// View ▸ Show Falloffs: tint points by falloff weight (FO-06).
    show_falloffs: bool,
    /// FO-05: the next press off the handles draws the geometric falloff
    /// (armed when one is chosen, or by its Draw button).
    falloff_draw_armed: bool,
    /// FO-52: the Lasso falloff's last outline, in view pixels.
    lasso_outline: Vec<glam::DVec2>,
    /// FO-54: the Image falloff's picture, by path, in grey.
    falloff_image: Option<(String, std::sync::Arc<operandi_core::GrayImage>)>,
    /// FO-53: clicks in the view add Path falloff points.
    falloff_path_editing: bool,
    /// The Path points at the press of a point drag.
    path_drag_base: Option<Vec<glam::DVec3>>,
    saved: Scene,
    path: Option<PathBuf>,
    status: String,
    /// "Operandi crashed last time. Report it?" (bug_report.rs).
    crash_prompt: bug_report::CrashPrompt,
    /// Help > Check for Updates and the launch check (update_check.rs).
    update_check: update_check::UpdateCheck,
    /// Online libraries in the preset browser (online_library.rs).
    online: online_library::OnlineLibrary,
    bridge_ui: bridge_panel::BridgeUi,
    fields: BTreeMap<String, String>,
    confirmed_fields: BTreeMap<String, String>,
    field_errors: BTreeMap<String, String>,
    field_focus: Option<Id>,
    /// The focused Tool Properties field takes words, not a number, so
    /// letters typed into it are text, not shortcuts (drop_keys.rs).
    field_focus_words: bool,
    /// The Tool Properties field being drag-scrubbed, if any (scrub.rs).
    scrub: Option<scrub::Scrub>,
    /// UIC-56: each linked field group's link mode, by group ("translation",
    /// "rotation", "scale"); per session.
    link_modes: std::collections::BTreeMap<String, link_mode::LinkMode>,
    /// The axis of the X/Y/Z group field `number` is drawing, if it is one
    /// (set around the call by Tool Properties, MN-06b).
    drawing_link: Option<usize>,
    /// The primary button is down in the viewport: everything from press
    /// to release is one tweak of the active tool (mid-tool undo).
    pointer_held: bool,
    /// MN-02: a tool key held down, and whether it has been used yet.
    key_hold: Option<key_toggles::KeyHold>,
    /// Status when this frame began, and whether `run` dispatched an action in
    /// this frame. Together they identify feedback an action (for example a
    /// Commands-row refusal) produced earlier in the same frame, so a numeric
    /// field's same-frame blur confirmation does not erase it.
    frame_status: String,
    frame_action: bool,
    editor_owned_input: bool,
    menu_owned_input: bool,
    /// The in-window menu bar is drawing or running an entry: its own
    /// popup is open, which mustn't grey its entries (app_menu.rs).
    in_window_bar: bool,
    modal_owned_input: bool,
    layout_owned_input: bool,
    navigation: Option<input::Navigation>,
    /// Free Cursor: the current drag's put-backs.
    cursor_wrap: cursor_wrap::Wrap,
    i2_gesture_ticket: Option<operandi_core::ExtrudeStageTicket>,
    bindings_draft: BTreeMap<String, String>,
    rename_item: Option<u64>,
    mesh_cleanup: Option<mesh_cleanup_dialog::CleanupDialog>,
    cleanup_options: mesh_cleanup_dialog::CleanupOptions,
    set_part: Option<polygon_part_dialog::SetPartDialog>,
    /// The last name Set Part applied this run (original-compatible A6).
    last_set_part: Option<String>,
    selection_set: Option<selection_set_dialog::SelectionSetDialog>,
    set_position: Option<set_position_dialog::SetPositionDialog>,
    scale_offset: Option<scale_offset_dialog::ScaleOffsetDialog>,
    /// Solid Sketch's pick-up question (solid_sketch_ui.rs).
    solid_sketch_ask: Option<solid_sketch_ui::SketchAsk>,
    /// Element or Screen action center waiting for its viewport click.
    /// A Snaps and Precision preset waits for the click that sets its
    /// centre (`snaps_panel.rs`).
    preset_center_click: bool,
    /// A Bevel handle drag was released (used when `bevel_redrag_stacks`).
    bevel_dragged: bool,
    /// Does a handle drag after a released one start a new bevel?
    /// `BEVEL_REDRAG_STACKS`; a field so tests can try both.
    bevel_redrag_stacks: bool,
    set_material: Option<polygon_material_dialog::SetMaterialDialog>,
    align_notice: Option<polygon_align::AlignNotice>,
    /// A failed Save or Open, shown as a dialog (title, reason) until OK:
    /// the status line alone was easy to miss.
    file_error: Option<Box<(String, String)>>,
    unify: Option<polygon_unify_dialog::UnifyDialog>,
    /// The USD import options dialog (docs/BEHAVIOR-USD-IMPORT.md).
    usd_import: Option<usd_import_dialog::UsdImportDialog>,
    flip_it: Option<flip_it_dialog::FlipItDialog>,
    /// Help > Report Bug or Idea…'s capture and dialog (report_bug_dialog.rs).
    report_bug: report_bug_dialog::ReportBug,
    /// The last file shown in Finder by Report Bug (tests read it).
    revealed_path: Option<PathBuf>,
    /// Edit tab Subdivide/UnSubdivide/Freeze dialogs and saved settings.
    subdivide_ui: subdivide_ui::SubdivideUi,
    unify_count: Option<usize>,
    scene_fraction: f32,
    layout_capture: Option<layout::Capture>,
    /// The last press on a side bar's resize line and when: a second one
    /// soon after is a double-click, which collapses that bar.
    last_edge_press: Option<(layout::Divider, f64, Pos2)>,
    scene_divider_rect: Rect,
    scene_focus_tab: Option<panel_registry::PanelKind>,
    visible_scene_lists: BTreeMap<panel_registry::PanelKind, Rect>,
    scene_available_height: f32,
    upper_frame: panel_registry::PanelGroup,
    /// Shading tab state: selected rows, expanded rows, drags (not saved).
    shading: shading_panel::ShadingPanel,
    /// The Properties panel's chosen category tab, and its tab rects (tests).
    properties_tab: String,
    properties_tab_rects: Vec<Rect>,
    lower_frame: panel_registry::PanelGroup,
    scene_rect: Rect,
    scene_rows: BTreeMap<u64, Rect>,
    collapsed_items: std::collections::BTreeSet<u64>,
    /// Skins opened to their joint influences in the Items list (SK-36):
    /// (mesh, op).
    open_skins: std::collections::BTreeSet<(u64, u64)>,
    /// The rig row (piece, skin or influence) last clicked in the Items
    /// list.
    rig_row: Option<items_tree::ItemKey>,
    /// The Items list's scene root row: collapsed, and where it was drawn.
    scene_collapsed: bool,
    scene_root_rect: Option<Rect>,
    show_plane: bool,
    show_diagnostics: bool,
    /// Help > About Operandi (about.rs).
    show_about: bool,
    /// Text a command wants on the clipboard; the next frame copies it
    /// (commands run without a `Context`, help_menu.rs).
    copy_next: Option<String>,
    /// The last link handed to the browser (help_menu.rs); tests read it.
    opened_url: Option<String>,
    /// F1 / Help > What's This? (help_mode.rs).
    help_mode: help_mode::HelpMode,
    /// Window > Frame Time readout in the viewport.
    show_frame_time: bool,
    /// The Alt+Shift+wheel field of view readout: which view, and until
    /// when (egui time) it shows (view_fov.rs).
    fov_readout: Option<(u32, f64)>,
    /// The transform drag's per-axis label (drag_readout.rs).
    drag_readout: Option<drag_readout::DragReadout>,
    /// The main window title last sent to the OS (`sync_window_title`).
    window_title: String,
    frame_stats: frame_stats::FrameStats,
    /// The main window's place, saved on exit (window_state.rs).
    main_window: window_state::Tracker,
    orbit_bench: Option<frame_stats::OrbitBench>,
    drag_bench: Option<frame_stats::DragBench>,
    surface_mini_properties: bool,
    /// CV-03: the Space Mini Properties pop-over, when open.
    mini_properties: Option<mini_properties::MiniProperties>,
    /// Where the next Mini Properties opens (the pointer at the Space tap).
    mini_properties_anchor: Option<Pos2>,
    /// The size Mini Properties' corner was dragged to (UIC-66); kept until
    /// quit, so it opens that size next time.
    mini_properties_size: Option<Vec2>,
    /// N03: where the in-window pop-overs (Mini Properties, Smooth Shift's
    /// mini window) were last drawn. Presses there belong to them, not to
    /// the viewport under them.
    overlay_rects: Vec<Rect>,
    item_fields: BTreeMap<String, String>,
    item_field_owner: Option<(u64, operandi_core::scene::Transform)>,
    prompt_focused: bool,
    tool_properties_open: bool,
    /// Tool Properties' share of the left panel body (`tool_split`).
    tool_split: f32,
    /// The tool the take-over last applied for (`tool_split::activated`).
    tool_split_seen: Option<&'static str>,
    /// The left panel body height last drawn, for divider drags.
    tool_split_total: f32,
    /// The shortest the tool drawer may be: what its tab strip needs.
    tool_list_min: f32,
    /// The tool list / Tool Properties divider's grab rect and layer.
    tool_divider_rect: Rect,
    tool_divider_layer: Option<egui::LayerId>,
    category: toolbox::Category,
    /// The Create panel utility in use (Paste Tool, Ruler, Align…), if any.
    utility: Option<create_utility::Utility>,
    /// Sculpt brushes: the active one, every brush's settings and the
    /// stroke in progress (sculpt_ui.rs). Boxed: the app state is large.
    sculpt: Box<sculpt_ui::SculptState>,
    /// The Create > Draw curve tool in use, if any.
    draw: Option<draw_tools::DrawTool>,
    /// DPM-11: each draw tool's last Mode (Add/Edit/Delete/Insert), and
    /// the Tube's current point (SPEC-DRAW-POINT-MODE).
    point_modes: draw_tools::PointModes,
    /// Create panel groups folded shut, by title.
    create_folded: std::collections::BTreeSet<String>,
    /// The Duplicate tools' source bounds for their handles, remembered
    /// per base mesh (duplicate_view.rs).
    duplicate_cache: std::cell::Cell<Option<duplicate_view::DupBounds>>,
    /// The open Detriangulate dialog (DT-10).
    detriangulate: Option<detriangulate_dialog::DetriangulateDialog>,
    curves_color: Option<curve_tab::CurvesColorDialog>,
    cut_dialog: Option<cut_dialogs::CutDialog>,
    curve_dialog: Option<curve_hands_on::CurveDialog>,
    /// Set Curves Color (CC-30): curves draw in this colour; a session
    /// display setting, never saved in the scene (curve_tab.rs).
    curve_color: Option<[u8; 3]>,
    /// The open Relax Mesh dialog (DP-22).
    relax_mesh: Option<relax_mesh_dialog::RelaxMeshDialog>,
    /// The UV view's Split and Sew brush while it is on (UVT-63).
    uv_brush: Option<uv_ops_ui::UvBrush>,
    /// The open Select tab tool dialog (SL-xx).
    select_dialog: Option<select_tools_ui::SelectDialog>,
    /// Retopo Sketch's stroke so far (world points on the background).
    retopo_stroke: Vec<DVec3>,
    /// Mesh Paint's click or stroke in progress (mesh_paint_ui.rs).
    mesh_paint: mesh_paint_ui::Gesture,
    /// Mesh Paint's settings from its last use this session (#46): the
    /// next start picks them up.
    mesh_paint_memory: Option<operandi_core::mesh_paint::MeshPaintSettings>,
    /// Topology Pen's drag, weld and pre-highlight state.
    pen: topology_pen_ui::PenUi,
    /// Save Environment Preset…'s options dialog (environment_preset_ui.rs).
    env_preset_dialog: Option<environment_preset_ui::EnvPresetDialog>,
    /// The "turn on Global Illumination" hint was shown this session (EP).
    gi_hint_shown: bool,
    /// An environment preset dropped on a 3D view: the Replace / Add menu
    /// at that point (EP §Apply).
    env_drop_menu: Option<(std::path::PathBuf, Pos2)>,
    /// The open Remesh / Automatic Retopology dialog, the remesh running
    /// on its worker, and whether Remesh... last ran Voxel (remesh_dialog.rs).
    remesh: Option<remesh_dialog::RemeshDialog>,
    remesh_job: Option<remesh_dialog::RemeshJob>,
    remesh_last_voxel: bool,
    /// UV Unwrap... or Pack UVs... while open (uv_dialogs.rs).
    uv_dialog: Option<uv_dialogs::UvDialog>,
    /// Atlas Unwrap..., Bake Curvature/Thickness... or Pad Texture to
    /// UVs... while open (bake_dialogs.rs).
    bake_dialog: Option<bake_dialogs::BakeDialog>,
    /// A bake or pad line waiting for its file panel's answer.
    bake_pending: Option<String>,
    /// 3D painting's dialogs, Images window and preferences (painting.rs).
    #[cfg(feature = "painting")]
    painting: painting::Painting,
    /// Views showing UV space instead of 3D (uv_view.rs), by view id.
    uv_views: BTreeMap<u32, uv_view::UvView>,
    /// UV views whose "3D" button was pressed this frame.
    uv_views_to_close: Vec<u32>,
    /// The open Boolean dialog (BOOL-23) and its refusal notice (BOOL-19).
    boolean: Option<boolean_dialog::BooleanDialog>,
    boolean_notice: Option<boolean_dialog::BooleanNotice>,
    /// The open Solid Drill or Axis Drill dialog (DRILL-08).
    drill: Option<drill_dialog::DrillDialog>,
    /// Armed Edge panel click tool (Add Loop / Add Point), if any.
    edge_click: Option<edge_panel::EdgeClick>,
    /// Percent shown beside the pointer while an Add Loop slides.
    edge_insert_label: Option<(Pos2, String)>,
    /// Add Loop and Add Point tool properties (tool audit Tier 1).
    add_loop_settings: edge_panel::InsertSettings,
    add_point_settings: edge_panel::InsertSettings,
    /// Item list row being dragged for parenting.
    /// The Items list's view state (tree_list.rs, items_tree.rs).
    items_tree: Box<tree_list::TreeState<items_tree::ItemKey>>,
    /// An Alt+click "show only this one" on an Items eye, and what it hid.
    item_solo: Option<(u64, Vec<u64>)>,
    /// The Groups list's view state.
    groups_tree: Box<tree_list::TreeState<groups_ui::GroupKey>>,
    /// The UV Maps panel's list state and its picked rows (uv_edit_ui.rs).
    uv_maps_tree: Box<tree_list::TreeState<usize>>,
    uv_maps_picked: Vec<usize>,
    /// Layouts: the layout bar, switching, the UV Editor window
    /// (layouts.rs, uv_editor.rs).
    layouts: layouts::Layouts,
    /// The Mesh Stack tab's list state and its drag gesture (mesh_stack_ui.rs).
    mesh_stack_tree: Box<tree_list::TreeState<mesh_stack_ui::OpKey>>,
    mesh_stack_gesture: Option<u64>,
    /// The Add browsers (AB-01, AB-24): Add Operator, Add Selection and
    /// Add Item, each with its own remembered state and the button it
    /// opened from.
    add_browsers: std::collections::BTreeMap<&'static str, (add_browser::State, egui::Rect)>,
    /// Weld armed (WD-01), and the drag in progress (WD-02..05).
    vertex_weld: bool,
    weld_drag: Option<weld_drag::WeldDragState>,
    /// MV-C16 captured source item and polygons for Sync Index Order.
    sync_source: Option<(u64, [u32; 2])>,
    vertex_dialog: Option<vertex_panel::VertexDialog>,
    /// Construction plane of the current Pen draft (set by its first point).
    pen_plane: Option<(u8, DVec3)>,
    /// The last Symmetry notice shown for the open tool (symmetry_ui.rs).
    symmetry_notice: Option<String>,
    /// The open tool's selection inference line (SI-08), last shown.
    inference_notice: Option<String>,
    /// The Bend notice last shown (BN-45).
    bend_notice: Option<&'static str>,
    toolbox_rects: BTreeMap<String, Rect>,
    /// Toolbox and toolbar forms, shipped and the user's (layouts stage 3,
    /// form/command_form.rs).
    forms: form::command_form::FormStore,
    /// Where each form command button was drawn last frame, by its command
    /// line (tests find buttons by line, toolbox_form.rs).
    form_rects: BTreeMap<String, Rect>,
    /// Workbench edit mode (layouts stage 4, workbench.rs).
    workbench: workbench::State,
    /// Live modifier state for buttons with Shift/Ctrl/Alt variants.
    button_variants: button_variants::State,
    name: String,
    command: String,
    /// The Command History: records, logs and its panel (command_history.rs).
    /// Boxed: the app state is near the stack limit.
    history: Box<command_history::CommandHistory>,
    /// The Command History panel's list states (command_history_ui.rs).
    history_trees: Box<command_history_ui::HistoryTrees>,
    /// Why the line running now failed, when it did (the record's outcome).
    last_failure: Option<String>,
    file_dialog: Option<FileDialog>,
    /// Active project (docs/PROJECT_SYSTEM.md); None works exactly as before.
    project: Option<operandi_core::Project>,
    recent_projects: Vec<PathBuf>,
    project_dialog: Option<project_ui::ProjectDialog>,
    /// Status notes from copying added files into the project this frame
    /// (`project_files.rs`), put after the action's own status line.
    copy_notes: Vec<String>,
    /// Native file panels; None (tests) falls back to typed-path dialogs.
    chooser: Option<file_chooser::Chooser>,
    /// Native panels as sheets (the real app); tests use `chooser` instead.
    native_panels: bool,
    /// File…Help are in the system menu bar even though no AppKit menu is
    /// installed: the offscreen QA harness on macOS sets it so its captures
    /// match the real Mac window (see `menus_in_system_bar`).
    menus_in_system_bar: bool,
    /// A native panel that is open, waiting for the user.
    choosing: Option<file_chooser::Pending>,
    /// Tests: native-style panels held open (file_chooser.rs).
    #[cfg(test)]
    held_panel: Option<file_chooser::HeldAnswer>,
    pending: Option<FileAction>,
    allow_close: bool,
    ground_axes: bool,
    /// Orbit Selected's last pivot (orbit_selected.rs).
    orbit_pivot_memo: Option<Box<orbit_selected::PivotMemo>>,
    viewport_preferences: viewport_preferences::ViewportPreferences,
    input_preferences: preferences::InputPreferences,
    show_preferences: bool,
    /// The Preferences window's page and Colors-page state.
    preferences_ui: preferences::PreferencesUi,
    undo_autosave: undo_autosave::UndoAutosavePreferences,
    selection_preferences: selection_preferences::SelectionPreferences,
    handle_preferences: handle_preferences::HandlePreferences,
    /// Original-compatible A1: the active tool's handle zoom (see
    /// `Camera::handle_zoom`) as `handles` last found it; 1 with A1 off.
    /// Point squares drawn with those handles use it.
    handle_zoom: std::cell::Cell<f32>,
    /// #40: where the live tool's arrows start, in world space, as
    /// `handles` last placed them; axis drags track the pointer along it.
    handle_origin: std::cell::Cell<Option<DVec3>>,
    /// LP-SUN-13 places the user stored (sun-locations.json).
    sun_locations: light_tier3_ui::SunLocations,
    /// The top-bar Drop Action choice (drop_action_ui.rs).
    drop_action: drop_action_ui::DropActionPreferences,
    /// Items list header: Add Item, Select, Filter (item_list_ui.rs).
    item_list: item_list_ui::ItemListUi,
    /// A viewport press on an item that may become a drop.
    drop_drag: Option<drop_action_ui::DropDrag>,
    snapping: snapping::SnappingPreferences,
    /// Mesh Constraint's saved settings (MC-06), its popover's place while
    /// open, pinned (UIC-64) and entered (background_ui.rs).
    mesh_constraint: background_ui::MeshConstraintPreferences,
    /// Vertex Normal Tools (vertex_normal_ui.rs, G9).
    vertex_normal_tools: vertex_normal_ui::VnState,
    /// The Statistics panel (statistics_ui.rs).
    statistics: Box<statistics_ui::StatsState>,
    mesh_constraint_popover: Option<Pos2>,
    mesh_constraint_pinned: bool,
    mesh_constraint_entered: bool,
    /// Snapping Options window, open at this position (SN-04).
    snapping_options: Option<Pos2>,
    /// The Snapping popover is pinned (UIC-64), and the pointer has been
    /// over it since it opened (rolling off closes it only after that).
    snapping_pinned: bool,
    snapping_entered: bool,
    /// N06: Snapping Options was just opened by a click (see PopoverState).
    snapping_opening: bool,
    /// The Snapping popover's Mode dropdown (SN2-11; Global on open).
    snapping_mode: usize,
    /// The Preset row's (New Preset) / (Remove Preset) step under way, and
    /// a preset name being typed for it (SN2-12).
    snapping_preset_step: Option<snapping::PresetStep>,
    snapping_naming: String,
    /// The Absolute Scaling window's fields (session only, spec A3) and
    /// whether it is open.
    absolute: operandi_core::AbsoluteFields,
    absolute_open: bool,
    /// The read-only node graph window (NM-15).
    graph_window: node_material_ui::GraphWindow,
    /// The Snaps and Precision panel is open (snaps_panel.rs).
    snaps_open: bool,
    /// The preset browser (preset_browser.rs), boxed: Operandi is big.
    preset_browser: Box<preset_browser::PresetBrowser>,
    /// The running Snaps and Precision preset and the action context it
    /// replaced.
    snap_preset: Option<(SnapPreset, ActionContext)>,
    /// Planar Rotate Snap's rest direction and axis points.
    planar: viewport::precision_handles::PlanarState,
    /// The snapping key held since it toggled (SN-02).
    snap_hold: Option<snapping::Hold>,
    /// Vertex targets for the current drag (SN-09).
    snap_session: snapping::Session,
    /// The shared point-snap caches every snap queries (`snap_service`).
    snap_service: snap_service::SnapService,
    /// The pre-highlighted target (SN-08) and whether the handle is on it.
    snap_marker: Option<(glam::DVec3, bool)>,
    /// SN2-43b: where the dragged point would be without snapping, the
    /// reticle that hunts for targets.
    snap_reticle: Option<glam::DVec3>,
    units_preferences: units_preferences::UnitsPreferences,
    defaults_preferences: defaults_preferences::DefaultsPreferences,
    mesh_items_preferences: mesh_items_preferences::MeshItemsPreferences,
    /// Graphics memory watch and its stand-ins (gpu_memory.rs).
    gpu_memory: gpu_memory::MemoryWatch,
    ui_preferences: ui_preferences::UiPreferences,
    /// 3D mouse input and Preferences > Input > 3D Mouse (boxed: the
    /// app state is large).
    space_mouse: Box<space_mouse::SpaceMouse>,
    render_preferences: render_preferences::RenderPreferences,
    file_io_preferences: file_io_preferences::FileIoPreferences,
    /// Block System Sleep While Rendering.
    awake: render_preferences::Awake,
    autosave: undo_autosave::Autosave,
    axes: bool,
    plane_widget: bool,
    gpu: String,
    viewport_gpu: Option<gpu::ViewportGpu>,
    last_drawable: [u32; 2],
    viewport_rect: Rect,
    /// The views (views.rs): their layout, each view's camera and rect, the
    /// active one (whose camera is `camera`), and the one being drawn.
    views: operandi_layout::Layout,
    cameras: BTreeMap<u32, Camera>,
    /// The active view's display settings (view_display.rs), swapped like
    /// `camera`; the other views' are in `displays`. Show Verts (MV-C03),
    /// the grid and the shading style are per view.
    display: view_display::ViewDisplay,
    displays: BTreeMap<u32, view_display::ViewDisplay>,
    /// Saved viewport presets (kept with the preferences, not the scene).
    view_presets: view_display::Presets,
    preset_dialog: Option<view_display_ui::PresetDialog>,
    view_rects: BTreeMap<u32, Rect>,
    /// The view header's corner buttons (viewport_buttons.rs): Pan, Rotate,
    /// Zoom, Maximise and the Viewport Properties popover.
    corner: viewport_buttons::CornerState,
    /// MN-08: the docked panel shown alone in its group (`panel_maximize.rs`),
    /// and the panel a Maximise key was pressed over.
    panel_maximized: Option<operandi_layout::Pane>,
    panel_target: Option<operandi_layout::Pane>,
    /// Where the side panels drew last frame, outside the dock tree.
    side_panel_rects: std::collections::BTreeMap<operandi_layout::Pane, Rect>,
    /// Every panel's corner dot (panel_dot.rs) and View > Layout's locks.
    dots: panel_dot::DotState,
    active_view: u32,
    drawing_view: u32,
    /// True while a view other than the active one draws: no input.
    view_passive: bool,
    /// Palettes and the app's floating windows (layouts stage 6,
    /// palettes.rs).
    palettes: palettes::Palettes,
    /// The timeline strip (timeline.rs) and the animation jobs (anim_ui.rs).
    timeline: timeline::Timeline,
    /// The graph editor window (graph_editor.rs).
    graph: graph_editor::GraphEditor,
    /// The Channel Links window (channel_links_ui.rs, RG-50).
    links: channel_links_ui::ChannelLinks,
    /// The Schematic window (schematic_ui.rs, RG-70).
    schematic: schematic_ui::SchematicView,
    /// The Skeleton tool (skeleton_tool_ui.rs, SK-10).
    skeleton: skeleton_tool_ui::SkeletonTool,
    /// The Pose tool (pose_tool_ui.rs, IK-50).
    pose: pose_tool_ui::PoseTool,
    /// The Script Console and the running script (python_ui.rs).
    python: python_ui::PythonConsole,
    anim_ui: anim_ui::AnimUi,
    /// The Form Editor (layouts stage 7, form_editor.rs).
    form_editor: form_editor::State,
    field_rects: BTreeMap<String, Rect>,
    text_editors: std::collections::HashSet<Id>,
    /// Each dialog's last OK'd values (UIC-95, form/).
    dialog_memory: form::Memory,
    /// Counts gestures that write several items as one undo step
    /// (Item Transform over a multi-selection).
    fold_serial: u64,
    recovery_warning: Option<String>,
    journal_blocked: bool,
    /// The keys that fire in the current layout: `global_bindings` with
    /// the layout's own keys on top (layout_keys.rs, LAY-20).
    bindings: BTreeMap<String, String>,
    /// The global input map, as saved in input-map.json.
    global_bindings: BTreeMap<String, String>,
    /// The layout keys `bindings` was last built with.
    layout_keys_applied: BTreeMap<String, String>,
    /// The Input Map's draft of the current layout's keys.
    layout_keys_draft: BTreeMap<String, String>,
    show_bindings: bool,
    input_map_ui: input_map_editor::State,
    lasso_style: lasso::Style,
    lasso_partial: bool,
    lasso_partial_held: bool,
    selection_context_requested: bool,
    /// A Command-press waiting for its release to open the modeling menu.
    modeling_menu_pressed: bool,
    /// Edge mode: time and place of the last ordinary select press, so the
    /// next one can make it a double-click (Select Loop).
    last_component_press: Option<(f64, Pos2)>,
    /// A Shift press that missed every component (empty_expand.rs), so a
    /// second one there makes Expand Selection.
    last_empty_shift_press: Option<(f64, Pos2)>,
    /// Opens the modeling menu this frame (modeling_menu.rs).
    modeling_menu_requested: bool,
    /// The open pie menu (pie_menu.rs), and one asked for by name.
    pie: Option<pie_menu::Open>,
    pie_request: Option<String>,
    /// Export Pie / Import Pie in progress (pie_forms.rs).
    pie_edit: pie_forms::State,
    segment_detent_pixels: f64,
    native_numbers: Vec<native::NumberKey>,
    /// Numeric Enter as Tab: a keypad Enter press turned into Tab whose
    /// release hasn't arrived yet.
    keypad_enter_held: bool,
    /// The egui context, for work outside a frame's UI: applying
    /// preferences (Reset Preferences) and the registry commands that need
    /// the window (menus as forms: Compact / Standard, the MCP bridge,
    /// project presets). Set at the start of every frame and by
    /// `menu_action`.
    frame_ctx: Option<Context>,
    /// Ctrl was held as the current drag began: the constrain key on every
    /// platform, decided at the press for every tool and kept for the whole
    /// drag (HM-01). Rotate snaps to Rotation Snap Angle (HM-20/21), a Move
    /// haul locks to one work-plane axis (HM-10/11), a Scale axis handle
    /// scales uniformly (HM-30).
    handle_constrain: bool,
    /// The Planar locator's host item for World Coordinates off, memoised
    /// per locator and edit (planar_texture_ui.rs).
    planar_host: std::cell::RefCell<Option<(planar_texture_ui::HostKey, Option<u64>)>>,
    /// The locator the Shading panel's layer selection pointed at last
    /// frame: a change selects it (PD-Q02).
    layer_seen: Option<u64>,
    /// The work-plane axis a Ctrl haul with Move/Transform locked to, once
    /// the pointer has travelled far enough to pick it (HM-12).
    haul_axis: Option<DVec3>,
    /// Tab chords bound in the input map, taken out of egui's input so they
    /// don't move widget focus (SDV-02); `keyboard` runs them.
    held_tab: Vec<Event>,
    recovery: Option<Scene>,
    /// Journals of further scenes open at the crash, restored with
    /// `recovery` into their own tabs.
    recovery_more: Vec<Scene>,
    /// The journal files `recovery` and `recovery_more` were read from.
    /// Recover and Discard delete only these; a journal that couldn't be
    /// read is never deleted (Astra P7-002).
    recovery_paths: Vec<PathBuf>,
    /// The open scenes (scenes.rs); the edited one's state is in the fields
    /// above.
    scenes: scenes::OpenScenes,
    journaled: Option<Scene>,
    /// A recovery journal write running in the background.
    journal_job: Option<std::thread::JoinHandle<operandi_core::Result<Scene>>>,
    /// When the recovery journal last wrote (see `journal_when_due`).
    journal_written: Option<std::time::Instant>,
    /// A failed recovery copy or auto-save showed its dialog; later
    /// failures use the status line only, until one succeeds.
    journal_error_shown: bool,
    autosave_error_shown: bool,
}
/// Shortest gap between two crash-recovery journal writes, for every scene
/// (Jason, 2026-11-09: 15 s; was 1 s, which meant ~800 MB a second while
/// editing a 5-million-polygon scene).
const JOURNAL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);
impl Operandi {
    /// The per-frame journal step. Each write forces the file to disk, which
    /// takes ~100 ms on macOS; writing every frame of a drag made dragging
    /// crawl (Jason, 2026-11-01). So: never mid-gesture (the result is
    /// written once it ends) and at most once per JOURNAL_INTERVAL, with a
    /// wake-up so the last change is still written when input stops.
    fn journal_when_due(&mut self, ctx: &Context) {
        // Collect the completion before computing what still needs saving.
        if let Some(job) = self.journal_job.as_ref() {
            if !job.is_finished() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
                return;
            }
            self.finish_journal();
        }
        let pending = !self.allow_close
            && self.dirty()
            && !self
                .journaled
                .as_ref()
                .is_some_and(|j| same_document(j, &self.editor.scene));
        if pending {
            if self.navigation.is_some() {
                return;
            }
            if let Some(last) = self.journal_written {
                let since = last.elapsed();
                if since < JOURNAL_INTERVAL {
                    ctx.request_repaint_after(JOURNAL_INTERVAL - since);
                    return;
                }
            }
        }
        if pending
            && self.recovery.is_none()
            && self.recovery_warning.is_none()
            && !self.journal_blocked
        {
            // Encoding and syncing a large scene takes long enough to stall
            // a frame (a visible hitch on every selection change of a big
            // imported model), so the write runs off the frame. The scene
            // snapshot shares its meshes, so taking it is cheap.
            let scene = self.editor.scene.clone();
            let path = self.journal_path();
            self.journal_job = Some(std::thread::spawn(move || {
                persistence::save(&path, &scene).map(|()| scene)
            }));
            self.journal_written = Some(std::time::Instant::now());
            return;
        }
        let before = self.journaled.clone();
        self.journal();
        if self.journaled != before && self.journaled.is_some() {
            self.journal_written = Some(std::time::Instant::now());
        }
    }
    fn finish_journal(&mut self) {
        if let Some(job) = self.journal_job.take() {
            let result = job
                .join()
                .unwrap_or_else(|_| Err("the write stopped unexpectedly".into()));
            let result = result.map(|scene| self.journaled = Some(scene));
            self.journal_result(result);
        }
    }
    /// A recovery copy write finished. A failure shows a dialog the first
    /// time, then only the status line until a write succeeds.
    fn journal_result(&mut self, result: operandi_core::Result<()>) {
        match result {
            Ok(()) => self.journal_error_shown = false,
            Err(e) => {
                self.status = format!("Recovery journal unavailable: {e}");
                if !std::mem::replace(&mut self.journal_error_shown, true) {
                    self.report_file_error(
                        "Crash Recovery",
                        format!(
                            "The crash-recovery copy of this scene couldn't be written, so \
                             unsaved changes may be lost if Operandi quits unexpectedly. \
                             Save the scene to keep your work.\n\n{e}\n\n\
                             This won't be shown again until a copy is written."
                        ),
                    );
                }
            }
        }
    }
    fn journal(&mut self) {
        // Quitting: the scenes were saved or discarded on purpose.
        if self.allow_close {
            return;
        }
        if self.recovery.is_some() || self.recovery_warning.is_some() || self.journal_blocked {
            return;
        }
        if self.dirty() {
            if !self
                .journaled
                .as_ref()
                .is_some_and(|j| same_document(j, &self.editor.scene))
            {
                let result = persistence::save(&self.journal_path(), &self.editor.scene);
                if result.is_ok() {
                    self.journaled = Some(self.editor.scene.clone());
                }
                self.journal_result(result);
            }
        } else {
            let _ = std::fs::remove_file(self.journal_path());
            self.journaled = None;
        }
    }
    fn new(cc: &eframe::CreationContext<'_>, environment: app_environment::AppEnvironment) -> Self {
        // The user's colours first, so the very first frame draws with them.
        let scheme = color_theme::load(&environment.paths().color_theme());
        theme::install(&cc.egui_ctx);
        // Interface Scale (Preferences > Defaults > UI) is the one way to
        // zoom the interface; egui's own Cmd/Ctrl +/- zoom would fight it
        // every frame and isn't saved.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        native::install(cc.egui_ctx.clone());
        open_documents::connect(cc.egui_ctx.clone());
        if let Some(s) = &cc.wgpu_render_state {
            gpu_backend::watch_device_lost(&s.device);
        }
        let gpu = cc
            .wgpu_render_state
            .as_ref()
            .map(|s| {
                format!(
                    "{:?} · {}",
                    s.adapter.get_info().backend,
                    s.adapter.get_info().name
                )
            })
            .unwrap_or_else(|| "GPU unavailable".into());
        let bindings = input::load_bindings(&environment.paths().input_map());
        let mut app = Self::with_environment(environment);
        app.click_to_start = true;
        app.native_panels = true;
        bug_report::set_gpu(&gpu);
        app.crash_prompt =
            bug_report::CrashPrompt::at_launch(&app.environment.paths().crashes(), &gpu);
        app.gpu = gpu;
        app.global_bindings = bindings.clone();
        app.bindings = bindings;
        app.viewport_gpu = cc.wgpu_render_state.clone().map(gpu::ViewportGpu::new);
        if mcp::on_at_launch() {
            app.set_mcp(&cc.egui_ctx, true);
        }
        let _ = std::fs::create_dir_all(app.environment.paths().data_root());
        app.load_kits();
        app.first_run_tip();
        app.viewport_preferences = viewport_preferences::ViewportPreferences::load(
            &app.environment.paths().viewport_display(),
        );
        app.input_preferences =
            preferences::InputPreferences::load(&app.environment.paths().input_preferences());
        app.input_preferences.options.apply_style(&cc.egui_ctx);
        dialog_place::set_under_mouse(app.input_preferences.options.dialogs_under_mouse);
        app.selection_preferences = selection_preferences::SelectionPreferences::load(
            &app.environment.paths().selection_preferences(),
        );
        app.apply_launch_selection();
        app.handle_preferences = handle_preferences::HandlePreferences::load(
            &app.environment.paths().handle_preferences(),
        );
        app.sun_locations =
            light_tier3_ui::SunLocations::load(&app.environment.paths().sun_locations());
        app.drop_action =
            drop_action_ui::DropActionPreferences::load(&app.environment.paths().drop_action());
        app.item_list.prefs =
            item_list_ui::ItemListPreferences::load(&app.environment.paths().item_list());
        app.snapping =
            snapping::SnappingPreferences::load(&app.environment.paths().snapping_preferences());
        app.preview.saved = preview::Saved::load(&app.environment.paths().preview_window());
        app.load_subdivide_settings();
        app.load_sculpt_settings();
        // QA builds only: OPERANDI_QA_PREVIEW=1 opens the Preview and starts
        // it, for checking a build without pressing keys.
        if !mcp::packaged_release() && std::env::var_os("OPERANDI_QA_PREVIEW").is_some() {
            app.preview.open = true;
            app.preview.autostart = true;
        }
        app.dialog_memory = form::Memory::load(&app.environment.paths().dialog_values());
        // Panel sections' folds (UIC-107), shared by every panel.
        form::rows::set_folds(app.dialog_memory.panel(form::rows::FOLDS).folded);
        app.units_preferences =
            units_preferences::UnitsPreferences::load(&app.environment.paths().units_preferences());
        app.units_preferences.apply();
        app.defaults_preferences = defaults_preferences::DefaultsPreferences::load(
            &app.environment.paths().defaults_preferences(),
        );
        app.file_io_preferences = file_io_preferences::FileIoPreferences::load(
            &app.environment.paths().file_io_preferences(),
        );
        app.file_io_preferences.apply();
        app.film_backs =
            camera_props_ui::FilmBackPresets::load(&app.environment.paths().film_back_presets());
        app.value_presets =
            value_presets::ValuePresets::load(&app.environment.paths().value_presets());
        app.render_preferences = render_preferences::RenderPreferences::load(
            &app.environment.paths().render_preferences(),
        );
        // The Render window's history, preferences and place (IV-12, UIC-73).
        app.render.prefs = image_viewer::ViewerPreferences::load(
            &app.environment.paths().image_viewer_preferences(),
        );
        app.render.viewer.saved =
            image_viewer::WindowSaved::load(&app.environment.paths().render_window());
        app.load_render_history();
        app.ui_preferences =
            ui_preferences::UiPreferences::load(&app.environment.paths().ui_preferences());
        app.ui_preferences.apply();
        app.space_mouse.prefs = space_mouse::SpaceMousePreferences::load(
            &app.environment.paths().space_mouse_preferences(),
        );
        app.mesh_items_preferences = mesh_items_preferences::MeshItemsPreferences::load(
            &app.environment.paths().mesh_items_preferences(),
        );
        // Names, Backdrop sizing and the lens new views start with; then
        // the first scene again, now that the preferences are known
        // (Default Scene, Item Drawing size), and the launch camera at the
        // preferred Flatness of Perspective.
        app.apply_user_defaults();
        app.camera
            .reflatten(camera::flatness_scale(app.viewport_preferences.flatness));
        app.initialize_new_document();
        app.launch_orbit_bench();
        app.launch_drag_bench();
        app.apply_launch_selection();
        app.apply_tool_defaults();
        app.preferences_ui.scheme = scheme;
        app.undo_autosave =
            undo_autosave::UndoAutosavePreferences::load(&app.environment.paths().undo_autosave());
        app.editor.rig_compensation = app.undo_autosave.rig_compensation;
        python_switch::set_preference(app.undo_autosave.python_preview);
        app.update_check.prefs =
            update_check::UpdatePreferences::load(&app.environment.paths().update_preferences());
        app.online.prefs =
            online_library::OnlinePreferences::load(&app.environment.paths().online_preferences());
        app.view_presets = view_display::Presets::load(&app.environment.paths().viewport_presets());
        // Layouts (layouts.rs): the one open at quit, or the old layout
        // files read once as the Model layout.
        // The old app-wide locks seed each layout's own (LAY-48).
        app.dots.locks = panel_dot::LayoutLocks::load(&app.environment.paths().layout_locks());
        app.start_layouts();
        app.load_view_properties();
        app.restore_snapping_popover();
        app.load_mesh_constraint();
        app.load_palettes();
        app.load_form_groups();
        app.load_preset_browser();
        app.cleanup_options = mesh_cleanup_dialog::CleanupOptions::load(
            &app.environment
                .paths()
                .data_root()
                .join("mesh-cleanup.json"),
        );
        app.load_bevel_recipes();
        app.profile_browser =
            profile_browser::ProfileBrowser::load(&app.environment.paths().profile_browser());
        app.finish_layout_migration(&cc.egui_ctx);
        app.load_projects();
        app.read_recovery();
        app.queue_startup_documents();
        native_menu::install(cc.egui_ctx.clone(), app.menu_model(&cc.egui_ctx));
        #[cfg(target_os = "macos")]
        if !native_menu::installed() {
            app.status = "Native menu unavailable; using window menus".into();
        }
        app
    }
    fn queue_startup_documents(&mut self) {
        self.document_opens
            .extend(self.environment.document_arguments());
    }

    // Startup arguments and Finder opens share the File > Open route, one per frame.
    fn open_next_document(&mut self) {
        let busy = self.has_modal() || self.queued_open.is_some();
        if let Some(path) = self.document_opens.next(busy) {
            self.request_open_path(path);
        }
    }

    #[cfg(any(feature = "qa", test))]
    fn open_qa_fixture(&mut self) -> Result<()> {
        let fixture = self
            .environment
            .document_argument()
            .ok_or("QA needs original native fixture path")?;
        self.open(fixture);
        Ok(())
    }

    fn with_environment(environment: app_environment::AppEnvironment) -> Self {
        // File > Export ▸ USD, scene.saveAs and the MCP bridge reach the
        // USD writer through interchange (operandi-usd links OpenUSD only
        // with the `usd` feature).
        operandi_core::interchange::set_usd_exporter(operandi_usd::export);
        operandi_core::interchange::set_usd_importer(operandi_usd::import);
        Self {
            commands: command_handlers::registry(),
            surface_cache: Default::default(),
            backdrop_images: Default::default(),
            sky_cache: Default::default(),
            locator_draft: None,
            film_backs: Default::default(),
            value_presets: Default::default(),
            film_back_naming: None,
            subdivision_cache: Default::default(),
            resident_cache: Default::default(),
            edge_cache: Default::default(),
            tweak_hover_cache: Default::default(),
            edge_slide_frame_memo: Default::default(),
            vertex_frame_memo: Default::default(),
            edge_frame_memo: Default::default(),
            selection_normal_memo: Default::default(),
            twist_bevel_frame_memo: Default::default(),
            twist_bevel_turn: None,
            whole_mesh_basis_memo: Default::default(),
            local_handle_memo: Default::default(),
            force_resident: false,
            online: online_library::OnlineLibrary::new(environment.paths().online_libraries()),
            environment,
            import_job: None,
            import_ready: None,
            import_message: None,
            queued_open: None,
            document_opens: open_documents::Queue::default(),
            editor: Editor::default(),
            click_to_start: false,
            mcp: Default::default(),
            render: Default::default(),
            preview: Default::default(),
            camera: Camera::default(),
            nav_pointer: None,
            view_motion: None,
            show_locked: true,
            show_falloffs: true,
            falloff_draw_armed: false,
            lasso_outline: Vec::new(),
            falloff_image: None,
            falloff_path_editing: false,
            path_drag_base: None,
            saved: Scene::default(),
            path: None,
            status: "Ready · New Mesh to begin".into(),
            bridge_ui: Default::default(),
            fields: BTreeMap::new(),
            confirmed_fields: BTreeMap::new(),
            field_errors: BTreeMap::new(),
            field_focus: None,
            field_focus_words: false,
            scrub: None,
            link_modes: Default::default(),
            drawing_link: None,
            pointer_held: false,
            key_hold: None,
            frame_status: String::new(),
            crash_prompt: Default::default(),
            update_check: Default::default(),
            frame_action: false,
            editor_owned_input: false,
            menu_owned_input: false,
            in_window_bar: false,
            modal_owned_input: false,
            layout_owned_input: false,
            navigation: None,
            cursor_wrap: Default::default(),
            i2_gesture_ticket: None,
            bindings_draft: BTreeMap::new(),
            rename_item: None,
            mesh_cleanup: None,
            cleanup_options: Default::default(),
            set_part: None,
            last_set_part: None,
            selection_set: None,
            set_position: None,
            scale_offset: None,
            solid_sketch_ask: None,
            preset_center_click: false,
            bevel_dragged: false,
            bevel_redrag_stacks: BEVEL_REDRAG_STACKS,
            set_material: None,
            align_notice: None,
            file_error: None,
            unify: None,
            usd_import: None,
            flip_it: None,
            report_bug: Default::default(),
            revealed_path: None,
            subdivide_ui: Default::default(),
            unify_count: None,
            scene_fraction: 0.5,
            layout_capture: None,
            last_edge_press: None,
            scene_divider_rect: Rect::NOTHING,
            scene_focus_tab: None,
            visible_scene_lists: BTreeMap::new(),
            scene_available_height: 1.,
            upper_frame: panel_registry::PanelGroup::new(
                "upper-sidebar-tabs",
                &[
                    panel_registry::PanelKind::Items,
                    panel_registry::PanelKind::ShaderTree,
                    panel_registry::PanelKind::Groups,
                    // Last, so the other tabs keep their places (MO-10
                    // puts it beside the Items list; provisional).
                    panel_registry::PanelKind::MeshStack,
                ],
            ),
            shading: Default::default(),
            properties_tab: String::new(),
            properties_tab_rects: Vec::new(),
            lower_frame: panel_registry::PanelGroup::new(
                "lower-sidebar-tabs",
                &[panel_registry::PanelKind::Properties],
            ),
            scene_rect: Rect::NOTHING,
            scene_rows: BTreeMap::new(),
            collapsed_items: Default::default(),
            open_skins: Default::default(),
            rig_row: None,
            scene_collapsed: false,
            scene_root_rect: None,
            show_plane: false,
            show_diagnostics: false,
            show_about: false,
            copy_next: None,
            opened_url: None,
            help_mode: Default::default(),
            show_frame_time: false,
            fov_readout: None,
            drag_readout: None,
            window_title: String::new(),
            frame_stats: Default::default(),
            main_window: Default::default(),
            orbit_bench: None,
            drag_bench: None,
            surface_mini_properties: false,
            mini_properties: None,
            mini_properties_anchor: None,
            mini_properties_size: None,
            overlay_rects: Vec::new(),
            item_fields: BTreeMap::new(),
            item_field_owner: None,
            prompt_focused: false,
            tool_properties_open: true,
            tool_split: tool_split::DEFAULT_FRACTION,
            tool_split_seen: None,
            tool_split_total: 1.,
            tool_list_min: tool_split::LIST_MIN_HEIGHT,
            tool_divider_rect: Rect::NOTHING,
            tool_divider_layer: None,
            bevel_ui: Default::default(),
            profile_browser: Default::default(),
            category: Default::default(),
            utility: None,
            sculpt: Default::default(),
            draw: None,
            point_modes: Default::default(),
            // Every Create group starts open (Jason, 2026-11-05).
            create_folded: Default::default(),
            duplicate_cache: Default::default(),
            detriangulate: None,
            curves_color: None,
            cut_dialog: None,
            curve_dialog: None,
            curve_color: None,
            relax_mesh: None,
            uv_brush: None,
            select_dialog: None,
            retopo_stroke: Vec::new(),
            mesh_paint: Default::default(),
            mesh_paint_memory: None,
            pen: Default::default(),
            env_preset_dialog: None,
            gi_hint_shown: false,
            env_drop_menu: None,
            remesh: None,
            remesh_job: None,
            remesh_last_voxel: true,
            uv_dialog: None,
            bake_dialog: None,
            bake_pending: None,
            #[cfg(feature = "painting")]
            painting: Default::default(),
            uv_views: BTreeMap::new(),
            uv_views_to_close: Vec::new(),
            boolean: None,
            boolean_notice: None,
            drill: None,
            edge_click: None,
            edge_insert_label: None,
            add_loop_settings: Default::default(),
            add_point_settings: Default::default(),
            items_tree: Default::default(),
            item_solo: None,
            groups_tree: Default::default(),
            uv_maps_tree: Default::default(),
            uv_maps_picked: Vec::new(),
            layouts: Default::default(),
            mesh_stack_tree: Default::default(),
            mesh_stack_gesture: None,
            add_browsers: Default::default(),
            vertex_weld: false,
            weld_drag: None,
            sync_source: None,
            vertex_dialog: None,
            pen_plane: None,
            symmetry_notice: None,
            inference_notice: None,
            bend_notice: None,
            toolbox_rects: BTreeMap::new(),
            forms: toolbox_form::shipped_store(None),
            form_rects: BTreeMap::new(),
            workbench: Default::default(),
            button_variants: Default::default(),
            name: String::new(),
            command: String::new(),
            history: Box::default(),
            history_trees: Box::default(),
            last_failure: None,
            file_dialog: None,
            project: None,
            recent_projects: Vec::new(),
            project_dialog: None,
            copy_notes: Vec::new(),
            chooser: None,
            native_panels: false,
            menus_in_system_bar: false,
            choosing: None,
            #[cfg(test)]
            held_panel: None,
            pending: None,
            allow_close: false,
            ground_axes: true,
            orbit_pivot_memo: None,
            viewport_preferences: Default::default(),
            input_preferences: Default::default(),
            show_preferences: false,
            preferences_ui: Default::default(),
            undo_autosave: Default::default(),
            selection_preferences: Default::default(),
            handle_preferences: Default::default(),
            handle_zoom: std::cell::Cell::new(1.),
            handle_origin: std::cell::Cell::new(None),
            sun_locations: Default::default(),
            drop_action: Default::default(),
            item_list: Default::default(),
            drop_drag: None,
            snapping: Default::default(),
            snapping_options: None,
            snapping_pinned: false,
            snapping_entered: false,
            snapping_opening: false,
            mesh_constraint: Default::default(),
            vertex_normal_tools: Default::default(),
            statistics: Default::default(),
            mesh_constraint_popover: None,
            mesh_constraint_pinned: false,
            mesh_constraint_entered: false,
            snapping_mode: 0,
            snapping_preset_step: None,
            snapping_naming: String::new(),
            absolute: Default::default(),
            absolute_open: false,
            graph_window: Default::default(),
            snaps_open: false,
            preset_browser: Box::new(preset_browser::PresetBrowser::new(
                std::env::temp_dir().join("operandi-thumbs"),
            )),
            snap_preset: None,
            planar: Default::default(),
            snap_hold: None,
            snap_session: Default::default(),
            snap_service: Default::default(),
            snap_marker: None,
            snap_reticle: None,
            units_preferences: Default::default(),
            defaults_preferences: Default::default(),
            mesh_items_preferences: Default::default(),
            gpu_memory: Default::default(),
            ui_preferences: Default::default(),
            space_mouse: Default::default(),
            render_preferences: Default::default(),
            file_io_preferences: Default::default(),
            awake: Default::default(),
            autosave: Default::default(),
            axes: false,
            plane_widget: true,
            gpu: "GPU unavailable".into(),
            viewport_gpu: None,
            last_drawable: [0, 0],
            viewport_rect: Rect::NOTHING,
            views: operandi_layout::Layout::single_viewport(),
            cameras: BTreeMap::new(),
            display: Default::default(),
            displays: BTreeMap::new(),
            view_presets: Default::default(),
            preset_dialog: None,
            view_rects: BTreeMap::new(),
            corner: Default::default(),
            panel_maximized: None,
            panel_target: None,
            side_panel_rects: Default::default(),
            dots: Default::default(),
            active_view: 0,
            drawing_view: 0,
            view_passive: false,
            palettes: Default::default(),
            timeline: Default::default(),
            graph: Default::default(),
            links: Default::default(),
            schematic: Default::default(),
            skeleton: Default::default(),
            pose: Default::default(),
            python: Default::default(),
            anim_ui: Default::default(),
            form_editor: Default::default(),
            field_rects: BTreeMap::new(),
            text_editors: Default::default(),
            dialog_memory: Default::default(),
            fold_serial: 0,
            recovery_warning: None,
            journal_blocked: false,
            bindings: input::default_bindings(),
            global_bindings: input::default_bindings(),
            layout_keys_applied: BTreeMap::new(),
            layout_keys_draft: BTreeMap::new(),
            show_bindings: false,
            input_map_ui: Default::default(),
            lasso_style: lasso::Style::default(),
            lasso_partial: false,
            lasso_partial_held: false,
            selection_context_requested: false,
            modeling_menu_pressed: false,
            last_component_press: None,
            last_empty_shift_press: None,
            modeling_menu_requested: false,
            pie: None,
            pie_request: None,
            pie_edit: Default::default(),
            segment_detent_pixels: 12.,
            native_numbers: Vec::new(),
            keypad_enter_held: false,
            frame_ctx: None,
            handle_constrain: false,
            planar_host: Default::default(),
            layer_seen: None,
            haul_axis: None,
            held_tab: Vec::new(),
            recovery: None,
            recovery_more: Vec::new(),
            recovery_paths: Vec::new(),
            scenes: Default::default(),
            journaled: None,
            journal_written: None,
            journal_error_shown: false,
            autosave_error_shown: false,
            journal_job: None,
        }
    }
    #[cfg(test)]
    fn empty() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../development/stab07b-environment-20261026/test-data");
        std::fs::create_dir_all(&parent).expect("create app test data parent");
        let root = parent.join(format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock after epoch")
                .as_nanos(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("create exclusive app test data root");
        let mut app = Self::empty_at(root.clone());
        app.environment.scratch = Some(std::sync::Arc::new(app_environment::ScratchRoot(root)));
        app
    }
    /// An app on its own data folder (`root` exists), as after launch but
    /// without a window: tests that reopen a folder (layouts_tests.rs).
    #[cfg(test)]
    fn empty_at(root: PathBuf) -> Self {
        Self::with_environment(app_environment::AppEnvironment::from_inputs(
            Some(root.into_os_string()),
            None,
            Vec::new(),
            env!("CARGO_MANIFEST_DIR"),
        ))
    }
    /// Reads every recovery journal on its own (Astra P7-002): each healthy
    /// one is offered, in slot order, and a damaged one neither hides the
    /// others nor gets deleted. A damaged journal is copied aside for
    /// diagnosis first; if that copy fails, recovery writes stay off this
    /// session so nothing overwrites the original.
    fn read_recovery(&mut self) {
        let mut scenes = Vec::new();
        let mut paths = Vec::new();
        let mut problems = Vec::new();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for path in scenes::journal_files(&self.environment) {
            match persistence::load(&path) {
                Ok(scene) => {
                    scenes.push(scene);
                    paths.push(path);
                }
                Err(e) => {
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "recovery".into());
                    let backup = path.with_file_name(format!("{stem}-corrupt-{stamp}.opscene"));
                    match std::fs::copy(&path, &backup) {
                        Ok(_) => problems.push(format!(
                            "{e}. Diagnostic copy preserved at {}",
                            backup.display()
                        )),
                        Err(copy) => {
                            self.journal_blocked = true;
                            problems.push(format!(
                                "{e}; backup failed: {copy}. Original journal is preserved at {}; recovery writes disabled for this session",
                                path.display()
                            ));
                        }
                    }
                }
            }
        }
        self.recovery_warning = (!problems.is_empty()).then(|| problems.join("\n\n"));
        self.recovery_paths = paths;
        let mut scenes = scenes.into_iter();
        self.recovery = scenes.next();
        self.recovery_more = scenes.collect();
    }
    /// Recover Scene, OK: the first journal's scene replaces the new one
    /// and each further one opens in its own tab. The journals that were
    /// read are replaced by the recovered tabs' own; the first is written
    /// at once so the work is never only in memory.
    fn recover_journals(&mut self) {
        let Some(scene) = self.recovery.take() else {
            return;
        };
        let more = std::mem::take(&mut self.recovery_more);
        let first = self.journal_path();
        for path in std::mem::take(&mut self.recovery_paths) {
            if path != first {
                let _ = std::fs::remove_file(path);
            }
        }
        self.editor = Editor {
            scene,
            ..Default::default()
        };
        self.name = self
            .editor
            .scene
            .active()
            .map(|i| i.name.clone())
            .unwrap_or_default();
        self.path = None;
        self.saved = Scene::default();
        self.clear_fields();
        self.status = "Recovered unsaved scene; Save As to preserve it".into();
        self.frame(false);
        self.journal();
        if !more.is_empty() {
            let count = more.len() + 1;
            self.recover_extra_scenes(more);
            self.status = format!("Recovered {count} unsaved scenes; Save As to preserve them");
        }
    }
    /// Recover Scene, Discard: deletes the journals that were read (never
    /// one that couldn't be read).
    fn discard_journals(&mut self) {
        self.recovery = None;
        self.recovery_more.clear();
        for path in std::mem::take(&mut self.recovery_paths) {
            let _ = std::fs::remove_file(path);
        }
    }
    /// Recover Scene, Cancel: keeps every journal for the next launch. No
    /// recovery writes or journal deletions happen this session (UIC-96),
    /// so Close Scene and Quit leave them too (Astra P7-001).
    fn keep_journals(&mut self) {
        self.recovery = None;
        self.recovery_more.clear();
        self.recovery_paths.clear();
        self.journal_blocked = true;
        self.status =
            "Recovery journal kept for the next launch; this session isn't journaled".into();
    }
    fn clear_fields(&mut self) {
        self.bridge_ui = Default::default();
        self.fields.clear();
        self.confirmed_fields.clear();
        self.field_errors.clear();
        self.field_focus = None;
    }
    fn field_label(key: &str) -> String {
        if let Some(label) = match key {
            "bevel_segments" => Some("Segments"),
            "bevel_shift_range" => Some("Shift Range"),
            "bevel_inset_range" => Some("Inset Range"),
            "bevel_angle" => Some("Max. Smoothing Angle"),
            "bevel_weld_range" => Some("Weld Range"),
            "edge_slide_distance" => Some("Distance"),
            "tweak_range" => Some("Range"),
            "vertex_slide_percent" => Some("Percent"),
            "slide_tension" => Some("Tension"),
            "edge_weight" => Some("Weight"),
            _ => None,
        } {
            return label.into();
        }
        if key == "inset" {
            return "Inset".into();
        }
        if key == "shift" {
            return "Shift".into();
        }
        if key == "radial_sides" || key == "extrude_sides" {
            return "Sides".into();
        }
        if key == "radial_segments" {
            return "Segments".into();
        }
        let axis = key.chars().last().and_then(|c| c.to_digit(10)).unwrap_or(0) as usize;
        format!(
            "{} {}",
            if key.starts_with("size") {
                "Size"
            } else if key.starts_with("position") {
                "Position"
            } else if key.starts_with("radius") {
                "Radius"
            } else if key.starts_with("min") {
                "Min"
            } else if key.starts_with("max") {
                "Max"
            } else if key.starts_with("extent") {
                "Extent"
            } else {
                "Transform"
            },
            ["X", "Y", "Z"][axis.min(2)]
        )
    }
    fn confirm_fields(&mut self) -> bool {
        self.confirm_tool_fields()
    }
    /// Feedback produced by an action earlier in this frame, if any. A repeated
    /// identical message still counts, because `run` marks the frame.
    fn same_frame_action_status(&self) -> Option<String> {
        (self.frame_action || self.status != self.frame_status).then(|| self.status.clone())
    }
    /// Whether an editor tool session (live tool, primitive, Bridge, Spike,
    /// Pen or Reduction) is open.
    fn editor_tool_active(&self) -> bool {
        self.editor.has_active_task()
    }

    /// A viewport press began a gesture: everything it changes until the
    /// release is one undo step (mid-tool undo, Jason 2026-11-04/14). The
    /// editor holds the tweak open itself, so a gesture's updates join it
    /// whichever path they take to the editor (`run`, or a direct
    /// `editor.dispatch`); no tool or button sets this on its own.
    fn hold_pointer(&mut self) {
        self.pointer_held = true;
        self.editor.tweak_held = true;
    }

    /// The press is over: the next change starts a new step.
    fn release_pointer(&mut self) {
        self.pointer_held = false;
        self.editor.end_tweak();
    }

    /// Dispatch `c` and report the result in the status bar.
    fn run(&mut self, c: Command) {
        self.run_ok(c);
    }

    /// Like [`Self::run`], and returns true when the command succeeded, false
    /// when it was refused or failed. Use this instead of reading the status
    /// text to find out what happened.
    fn run_ok(&mut self, c: Command) -> bool {
        let ok = self.run_ok_inner(c);
        // A refusal is the running line's outcome (Command History). The
        // click tools' "… off" ends aren't failures.
        if !ok && !self.status.ends_with(" off") {
            self.last_failure = Some(self.status.clone());
        }
        ok
    }
    fn run_ok_inner(&mut self, c: Command) -> bool {
        self.frame_action = true;
        // A Camera view move still settling is the newest step.
        if matches!(c, Command::Undo | Command::Redo) {
            self.flush_camera_view();
        }
        // CS-01: the editor may have been replaced (a new or opened scene),
        // so the switch is passed on with every command.
        self.editor.auto_activate_flags = self.click_to_start;
        // Select Connected in a UV view follows UV islands (UVV-04).
        let c = match c {
            Command::Selection(SelectionOperation::Connected)
                if self.is_uv_view(self.active_view) =>
            {
                // The map the UV view shows (the mesh's selected map).
                Command::SelectUvIslands(
                    self.editor
                        .scene
                        .active()
                        .and_then(|i| operandi_core::uv_maps::current_map(&i.mesh)),
                )
            }
            c => c,
        };
        if self.align_notice.is_some() {
            self.status = "Close Align before using another command or tool".into();
            return false;
        }
        if self.unify.is_some() || self.unify_count.is_some() {
            self.status = "Close Unify before using another command or tool".into();
            return false;
        }
        if self.set_material.is_some() {
            self.status = "Close Set Material before using another command or tool".into();
            return false;
        }
        if self.scale_offset.is_some() {
            self.status = "Close Scale Offset before using another command or tool".into();
            return false;
        }
        // A dropped Pen polygon faces the viewer whichever way it was
        // clicked (a Flip Polygon option would invert that).
        if matches!(c, Command::Drop)
            && let Some(draft) = &self.editor.pen_draft
            && draft.points().len() >= 3
            && draft.options().output == PenType::Polygon
        {
            let points = draft.points();
            let ring = |f: &[u32]| -> DVec3 {
                (0..f.len())
                    .map(|i| points[f[i] as usize].cross(points[f[(i + 1) % f.len()] as usize]))
                    .sum()
            };
            // A Make Quads strip faces the viewer as a whole (provisional,
            // PMQ-Q2): the sum of its polygons' normals.
            let normal: DVec3 = if draft.options().strip() {
                // Already reversed by an earlier Drop that was refused?
                let reversed = draft.build_options().flip != draft.options().flip;
                operandi_core::pen_strip(points, reversed)
                    .iter()
                    .map(|f| ring(f))
                    .sum()
            } else {
                let all: Vec<u32> = (0..points.len() as u32).collect();
                ring(&all)
            };
            let center = points.iter().copied().sum::<DVec3>() / points.len() as f64;
            let (eye, _, _, forward) = self.camera.basis();
            let toward = if self.camera.view == View::Perspective {
                eye - center
            } else {
                -forward
            };
            if normal.dot(toward) < 0. {
                let _ = self.editor.dispatch(Command::PenReverse);
            }
        }
        // Add Loop / Add Point stay armed until dropped, cancelled or
        // replaced by another tool, like the other click tools.
        if let Some(tool) = self.edge_click
            && matches!(
                c,
                Command::Drop
                    | Command::Cancel
                    | Command::Activate(_)
                    | Command::PenStart
                    | Command::ArmReduction
            )
        {
            self.edge_click = None;
            if matches!(c, Command::Drop | Command::Cancel) && !self.editor_tool_active() {
                self.status = format!("{} off", tool.label());
                return false;
            }
        }
        // Weld ends the same way.
        if self.vertex_weld
            && matches!(
                c,
                Command::Drop
                    | Command::Cancel
                    | Command::Activate(_)
                    | Command::PenStart
                    | Command::ArmReduction
            )
        {
            self.vertex_weld = false;
            self.weld_drag = None;
            if matches!(c, Command::Drop | Command::Cancel) && !self.editor_tool_active() {
                self.status = "Weld off".into();
                return false;
            }
        }
        // An open Pen draft is dropped by the editor's tool switch.
        if matches!(&c, Command::ArmReduction) && self.set_position.is_some() {
            self.status = "Close Set Position before Polygon Reduction".into();
            return false;
        }
        // Unit insertion drops an open tool first (the editor stages both),
        // so pending field text is confirmed into it like any Drop. An
        // unfinished selection gesture still refuses, before any field text
        // is committed.
        if matches!(
            c,
            Command::InsertUnit { .. }
                | Command::InsertEllipsoidUnit(_)
                | Command::InsertShapeUnit(..)
        ) && self.editor.selection_start.is_some()
        {
            self.status = "Finish the selection gesture before unit insertion".into();
            return false;
        }
        if !matches!(
            c,
            Command::Pen(_)
                | Command::FlipPolygons
                | Command::Preview(_)
                | Command::ConfigureSmoothShift(_)
                | Command::ResetSmoothShift
                | Command::ConfigureBevel(_)
                | Command::ConfigureExtrude(_)
                | Command::ResetExtrude
                | Command::Cancel
                | Command::ResetEllipsoid
                | Command::ResetShape
                | Command::ResetBridge
                | Command::ResetSpike
                | Command::ResetToolAttributes
                | Command::ConfigureBridge(_)
                | Command::CleanupFloating(_)
                | Command::SetPart(_)
                | Command::SetPosition(_)
                | Command::AbsoluteScale(_)
                | Command::ScaleOffsetPolygon(_)
                | Command::FlattenNgons
                | Command::SetMaterial(_)
                | Command::UnifyPolygons(_)
                | Command::SpinQuads
                | Command::AlignPolygons
                | Command::SplitPolygon
                | Command::QuadruplePolygon
                | Command::Triple
                | Command::RemovePolygons
                | Command::MergePolygons
                | Command::CollapsePolygon
                | Command::MakePolygon
        ) && !self.confirm_fields()
        {
            return false;
        }
        let history_request = match &c {
            Command::Undo => Some(HistoryRequest::Undo),
            Command::Redo => Some(HistoryRequest::Redo),
            Command::Cancel => Some(HistoryRequest::Cancel),
            _ => None,
        };
        let continuing = matches!(c, Command::Continue);
        let was_live = self.editor.has_active_task();
        // A tree click re-targets a live transform tool (editor.rs): its
        // fields restart at zero like after Undo/Redo.
        let item_select = matches!(
            c,
            Command::SelectItem(_)
                | Command::SelectItems(_)
                | Command::ToggleItemSelection(_)
                | Command::SelectGroup(_)
        );
        let reset_ellipsoid = matches!(
            c,
            Command::ResetEllipsoid
                | Command::ResetShape
                | Command::ResetBridge
                | Command::ResetSpike
                | Command::ResetExtrude
                | Command::ResetSmoothShift
                | Command::ResetBevel
                | Command::ResetToolAttributes
        );
        let discard_draft = matches!(c, Command::Drop)
            && self.editor.live.as_ref().is_some_and(|l| {
                l.params
                    .radial
                    .is_some_and(|p| p.radius.min_element() == 0.)
            });
        let recipe_change = matches!(
            c,
            Command::BevelStateSave(_)
                | Command::BevelStateRename(_)
                | Command::BevelStateDelete
                | Command::BevelStateRecall(_)
        );
        let previous = recipe_change.then(|| self.editor.clone());
        let item_delete = matches!(c, Command::DeleteItems(_));
        let completion = if let Some(request) = history_request {
            self.editor
                .dispatch_history_reported(request)
                .map(Some)
                .map_err(|failure| failure.to_string())
        } else {
            // A held viewport drag or field scrub makes one tweak of the
            // active tool, however many previews it sends (mid-tool undo).
            let held = std::mem::replace(
                &mut self.editor.tweak_held,
                self.pointer_held || self.scrub.is_some(),
            );
            // A colour field's drag or pick is one undo step across frames
            // and items (colour_field.rs); a caller's own gesture (a
            // Render item field drag, an environment layer) is kept.
            let outer = self.editor.history.undo_fold;
            self.editor.history.undo_fold = colour_field::frame_gesture().or(outer);
            // Subdivide checks its result against free memory (scenes
            // replace the editor, so it is set on every command).
            self.editor.free_memory = operandi_core::FreeMemory(Some(
                self.subdivide_ui
                    .free_memory
                    .unwrap_or(free_memory::free_bytes),
            ));
            let result = self.editor.dispatch(c).map(|()| None);
            self.editor.history.undo_fold = outer;
            self.editor.tweak_held = held;
            result
        };
        match completion {
            Ok(completion) => {
                if recipe_change && let Err(e) = self.save_bevel_recipes() {
                    self.editor = previous.unwrap();
                    self.status = format!("Could not save Bevel recipes: {e}");
                    return false;
                }
                if completion.is_some() {
                    self.navigation = None;
                    self.i2_gesture_ticket = None;
                }
                let foreground_closed = completion.as_ref().map_or_else(
                    || was_live && !self.editor.has_active_task(),
                    |completion| completion.foreground_closed,
                );
                if completion.as_ref().is_some_and(|c| {
                    c.restored_local() || matches!(&c.effect, HistoryEffect::PenPointUndo)
                }) || reset_ellipsoid
                    || continuing
                    || foreground_closed
                    || (item_select && was_live)
                    // Undo/Redo restart a live transform tool at zero, and
                    // the fields of a tool that keeps running through
                    // history (the primitives) show its current values.
                    || (completion.is_some() && self.editor.has_active_task())
                {
                    self.clear_fields();
                    if self.editor.live.is_none() {
                        self.surface_mini_properties = false;
                    }
                }
                let discarded = completion.as_ref().and_then(|c| match &c.effect {
                    HistoryEffect::PreviewDiscarded { label } => Some(label.clone()),
                    _ => None,
                });
                let tool_change = completion.as_ref().and_then(|c| match &c.effect {
                    HistoryEffect::ToolClosed { label } => Some(format!("Undo closed {label}")),
                    HistoryEffect::ToolReopened { label } => Some(format!("Redo opened {label}")),
                    HistoryEffect::TweakUndone { label } => {
                        Some(format!("Undo · {label} is still active"))
                    }
                    HistoryEffect::TweakRedone { label } => {
                        Some(format!("Redo · {label} is still active"))
                    }
                    _ => None,
                });
                // UIC-160: report the committed delete on the shared command route.
                self.status = if item_delete && let Some(note) = self.editor.command_note.take() {
                    note
                } else if let Some(label) = discarded {
                    format!("{label} preview undone · the tool is still active")
                } else if let Some(status) = tool_change {
                    status
                } else if continuing {
                    "New stage ready on selected caps · ordinary handles edit this stage; Shift-click continues".into()
                } else if discard_draft {
                    "Flat draft discarded; preceding geometry retained".into()
                } else if self
                    .editor
                    .live
                    .as_ref()
                    .is_some_and(|l| l.tool == Tool::Tweak)
                {
                    // TWR-13: say what the right button does with Tweak.
                    viewport::tweak_view::TWEAK_HINT.into()
                } else if let Some(armed) = self.editor.armed_status() {
                    // CS-06: an armed tool says how to start it.
                    armed
                } else {
                    "Ready".into()
                };
                if let Some(i) = self.editor.scene.active() {
                    self.name = i.name.clone();
                }
                true
            }
            Err(e) => {
                self.status = e;
                false
            }
        }
    }
    fn activate(&mut self, t: Tool) {
        let before = self.editor.live.as_ref().map(|l| l.tool);
        self.run(Command::Activate(t));
        if self.editor.live.as_ref().map(|l| l.tool) != before {
            self.clear_fields();
        }
    }
    fn dirty(&self) -> bool {
        self.dirty_by(same_document)
    }
    /// `dirty` with exact scene equality: an equal copy of a mesh (an undo
    /// that rebuilt it) is unchanged. Too slow for every frame; the MCP
    /// bridge asks it once per call, so a full undo leaves its document
    /// clean and the next scene_load isn't refused.
    pub(crate) fn dirty_exactly(&self) -> bool {
        self.dirty_by(|a, b| a == b)
    }
    fn dirty_by(&self, same: fn(&Scene, &Scene) -> bool) -> bool {
        needs_saving(&self.editor, &self.saved, same)
    }
    fn request_file(&mut self, a: FileAction) {
        if self.pending.is_some() || self.file_dialog.is_some() {
            return;
        }
        if let FileAction::Export(format) = a {
            self.request_export(format);
            return;
        }
        // New and Open keep the current scene, tool and all, in its tab, so
        // there is nothing to save first.
        if matches!(a, FileAction::New | FileAction::Open | FileAction::Import) {
            self.do_file(a);
            return;
        }
        // Quit: the edited scene is clean, but another may not be.
        if matches!(a, FileAction::Close) {
            self.scenes.quit_discarded.clear();
        }
        if matches!(a, FileAction::SaveAs) {
            let before = self.editor.clone();
            let confirmed = self.confirmed_fields.clone();
            let errors = self.field_errors.clone();
            if !self.save_field_edits() {
                return;
            }
            self.editor = before;
            self.confirmed_fields = confirmed;
            self.field_errors = errors;
            // The native panel asks before replacing an existing file.
            let (folder, name) = file_chooser::split(&self.save_dialog_default());
            if self.choose(
                file_chooser::Choose::Save { folder, name },
                file_chooser::Purpose::SaveAs,
            ) {
                return;
            }
            self.file_dialog = Some(FileDialog {
                action: a,
                path: self.save_dialog_default(),
                overwrite: false,
            });
        } else if self.dirty()
            || self.editor.live.is_some()
            || self.editor.pen_draft.is_some()
            || self
                .editor
                .ellipsoid
                .as_ref()
                .is_some_and(|s| s.preview.is_some())
            || self
                .editor
                .shape
                .as_ref()
                .is_some_and(|s| s.preview.is_some())
        {
            self.pending = Some(a);
            self.prompt_focused = false;
        } else {
            self.do_file(a);
        }
    }
    // The initial mesh belongs to the document baseline, not edit history.
    // Keep generic empty editors available for loading and isolated tests.
    fn initialize_new_document(&mut self) {
        // CA-CON-003/015: a new scene alone keeps Cone's pure settings: an
        // active session's confirmed valid values, else the Drop-stored ones.
        // A session holding a field error keeps the stored values (unchanged
        // behavior; no invalid-draft policy is admitted). The selected UV map
        // names a map of the old document, so it keeps its New default.
        let current = self
            .editor
            .cone
            .as_ref()
            .filter(|s| s.error.is_none())
            .map_or(
                self.editor.presets.get::<operandi_core::ConePreset>(),
                |s| &s.requested,
            );
        let cone_preset = ConeParameters {
            selected_uv_map: None,
            ..current.clone()
        };
        self.editor = Editor {
            bevel_states: self.editor.bevel_states.clone(),
            bevel_state: self.editor.bevel_state.clone(),
            bevel_profile: self.editor.bevel_profile,
            ..Default::default()
        };
        self.editor
            .presets
            .set::<operandi_core::ConePreset>(cone_preset);
        // Default Scene (Preferences > Defaults > Application): a scene
        // file every new scene starts from, untitled. Provisional (PREF-Q9):
        // Operandi scene files only; one that won't load is skipped with a
        // status line.
        let mut status = None;
        let default_scene = self.defaults_preferences.default_scene.trim().to_owned();
        let template = (!default_scene.is_empty())
            .then(
                || match persistence::load(std::path::Path::new(&default_scene)) {
                    Ok(scene) => Some(scene),
                    Err(e) => {
                        status = Some(format!("Default Scene {default_scene}: {e}"));
                        None
                    }
                },
            )
            .flatten();
        if let Some(scene) = template {
            self.editor.scene = scene;
        } else {
            self.editor
                .scene
                .new_mesh()
                .expect("fresh scene has capacity for its first mesh");
            // CL-1: every new scene also gets one camera and one directional
            // light (opened files keep what they have). The mesh stays
            // selected.
            self.editor
                .scene
                .add_default_camera_and_light()
                .expect("fresh scene has capacity for its camera and light");
            // RI-05: and the new-scene render settings.
            self.editor.scene.use_new_scene_render_settings();
            // New scene spec NS01: the mesh selected, in Item mode.
            self.editor.scene.selection.mode = Mode::Item;
            // Item Drawing: Default Size for New Scenes.
            self.editor.scene.item_draw_size = self.viewport_preferences.item_draw_size;
        }
        self.editor.next_item_id = self.editor.scene.next_id;
        self.apply_tool_defaults();
        self.saved = self.editor.scene.clone();
        self.path = None;
        self.name = self
            .editor
            .scene
            .active()
            .map(|i| i.name.clone())
            .unwrap_or_default();
        self.clear_fields();
        self.status = status.unwrap_or_else(|| "New scene".into());
    }
    fn do_file(&mut self, a: FileAction) {
        match a {
            FileAction::New => self.new_scene_tab(),
            FileAction::Open | FileAction::Import => {
                if let Some(path) = self.queued_open.take() {
                    self.open(path);
                    return;
                }
                let (folder, _) = file_chooser::split(&self.open_dialog_default());
                let request = if matches!(a, FileAction::Import) {
                    file_chooser::Choose::Import { folder }
                } else {
                    file_chooser::Choose::Open { folder }
                };
                // PG-07: Import adds to the current scene.
                let purpose = if matches!(a, FileAction::Import) {
                    file_chooser::Purpose::Import
                } else {
                    file_chooser::Purpose::Open
                };
                if self.choose(request, purpose) {
                    return;
                }
                self.file_dialog = Some(FileDialog {
                    action: a,
                    path: self.open_dialog_default(),
                    overwrite: false,
                });
            }
            FileAction::SaveAs | FileAction::Export(_) => self.request_file(a),
            // Closing waits while the scene's image save panels are open,
            // so their answers write this scene's pixels (Astra P8-001).
            #[cfg(feature = "painting")]
            FileAction::Close | FileAction::CloseScene if self.image_saves_pending() => {
                self.painting.after_images = Some(painting::AfterImages::File(a));
            }
            FileAction::Close => self.continue_quit(),
            FileAction::CloseScene => {
                self.close_active_scene();
                if self.scenes.closing_all {
                    self.close_all_step();
                }
            }
        }
    }
    fn open(&mut self, path: PathBuf) {
        use std::io::Read;
        let mut magic = [0u8; 4];
        let form = std::fs::File::open(&path)
            .and_then(|mut f| f.read_exact(&mut magic))
            .is_ok()
            && &magic == b"FORM";
        if form
            || path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lxo"))
        {
            self.begin_lxo(path);
            return;
        }
        // USD asks its import options first (docs/BEHAVIOR-USD-IMPORT.md).
        if interchange::is_usd(&path) {
            self.ask_usd_import(path);
            return;
        }
        if interchange::is_importable(&path) {
            self.import_interchange(path);
            return;
        }
        // A preset dropped on the app or picked in Open loads into the
        // current scene, as File > Load Preset… does (PB-50).
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(operandi_core::preset::is_preset_extension)
        {
            let group = self.preset_target_group();
            self.load_preset_from(path, group, true);
            return;
        }
        if self.switch_to_open_file(&path) {
            return;
        }
        match persistence::load(&path) {
            Ok(scene) => self.open_loaded(path, scene),
            Err(e) => {
                self.status = format!("Open {}: {e}", path.display());
                self.report_file_error("Open", format!("Couldn't open {}.\n\n{e}", path.display()));
            }
        }
    }
    /// Makes `scene`, read from `path`, the open document.
    fn open_loaded(&mut self, path: PathBuf, scene: Scene) {
        self.make_room_for_document();
        self.editor = Editor {
            bevel_states: self.editor.bevel_states.clone(),
            bevel_state: self.editor.bevel_state.clone(),
            bevel_profile: self.editor.bevel_profile,
            scene: scene.clone(),
            ..Default::default()
        };
        self.saved = scene;
        self.path = Some(path);
        self.clear_fields();
        self.name = self
            .editor
            .scene
            .active()
            .map(|i| i.name.clone())
            .unwrap_or_default();
        self.status = "Scene opened".into();
        self.frame(false);
    }
    /// Shows a failed Save or Open as a dialog, as well as in the status line.
    fn report_file_error(&mut self, title: &str, message: String) {
        self.file_error = Some(Box::new((title.into(), message)));
    }
    fn file_error_dialog(&mut self, ctx: &Context) {
        let Some(error) = self.file_error.clone() else {
            return;
        };
        if self.show_notice(ctx, &error.0, &error.1) != form::Outcome::Open {
            self.file_error = None;
        }
    }
    fn save_field_edits(&mut self) -> bool {
        if self
            .editor
            .scene
            .active()
            .is_some_and(|item| item.name != self.name)
        {
            self.status =
                "Confirm the mesh name with Enter/Rename, or Escape to revert before Save".into();
            return false;
        }
        self.confirm_fields()
    }
    fn save(&mut self, path: PathBuf) -> bool {
        let before = self.editor.clone();
        let fields = self.fields.clone();
        let confirmed = self.confirmed_fields.clone();
        let errors = self.field_errors.clone();
        if !self.save_field_edits() {
            return false;
        }
        match self.editor.save_committed(&path) {
            Ok(()) => {
                self.clear_fields();
                self.saved = self.editor.scene.clone();
                self.path = Some(path);
                self.status = "Scene saved".into();
                #[cfg(feature = "painting")]
                self.after_scene_save();
                true
            }
            Err(e) => {
                // A failed save leaves the panel as it was.
                self.editor = before;
                self.fields = fields;
                self.confirmed_fields = confirmed;
                self.field_errors = errors;
                // Loud: a dialog as well as the status line. The title
                // keeps its "*" and the scene keeps its old name and path.
                self.report_file_error(
                    "Save",
                    format!("The scene was not saved; nothing changed on disk.\n\n{e}"),
                );
                self.status = e;
                false
            }
        }
    }
    fn save_current(&mut self) {
        if let Some(p) = self.path.clone() {
            self.save(p);
        } else {
            self.request_file(FileAction::SaveAs);
        }
    }
    fn frame(&mut self, selection: bool) {
        // A and Shift+A in a UV view frame its UVs (UVV-05).
        if self.uv_view_frame(selection) {
            return;
        }
        fn selected_whole_item(scene: &operandi_core::Scene, id: u64) -> bool {
            scene.selection.item == Some(id) && scene.selection.mode == Mode::Item
        }
        // Frame what the views show: an open tool's preview included, and
        // for items with a Mesh Stack their result (AR-01), not their Base
        // Mesh, which may be empty.
        let snapshot = self.editor.shown_display();
        let scene = snapshot.as_ref().unwrap_or(&self.editor.scene);
        // With a creation tool open, A frames the shape being made.
        let tool_points = (!selection)
            .then(|| self.creation_tool_frame_points())
            .flatten();
        let framing_tool = tool_points.is_some();
        let mut points = tool_points.unwrap_or_default();
        for i in scene
            .items
            .iter()
            .filter(|i| !framing_tool && scene.shows(i))
        {
            if selection && Some(i.id) != scene.selection.item {
                continue;
            }
            if selection {
                let selected = &scene.selection;
                match selected.mode {
                    Mode::Vertex => points.extend(
                        selected
                            .vertices
                            .iter()
                            .map(|&v| i.transform.point(i.mesh.vertices[v as usize])),
                    ),
                    Mode::Edge => {
                        for &(a, b) in &selected.edges {
                            points.extend([
                                i.transform.point(i.mesh.vertices[a as usize]),
                                i.transform.point(i.mesh.vertices[b as usize]),
                            ]);
                        }
                    }
                    Mode::Polygon => {
                        for &f in &selected.faces {
                            points.extend(
                                i.mesh.faces[f as usize]
                                    .vertices
                                    .iter()
                                    .map(|&v| i.transform.point(i.mesh.vertices[v as usize])),
                            );
                        }
                    }
                    Mode::Item => {
                        points.extend(i.mesh.vertices.iter().map(|&v| i.transform.point(v)))
                    }
                }
            } else {
                points.extend(i.mesh.vertices.iter().map(|&v| i.transform.point(v)));
            }
            // BD-42: view fitting includes the Backdrops this view draws
            // (unless Include Backdrops in Viewport Fitting is off).
            if self.viewport_preferences.backdrop_fitting
                && (!selection || selected_whole_item(scene, i.id))
                && self.display.settings.drawing.backdrops
                && i.backdrop
                    .as_ref()
                    .is_some_and(|b| b.drawn_in(backdrop_ui::view_kind(self.camera.view)))
                && let Some(corners) = backdrop_ui::shown_corners(scene, i.id)
            {
                points.extend(corners);
            }
        }
        if points.is_empty() {
            self.status = "Nothing visible to frame".into();
            return;
        }
        let lo = points
            .iter()
            .copied()
            .fold(DVec3::splat(f64::INFINITY), DVec3::min);
        let hi = points
            .iter()
            .copied()
            .fold(DVec3::splat(f64::NEG_INFINITY), DVec3::max);
        self.camera.target = (lo + hi) * 0.5;
        let r = self.viewport_rect;
        let (w, h) = if r.is_finite() && r.is_positive() {
            (r.width(), r.height())
        } else {
            (1., 1.)
        };
        // fit_distance assumes the default lens; a view whose lens kept its
        // world scale through Maximise frames the same way.
        let lens = if self.camera.looks_through() {
            1.
        } else {
            self.camera.lens / camera::DEFAULT_LENS
        };
        self.camera.distance = self
            .units_preferences
            .fit_distance((hi - lo).length(), w, h)
            * lens;
        // A framed view is back at a normal distance: the fixed near clip.
        self.camera.near_depth = f64::INFINITY;
    }
    fn shortcut(&self, name: &str) -> String {
        self.bindings
            .get(name)
            .filter(|s| !s.is_empty())
            .map(|s| format!("  [{}] ", input::display_chord(s)))
            .unwrap_or_default()
    }
    /// A text or number field has keyboard focus, so every app shortcut is
    /// off and its keys (M, Tab, digits...) go to the field. Any egui text
    /// edit counts, listed in `text_editors` or not, so a field drawn
    /// without registering itself can't leak its keys to the shortcuts.
    fn text_editor_focused(&self, ctx: &Context) -> bool {
        let Some(id) = ctx.memory(|m| m.focused()) else {
            return false;
        };
        self.text_editors.contains(&id) || numeric_tab::is_text_field(ctx, id)
    }
    fn has_modal(&self) -> bool {
        self.open_dialog().is_some()
    }
    /// The open modal dialog's name, if one is open (the MCP bridge names
    /// it when it refuses an edit).
    fn open_dialog(&self) -> Option<&'static str> {
        let open = [
            (self.choosing.is_some(), "file chooser"),
            (self.layouts.dialog.is_some(), "layout dialog"),
            (self.import_job.is_some(), "import progress"),
            (self.import_ready.is_some(), "import options"),
            (self.usd_import.is_some(), "USD import options"),
            (self.import_message.is_some(), "import message"),
            (self.file_dialog.is_some(), "file dialog"),
            (self.project_dialog.is_some(), "project dialog"),
            (self.pending.is_some(), "unsaved changes question"),
            (self.scenes.confirm_close_all, "Close All question"),
            (self.scenes.confirm_delete_children, "Delete question"),
            (
                self.scenes.confirm_last_mesh.is_some(),
                "last mesh item question",
            ),
            (self.show_bindings, "Key Bindings window"),
            (self.show_preferences, "Preferences window"),
            (self.preset_browser.asking(), "Preset Browser question"),
            (
                self.preferences_ui.confirm_reset,
                "Reset Preferences question",
            ),
            (self.recovery.is_some(), "auto-save recovery question"),
            (
                self.recovery_warning.is_some(),
                "auto-save recovery warning",
            ),
            (self.mesh_cleanup.is_some(), "Mesh Cleanup dialog"),
            (self.set_part.is_some(), "Set Part dialog"),
            (self.selection_set.is_some(), "Selection Set dialog"),
            (self.set_position.is_some(), "Set Position dialog"),
            (self.vertex_dialog.is_some(), "vertex dialog"),
            (self.scale_offset.is_some(), "Scale/Offset dialog"),
            (self.solid_sketch_ask.is_some(), "Solid Sketch question"),
            (self.set_material.is_some(), "Set Material dialog"),
            (self.align_notice.is_some(), "Align notice"),
            (self.file_error.is_some(), "file error"),
            (self.unify.is_some(), "Unify dialog"),
            (self.flip_it.is_some(), "Flip It dialog"),
            (self.report_bug.is_modal(), "Report Bug or Idea dialog"),
            (self.unify_count.is_some(), "Unify result"),
            (self.preset_dialog.is_some(), "preset dialog"),
            (self.subdivide_ui.open(), "Subdivide dialog"),
            (self.detriangulate.is_some(), "Detriangulate dialog"),
            (self.curves_color.is_some(), "Set Curves Color dialog"),
            (self.cut_dialog.is_some(), "Dicer dialog"),
            (self.curve_dialog.is_some(), "curve dialog"),
            (self.relax_mesh.is_some(), "Relax Mesh dialog"),
            (self.select_dialog.is_some(), "selection tool dialog"),
            (
                self.env_preset_dialog.is_some(),
                "Save Environment Preset dialog",
            ),
            (self.remesh.is_some(), "Remesh dialog"),
            (self.remesh_job.is_some(), "Remesh progress"),
            (self.uv_dialog.is_some(), "UV dialog"),
            (self.bake_dialog.is_some(), "bake dialog"),
            #[cfg(feature = "painting")]
            (self.painting_dialog_open(), "painting dialog"),
            (self.boolean.is_some(), "Boolean dialog"),
            (self.boolean_notice.is_some(), "Boolean notice"),
            (self.drill.is_some(), "Drill dialog"),
            // Astra P8-003: it was missing, so a queued Open could run
            // under it and the answer delete same-numbered layers there.
            (
                self.shading.confirm_delete.is_some(),
                "shader Delete question",
            ),
            // Last: it waits until every dialog above is closed.
            (self.update_dialog_open(), update_check_ui::DIALOG_NAME),
        ];
        open.into_iter().find(|(on, _)| *on).map(|(_, name)| name)
    }
    fn editor_input_allowed(&self, ctx: &Context) -> bool {
        !self.modal_owned_input
            && !self.menu_owned_input
            && !self.has_modal()
            && !egui::Popup::is_any_open(ctx)
    }
    fn begin_input(&mut self, ctx: &Context) {
        self.cancel_dot_drag(ctx);
        // Keys and hints follow the current layout's key context (LAY-20).
        self.sync_layout_keys();
        // UIC-51: a Tab taken from a numeric field moves focus before
        // anything draws, so the old field accepts and the new one opens.
        numeric_tab::begin_pass(ctx);
        self.release_idle_field_over_view(ctx);
        self.drop_key_in_tool_field(ctx);
        self.frame_status = self.status.clone();
        self.frame_action = false;
        self.menu_owned_input = native_menu::tracking()
            || self.pie.is_some()
            || egui::Popup::is_any_open(ctx)
            || self.corner.properties.owns_input();
        self.modal_owned_input = self.has_modal();
        self.editor_owned_input = self.text_editor_focused(ctx)
            || self.escape_left_a_field(ctx)
            || self.menu_owned_input
            || self.modal_owned_input;
        let scene_focus_rect = self
            .scene_focus_tab
            .and_then(|kind| self.visible_scene_lists.get(&kind).copied())
            .unwrap_or(self.scene_rect);
        if ctx.memory(|m|m.has_focus(Id::new("scene-list-focus"))) && ctx.input(|i| i.events.iter().any(|e| matches!(e,Event::PointerButton{pos,pressed:true,..} if !scene_focus_rect.contains(*pos)))) {
            ctx.memory_mut(|m|m.surrender_focus(Id::new("scene-list-focus")));
        }
        self.layout_owned_input = self.layout_input(ctx);
        self.button_variants.update(ctx);
    }
    /// SDV-02: when no text field has focus and Tab (with its modifiers) is
    /// bound in the input map, Tab is a shortcut, not egui focus travel. A
    /// focused value field keeps Tab.
    fn hold_bound_tab(&mut self, ctx: &Context, input: &mut RawInput) {
        if self.text_editor_focused(ctx)
            || self.has_modal()
            || egui::Popup::is_any_open(ctx)
            || native_menu::tracking()
        {
            return;
        }
        let bound = |mods: &Modifiers| {
            self.bindings
                .values()
                .any(|b| input::matches_chord(b, Key::Tab, *mods))
        };
        let mut held = Vec::new();
        input.events.retain(|event| match event {
            Event::Key {
                key: Key::Tab,
                modifiers,
                pressed,
                ..
            } if bound(modifiers) => {
                if *pressed {
                    held.push(event.clone());
                }
                false
            }
            _ => true,
        });
        self.held_tab.extend(held);
    }
    fn keyboard(&mut self, ctx: &Context) {
        // MN-02: a held tool key's release, before anything can return.
        self.key_hold_frame(ctx);
        let held_tab = std::mem::take(&mut self.held_tab);
        // The quick layout switcher owns the keys while it is open (UIC-18).
        if self.switcher_input(ctx, &held_tab) {
            return;
        }
        // An open pie menu owns the keys and clicks.
        if self.pie_input(ctx) {
            return;
        }
        // DA-6: Esc during a drop drag ends it without a change.
        if self.drop_drag.is_some()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.cancel_drop();
            ctx.request_repaint();
            return;
        }
        // Escape during a Weld drag cancels just that drag (nothing has
        // changed yet) and leaves Weld armed for another.
        if self.weld_drag.is_some()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.weld_drag = None;
            self.status = "Weld drag cancelled".into();
            ctx.request_repaint();
            return;
        }
        if self.navigation.is_some() {
            return;
        }
        // Pie keys open over any popup (closing it), so they come first.
        if self.pie_keys(ctx) {
            return;
        }
        if self.modal_owned_input || self.menu_owned_input || self.has_modal() {
            return;
        }
        // Next/Previous Tool Tab, also with a field focused (tool_tab_cycle.rs).
        self.tool_tab_keys(ctx);
        if let Some(action) = ctx.input(|i| input::file_shortcut(&i.events)) {
            match action {
                input::FileShortcut::Save => {
                    if self.file_dialog.is_none() && self.pending.is_none() {
                        self.save_current();
                    }
                }
                input::FileShortcut::Close => self.request_file(FileAction::CloseScene),
                input::FileShortcut::CloseAll => self.request_close_all(),
            }
            ctx.request_repaint();
            return;
        }
        if self.show_plane
            || self.show_diagnostics
            || self.show_about
            || self.layout_owned_input
            || self.menu_owned_input
            || self.editor_owned_input
            || egui::Popup::is_any_open(ctx)
            || self.text_editor_focused(ctx)
            || self.file_dialog.is_some()
            || self.pending.is_some()
            || self.show_bindings
            || self.recovery.is_some()
            || self.recovery_warning.is_some()
        {
            return;
        }
        // PNT-171: E/R/D/F size and turn Image Ink's widget while it shows.
        #[cfg(feature = "painting")]
        if self.ink_widget_keys(ctx) {
            ctx.request_repaint();
            return;
        }
        // CP-A04: arrow keys walk between items in Item mode.
        if self.pick_walk_key(ctx) {
            ctx.request_repaint();
            return;
        }
        // SPEC-BACKGROUND-CONSTRAINT: Esc over a view with no tool edit in
        // progress gets rid of the background constraint.
        if self.escape_clears_background_constraint()
            && ctx
                .pointer_hover_pos()
                .is_some_and(|p| self.viewport_rect.contains(p))
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.toggle_background_constraint();
            ctx.request_repaint();
            return;
        }
        // FO-02: Esc over a view with no tool edit in progress clears the
        // falloff.
        if self.escape_clears_falloff()
            && ctx
                .pointer_hover_pos()
                .is_some_and(|p| self.viewport_rect.contains(p))
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.run(Command::SpatialFalloff(None));
            self.status = "Falloff cleared".into();
            ctx.request_repaint();
            return;
        }
        let i2_escape_owner = self.i2_escape_owner();
        // Create's points are a scratch preview: Escape over the viewport
        // cancels them, as its panel says ("Esc cancels").
        let create_escape_owner = self
            .editor
            .live
            .as_ref()
            .is_some_and(|l| l.tool == Tool::VertexCreate);
        if (i2_escape_owner || self.vertex_weld || create_escape_owner)
            && ctx
                .pointer_hover_pos()
                .is_some_and(|p| self.viewport_rect.contains(p))
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.run(if create_escape_owner {
                Command::Cancel
            } else {
                Command::Drop
            });
            ctx.request_repaint();
            return;
        }
        // CV-01/02/04: Q (the Drop binding) and Esc drop a Draw tool, a
        // Create utility or a sculpt brush like other tools (Space did that
        // before CV-03): the curve stays, the tool goes. Esc only over the
        // viewport, as for Weld, so panels keep theirs.
        if self.draw.is_some() || self.utility.is_some() || self.sculpt.brush.is_some() {
            let drop_chord = self.bindings.get("Drop").cloned().unwrap_or_default();
            let over_view = ctx
                .pointer_hover_pos()
                .is_some_and(|p| self.viewport_rect.contains(p));
            let hit = ctx.input(|i| {
                i.events.iter().find_map(|event| match event {
                    Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } if input::matches_chord(&drop_chord, *key, *modifiers)
                        || (over_view && *key == Key::Escape && modifiers.is_none()) =>
                    {
                        Some((*modifiers, *key))
                    }
                    _ => None,
                })
            });
            if let Some((modifiers, key)) = hit {
                ctx.input_mut(|i| i.consume_key(modifiers, key));
                if self.utility.is_some() {
                    self.drop_utility();
                } else if self.sculpt.brush.is_some() {
                    self.drop_sculpt();
                } else {
                    self.drop_draw();
                }
                ctx.request_repaint();
                return;
            }
        }
        // CV-03 / MP-01 / UIC-66: a Space tap or hold opens Mini Properties
        // at the pointer; Space again closes it. Alt+Shift+Space (the old
        // mode cycle) is the Input Map's "Select Next Mode".
        let space = ctx.input(|i| {
            i.events.iter().any(|event| {
                matches!(event,
                    Event::Key { key: Key::Space, pressed: true, repeat: false, modifiers, .. }
                        if modifiers.is_none())
            })
        });
        if space && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space)) {
            self.mini_properties_anchor = ctx.pointer_hover_pos();
            self.execute(mini_properties::SPACE_LINE);
            ctx.request_repaint();
            return;
        }
        let mut commands = Vec::new();
        // Which key ran each command, for key toggles (MN-02).
        let mut command_keys: Vec<(String, Key)> = Vec::new();
        let mut conflict = false;
        let mut missing_physical = false;
        let mut numbers = self.native_numbers.iter();
        let focus = Id::new("scene-list-focus");
        if self
            .scene_focus_tab
            .is_some_and(|kind| !self.visible_scene_lists.contains_key(&kind))
        {
            ctx.memory_mut(|m| m.surrender_focus(focus));
            self.scene_focus_tab = None;
        }
        let scene_focused = ctx.memory(|m| m.has_focus(focus));
        let scene_hovered = ctx
            .pointer_hover_pos()
            .is_some_and(|p| self.scene_rect.contains(p));
        let viewport_hovered = ctx
            .pointer_hover_pos()
            .is_some_and(|p| self.viewport_rect.contains(p));
        // View shortcuts (Maximise, Frame All, view type, shading, zoom...)
        // act on the view under the pointer, whichever is active (UIC-70).
        let pointer_over_view = self.pointer_over_a_view(ctx);
        // MN-08.2: the pointer picks the panel, not the keyboard focus.
        let hovered_panel = (!pointer_over_view)
            .then(|| ctx.pointer_hover_pos().and_then(|p| self.dock_panel_at(p)))
            .flatten();
        self.panel_target = hovered_panel;
        let hovered_view = self.view_under_pointer(ctx);
        // Delete / Backspace over a view delete the selection of the
        // selection mode, with the points left unused
        // (input::viewport_delete_action; Jason, 2026-11-21). The pointer
        // picks the target (MN-08.2): a Shader Tree showing beside the view
        // no longer turns the keys off there (bug 2026-11-21, from the
        // docking change of 2026-11-19). With a list focused they delete
        // that list's rows, as before.
        let mode = self.editor.scene.selection.mode;
        let viewport_delete = !scene_focused && pointer_over_view;
        self.corner.target = Some(hovered_view);
        self.corner.target_pointer = ctx.pointer_hover_pos().filter(|_| pointer_over_view);
        // SS-U03: on the factory map, viewport Escape drops armed or live Smooth
        // Shift/Thicken like Q through the default Drop Cube assignment. It is a
        // fallback only: if any other action is bound to the key (available,
        // filtered out or conflicting) or Drop Cube is unbound/rebound, the
        // existing resolution below is left unchanged.
        let smooth_shift_default_drop = (viewport_hovered
            && self
                .editor
                .live
                .as_ref()
                .is_some_and(|l| l.tool.is_smooth_shift()))
        .then(|| {
            input::ACTIONS.iter().find(|&&(action, chord)| {
                action == "Drop Cube" && self.bindings.get(action).is_some_and(|b| b == chord)
            })
        })
        .flatten();
        let escape_drops_tool = self.escape_drops_tool();
        ctx.input(|i| {
            for event in i.events.iter().chain(&held_tab) {
                if let Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                {
                    let digit = if *key == Key::Period {
                        Some(10)
                    } else {
                        key.name().parse::<u8>().ok().filter(|&d| d <= 9)
                    };
                    let location = digit.and_then(|digit| {
                        numbers
                            .find(|n| n.digit == digit && n.pressed)
                            .map(|n| n.keypad)
                    });
                    missing_physical |= digit.is_some() && location.is_none();
                    let mut event_commands = Vec::new();
                    for (action, binding) in &self.bindings {
                        if (action != "Drop Cube"
                            || self
                                .editor
                                .live
                                .as_ref()
                                .is_some_and(|live| live.tool == Tool::Box))
                            && input::available(
                                action,
                                self.editor.scene.selection.mode,
                                (self.editor_tool_active()
                                    // UIC-306: Grow/Shrink re-target a live
                                    // transform tool between drags.
                                    && !(input::keeps_transform(action)
                                        && self.editor.selection_keeps_tool()
                                        && !self.pointer_held))
                                    || self.edge_click.is_some()
                                    || self.vertex_weld,
                            )
                            && !pie_menu::is_pie(action)
                            && !image_viewer_ui::is_viewer_action(action)
                            && (action != "Tool Properties"
                                || (!scene_focused
                                    && self
                                        .editor
                                        .live
                                        .as_ref()
                                        .is_some_and(|l| l.tool.is_smooth_shift())))
                            && (!action.starts_with("Delete Item")
                                || scene_focused
                                || viewport_delete)
                            // SC-080 / UIC-70: Esc drops the selection of
                            // the view under the pointer, not a panel's.
                            && (action != "Drop Selection" || pointer_over_view)
                            && (action != "New Mesh" || scene_hovered)
                            && (!action.ends_with(" View") || pointer_over_view)
                            && (!viewport_buttons::pointer_view_action(action)
                                || pointer_over_view
                                // MN-08: Maximise over any docked panel.
                                || (viewport_buttons::is_maximize(action)
                                    && hovered_panel.is_some()))
                            && (digit.is_none() || location == Some(binding.contains("Numpad")))
                            && input::matches_chord(binding, *key, *modifiers)
                        {
                            event_commands.push(
                                if viewport_delete && action.starts_with("Delete Item") {
                                    input::viewport_delete_action(mode).to_owned()
                                } else {
                                    action.clone()
                                },
                            );
                        }
                    }
                    if let Some(&(action, chord)) = smooth_shift_default_drop
                        && input::matches_chord(chord, *key, *modifiers)
                        && !self.bindings.iter().any(|(other, binding)| {
                            other != action && input::matches_chord(binding, *key, *modifiers)
                        })
                    {
                        event_commands.push(action.to_owned());
                    }
                    // UIC-24 / UIC-66: Esc over a view drops any tool,
                    // like Q, when nothing else in the map takes it.
                    if event_commands.is_empty()
                        && *key == Key::Escape
                        && modifiers.is_none()
                        && pointer_over_view
                        && escape_drops_tool
                    {
                        event_commands.push("Drop".to_owned());
                    }
                    conflict |= event_commands.len() > 1;
                    if event_commands.len() == 1 {
                        command_keys.push((event_commands[0].clone(), *key));
                        commands.extend(event_commands);
                    }
                }
            }
        });
        if missing_physical {
            self.status="Physical main/keypad identity unavailable for this digit; use labeled mode/view controls".into();
        }
        if conflict {
            self.status =
                "Input map conflict: assign unique keys before invoking this shortcut".into();
            return;
        }
        // The window turns Cmd+X/C/V into Cut/Copy/Paste events, which may
        // come instead of (or as well as) the key: run each once.
        ctx.input(|i| {
            for event in &i.events {
                let action = match event {
                    Event::Cut => "Cut Selection",
                    Event::Copy => "Copy Selection",
                    Event::Paste(_) => "Paste Selection",
                    _ => continue,
                };
                if self.bindings.get(action).is_some_and(|b| !b.is_empty()) {
                    commands.push(action.into());
                }
            }
        });
        let mut seen = std::collections::BTreeSet::new();
        commands.retain(|c| {
            !matches!(
                c.as_str(),
                "Cut Selection" | "Copy Selection" | "Paste Selection"
            ) || seen.insert(c.clone())
        });
        for c in commands {
            if pointer_over_view && viewport_buttons::hovered_view_action(&c) {
                self.activate_view(hovered_view);
            }
            let before = self.editor.active_task();
            self.execute_from(operandi_core::Source::Key, &c);
            if let Some(&(_, key)) = command_keys.iter().find(|(a, _)| *a == c) {
                self.note_key_hold(key, before);
            }
            ctx.request_repaint();
        }
    }
    /// Runs a line from a button, menu or pie (the source the drawing code
    /// set, `CommandHistory::ui_source`) and records it (UIC-224).
    fn execute(&mut self, text: &str) {
        // Pick to Add takes a clicked button's line instead (LAY-82); it
        // reads no names, so it isn't one of the sub-parsers.
        let picked = self.workbench_take_pick(text);
        if picked {
            return;
        }
        self.execute_from(self.history.ui_source, text);
    }
    /// Runs one line, unrecorded: `execute_from` records around it.
    fn execute_line(&mut self, text: &str) {
        // Deprecated command, tool and argument names become the current
        // ones here, so menus, pie entries, saved bindings and typed lines
        // all reach the parsers below under one name (command_names.rs).
        let rewritten =
            match operandi_core::command_names::rewrite(text, self.editor.scene.selection.mode) {
                Ok(r) => r,
                Err(e) => {
                    self.last_failure = Some(e.clone());
                    self.status = e;
                    return;
                }
            };
        for (old, new) in &rewritten.renamed {
            if let Some(note) = operandi_core::command_names::first_deprecation(old, new) {
                log_ring::log_line!("{note}");
            }
        }
        let text = rewritten.text.as_str();
        // A Modo-named command the parser knows by its phrase (UIC-165):
        // `app.undo` runs as `Undo` (command_registry::parser_spelling).
        let spelled;
        let text = match operandi_core::command_registry::parser_spelling(text) {
            Some(Ok(line)) => {
                spelled = line;
                spelled.as_str()
            }
            Some(Err(e)) => {
                self.last_failure = Some(e.clone());
                self.status = e;
                return;
            }
            None => text,
        };
        // A registry command with a handler (command_handlers.rs). Lines no
        // handler takes yet go through the older parsers below.
        if self.run_registered(text) {
            return;
        }
        if self.sculpt_execute(text) {
            return;
        }
        if self.canonical_command(text) {
            return;
        }
        if let Some(payload) = text.trim().strip_prefix("pen ") {
            match PenInput::parse(payload) {
                Ok(input) => self.run(Command::Pen(input)),
                Err(error) => self.status = error,
            }
            return;
        }
        let lowered = text.trim().to_lowercase();
        if let Some(rest) = lowered
            .strip_prefix("smooth shift ")
            .or_else(|| lowered.strip_prefix("thicken "))
            && let Some((name, value)) = rest.split_once(' ')
            && [
                "offset",
                "scale",
                "angle",
                "thicken",
                "sharp",
                "angle_weight",
                "both_sides",
            ]
            .contains(&name)
        {
            if !self.confirm_fields() {
                return;
            }
            let Some(l) = self
                .editor
                .live
                .as_ref()
                .filter(|l| l.tool.is_smooth_shift())
            else {
                self.status = "Activate Smooth Shift or Thicken first".into();
                return;
            };
            let mut p = l.params.smooth_shift;
            let result = (|| -> Result<()> {
                match name {
                    "offset" => {
                        p.offset = numeric::parse_quantity(value, numeric::Quantity::Distance)?
                    }
                    "scale" => {
                        p.scale = numeric::parse_quantity(value, numeric::Quantity::Percent)? / 100.
                    }
                    "angle" => p.angle = numeric::parse_quantity(value, numeric::Quantity::Angle)?,
                    _ => {
                        let b = match value {
                            "true" | "on" | "1" => true,
                            "false" | "off" | "0" => false,
                            _ => return Err("Use true/on/1 or false/off/0".into()),
                        };
                        match name {
                            "thicken" => p.thicken = b,
                            "sharp" => p.sharp = b,
                            "angle_weight" => p.angle_weight = b,
                            _ => p.both_sides = b,
                        }
                    }
                }
                self.editor.dispatch(Command::ConfigureSmoothShift(p))
            })();
            match result {
                Ok(()) => {
                    self.clear_fields();
                    self.status = "Ready".into();
                }
                Err(e) => self.status = e,
            }
            return;
        }

        // "Open Render Window" is its own phrase: the registry's longest
        // name decides (it used to open a file of that name in lower case).
        if let Some(path) = strip_prefix_ignoring_case(text.trim(), "open ")
            .or_else(|| strip_prefix_ignoring_case(text.trim(), "scene.open "))
            .map(|p| {
                let p = p.trim();
                strip_prefix_ignoring_case(p, "filename:").unwrap_or(p)
            })
            && operandi_core::command_registry::find_line(text)
                .is_some_and(|(e, _)| e.name == "scene.open")
        {
            self.request_open_path(PathBuf::from(path.trim()));
            return;
        }
        if let Some(value) = strip_prefix_ignoring_case(text.trim(), "spike strength ") {
            match numeric::parse_quantity(value, numeric::Quantity::Scalar) {
                Ok(value) => {
                    self.run(Command::ConfigureSpike(value));
                    if self.editor.spike.is_some() {
                        self.sync_numeric_value(
                            "spi_strength",
                            self.editor.spike.as_ref().unwrap().requested,
                            "",
                        );
                    }
                }
                Err(error) => self.status = error,
            }
            return;
        }
        if text.trim().to_lowercase().starts_with("select set ") {
            self.run(Command::SelectPolygonSet(text.trim()[11..].to_owned()));
            return;
        }
        if self.execute_bridge_parameter(text) {
            return;
        }
        if self.execute_pie_command(&lowered) {
            return;
        }
        // Commands that run without a window (command_exec), typed.
        if self.headless_command(text) {
            return;
        }
        self.last_failure = Some("Unknown command".into());
        self.status = "Unknown command. Use Cube, Sphere, Cylinder, Bevel, Extrude, Move, Rotate, Scale, Drop, Cancel, Undo, Redo, New Mesh or Frame All".into();
    }
}
/// `text` after `prefix`, which matches in any case like the other phrases.
fn strip_prefix_ignoring_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &text[prefix.len()..])
}
impl Operandi {
    /// Saves panel section folds after one changed (UIC-107).
    fn save_section_folds(&mut self) {
        if !form::rows::take_folds_dirty() {
            return;
        }
        let mut state = self.dialog_memory.panel(form::rows::FOLDS);
        state.folded = form::rows::folds();
        self.dialog_memory.set_panel(form::rows::FOLDS, state);
        if let Err(error) = self
            .dialog_memory
            .save(&self.environment.paths().dialog_values())
        {
            self.status = format!("Could not keep the panel's folds: {error}");
        }
    }
    fn compose(&mut self, ctx: &Context) {
        // A Preferences colour change reaches every widget from this frame on.
        theme::refresh(ctx);
        // Tool values a drag changed are recorded before its release lands
        // (Command History, CMD-14); again after the panels drew.
        self.history_frame(ctx, true);
        self.dot_frame_begin();
        self.workbench_frame_begin(ctx);
        self.form_editor_frame_begin(ctx);
        self.tick_view_motion(ctx);
        self.tick_space_mouse(ctx);
        // Decided once per frame: the dock tree is out of `views` while it
        // draws, so panels drawn inside it can't ask the tree themselves.
        self.layouts.message_line_here = self.message_line_in_tool_properties();
        // Playback, the rows' key requests and frame jobs (anim_ui.rs).
        self.anim_frame(ctx);
        // A running script's calls and the Script Console (python_ui.rs).
        self.python_frame(ctx);
        self.toolbar(ctx);
        self.timeline_panel(ctx);
        // The workbench row along the top, when shown (LAY-82).
        self.workbench_top_bar(ctx);
        self.dock_work_area(ctx);
        // Rebuild hit regions from panels drawn this frame, in tree order.
        self.toolbox_rects.clear();
        self.form_rects.clear();
        self.tool_divider_rect = Rect::NOTHING;
        self.scene_divider_rect = Rect::NOTHING;
        self.scene_rect = Rect::NOTHING;
        self.visible_scene_lists.clear();
        self.sidebars(ctx);
        self.viewport(ctx);
        self.autosave_frame(ctx);
        self.floating_windows(ctx);
        self.palettes_frame(ctx);
        self.dot_requests();
        self.close_panel_requests();
        self.render_window(ctx);
        self.node_view_frame();
        self.preview_window(ctx);
        self.snapping_window(ctx);
        self.mesh_constraint_window(ctx);
        self.sync_mesh_constraint();
        self.snaps_window(ctx);
        self.absolute_window(ctx);
        self.preset_browser_frame(ctx);
        self.update_snap_hold(ctx);
        let mut overlay_rects = Vec::new();
        if self.surface_mini_properties
            && self
                .editor
                .live
                .as_ref()
                .is_some_and(|l| l.tool.is_smooth_shift())
        {
            let mut open = true;
            let shown = Window::new("Smooth Shift · Mini Properties")
                .id(Id::new("surface-mini"))
                .open(&mut open)
                .default_width(300.)
                .show(ctx, |ui| {
                    crate::form::rows::scope(ui, "smooth-shift-mini", |ui| {
                        self.tool_panel::<smooth_shift_panel::SurfaceFields>(ui)
                    })
                });
            overlay_rects.extend(shown.map(|s| s.response.rect));
            self.surface_mini_properties = open;
        }
        overlay_rects.extend(self.channel_panel(ctx));
        overlay_rects.extend(self.mini_properties_window(ctx));
        self.overlay_rects = overlay_rects;
        self.workspace_dialogs(ctx);
        self.backdrop_clip_chooser(ctx);
        self.gradient_editor_window(ctx);
        self.about_window(ctx);
        self.project_dialog_window(ctx);
        self.dialogs(ctx);
        self.mesh_cleanup_dialog(ctx);
        self.set_part_dialog(ctx);
        self.selection_set_dialog(ctx);
        self.preset_dialog_window(ctx);
        self.set_position_dialog(ctx);
        self.vertex_dialog_window(ctx);
        self.set_material_dialog(ctx);
        self.align_notice_dialog(ctx);
        self.file_error_dialog(ctx);
        self.unify_dialog(ctx);
        self.usd_import_dialog(ctx);
        self.flip_it_dialog(ctx);
        self.report_bug_frame(ctx);
        self.update_check_dialogs(ctx);
        self.subdivide_dialogs(ctx);
        self.detriangulate_dialog(ctx);
        self.curves_color_dialog(ctx);
        self.cut_dialog_frame(ctx);
        self.curve_dialog_frame(ctx);
        self.relax_mesh_dialog(ctx);
        self.select_tools_dialog(ctx);
        self.environment_preset_dialog(ctx);
        self.environment_drop_menu(ctx);
        self.remesh_dialogs(ctx);
        self.uv_dialogs(ctx);
        self.bake_dialogs(ctx);
        #[cfg(feature = "painting")]
        self.painting_frame(ctx);
        self.boolean_dialog(ctx);
        self.drill_dialog(ctx);
        self.save_subdivide_settings();
        self.save_sculpt_settings();
        self.scale_offset_dialog(ctx);
        self.solid_sketch_dialog(ctx);
        self.preferences_window(ctx);
        self.reset_preferences_dialog(ctx);
        self.watch_gpu_memory();
        self.layout_cursor(ctx);
        self.pie_window(ctx);
        self.workbench_frame_end(ctx);
        self.history_frame(ctx, false);
        self.help_mode_after_compose(ctx);
    }
}
impl Operandi {
    /// Sends the main window title to the OS, only when it changed. Every
    /// viewport command makes egui draw another frame, so sending it each
    /// frame kept the app redrawing at the display's full rate (120 fps on
    /// ProMotion) with nothing happening, called NSWindow setTitle every
    /// frame, and kept the main thread waiting on the display, which on a
    /// tester's Mac delayed every click (2026-11-12).
    fn sync_window_title(&mut self, ctx: &Context) {
        let title = format!(
            "{}{} — {}{}{}",
            self.environment.title(),
            self.project
                .as_ref()
                .map(|p| format!(" — {}", p.name()))
                .unwrap_or_default(),
            self.path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or("Untitled".into()),
            if self.dirty() { " *" } else { "" },
            self.mcp_title()
        );
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }
}

impl eframe::App for Operandi {
    fn on_exit(&mut self) {
        self.main_window
            .save(&self.environment.paths().main_window());
        // A quit during a camera drag keeps the view (persist_layout waits
        // for the drag to end).
        self.persist_layout_on_exit();
        self.autosave_quit();
    }
    fn raw_input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        self.input_preferences.options.adjust_input(input);
        // Help mode's click and Esc reach no control (help_mode.rs).
        self.help_mode_take_input(input);
        // A numeric field's Tab moves to the next field (UIC-51); only the
        // rest may become a bound Tab shortcut.
        numeric_tab::take_tab(ctx, input);
        // Ctrl+Tab / Ctrl+Shift+Tab page the tool tabs (tool_tab_cycle.rs).
        self.take_tool_tab_keys(ctx, input);
        self.hold_bound_tab(ctx, input);
        // The drag benchmark's scripted pointer events.
        if let Some(bench) = &mut self.drag_bench {
            input.events.append(&mut bench.events);
        }
        // The MCP bridge's pointer_stroke (mcp_pointer.rs).
        self.mcp_take_stroke_input(input);
        // Puck buttons press their shortcuts as keys (space_mouse/buttons.rs).
        self.space_mouse_take_events(&mut input.events);
        self.native_numbers = native::take_numbers();
        // Input > Pen Tablet: the curve is applied where samples arrive.
        tablet::set_curve(self.input_preferences.pen_pressure_curve);
        self.space_mouse_take_numbers();
        self.input_preferences.options.keypad_enter_to_tab(
            &mut input.events,
            native::take_keypad_enters(),
            &mut self.keypad_enter_held,
        );
        let allowed = !self.text_editor_focused(ctx)
            && !self.has_modal()
            && !egui::Popup::is_any_open(ctx)
            && !native_menu::tracking();
        for number in &self.native_numbers {
            if number.digit == 8 && !number.keypad {
                self.lasso_partial_held = number.pressed && allowed;
            }
        }
        if input
            .events
            .iter()
            .any(|event| matches!(event, Event::WindowFocused(false)))
        {
            self.lasso_partial_held = false;
        }
    }
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.frame_stats.begin();
        self.frame_ctx = Some(ctx.clone());
        if let Some(text) = self.copy_next.take() {
            ctx.copy_text(text);
        }
        // Mesh Stacks evaluate on worker threads in the app (SAFE-02); a
        // finished result repaints.
        self.editor.mesh_stack.ensure_background(|| {
            let ctx = ctx.clone();
            std::sync::Arc::new(move || ctx.request_repaint())
        });
        // Interface Scale (ui_preferences.rs) before anything is laid out.
        ui_preferences::sync_zoom(ctx);
        let zoom = ctx.zoom_factor();
        ctx.input(|i| self.main_window.observe_zoomed(i.viewport(), zoom));
        colour_field::begin_frame();
        self.sync_tag_shader_selection();
        self.take_picker_color_units();
        self.poll_import();
        self.poll_choice(ctx);
        self.flush_copy_notes();
        self.serve_mcp(ctx);
        self.poll_render(ctx);
        self.poll_preview(ctx);
        self.remember_action_context();
        self.settle_topology_pen();
        self.keep_awake_while_rendering();
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            let at = self.main_window_pointer(ctx);
            self.files_dropped(dropped, at);
        }
        self.document_opens.extend(open_documents::take());
        self.open_next_document();
        self.begin_input(ctx);
        if (native::take_quit() || ctx.input(|i| i.viewport().close_requested()))
            && !self.allow_close
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            // Changed images are listed first (PNT-34), in every scene.
            #[cfg(feature = "painting")]
            self.begin_quit();
            #[cfg(not(feature = "painting"))]
            self.request_file(FileAction::Close);
        }
        if self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        let actions = native_menu::take_actions();
        if !actions.is_empty() {
            for action in actions {
                self.menu_action(ctx, action);
            }
            // The native menu owns activation input for this frame, so Return,
            // digit navigation or a released accelerator cannot also model.
            self.menu_owned_input = true;
            self.editor_owned_input = true;
            self.lasso_partial_held = false;
        }
        self.compose(ctx);
        self.save_section_folds();
        self.mcp_after_compose();
        if let Some(status) = self.crash_prompt.ui(ctx) {
            self.status = status;
        }
        self.keep_utility_exclusive();
        self.keep_sculpt_exclusive();
        self.keep_draw_valid();
        self.sync_symmetry_notice();
        // What the last Reset or Freeze skipped (locked items, children
        // that would shear), however it was run.
        if let Some(note) = self.editor.transform_note.take() {
            self.status = note;
        }
        self.keyboard(ctx);
        native_menu::sync(self.menu_model(ctx));
        self.persist_layout(ctx);
        self.journal_when_due(ctx);
        identity_audit::capture(self, ctx);
        self.sync_window_title(ctx);
        log_ring::note_status(&self.status);
        self.end_frame(ctx);
        // After the first frame, so the window is up before any prompt.
        self.launch_update_check(ctx);
        bug_report::test_crash_after_first_frame();
    }
}
/// Scene equality for unsaved-changes and recovery checks, which run
/// several times a frame. A mesh counts as unchanged only when it is the
/// same shared allocation: comparing a large mesh's contents polygon by
/// polygon every frame cost an imported scene (whose baseline holds an
/// equal but separate copy) a large part of each frame. An equal copy reads
/// as changed, which only means a save is offered.
pub(crate) fn same_document(a: &Scene, b: &Scene) -> bool {
    a.items.len() == b.items.len()
        && a.items
            .iter()
            .zip(&b.items)
            .all(|(x, y)| operandi_core::Shared::same(x.mesh.shared(), y.mesh.shared()))
        && a == b
}
#[path = "tools/drag_readout.rs"]
mod drag_readout;
#[path = "window_layout/floating.rs"]
mod floating;
#[path = "window_layout/form_editor.rs"]
mod form_editor;
#[cfg(test)]
#[path = "window_layout/form_editor_pie_tests.rs"]
mod form_editor_pie_tests;
#[cfg(test)]
#[path = "window_layout/form_editor_tests.rs"]
mod form_editor_tests;
#[path = "window_layout/form_icons.rs"]
mod form_icons;
#[cfg(test)]
#[path = "window_layout/form_icons_tests.rs"]
mod form_icons_tests;
#[path = "nodes/graph_editor.rs"]
mod graph_editor;
#[path = "create_tools/guides_ui.rs"]
mod guides_ui;
#[path = "graphics/matcap_render.rs"]
mod matcap_render;
#[path = "nodes/materialx_ui.rs"]
mod materialx_ui;
#[cfg(test)]
#[path = "nodes/materialx_ui_tests.rs"]
mod materialx_ui_tests;
#[cfg(test)]
#[path = "menus/menu_forms_tests.rs"]
mod menu_forms_tests;
#[path = "window_layout/palettes.rs"]
mod palettes;
#[cfg(test)]
#[path = "window_layout/palettes_tests.rs"]
mod palettes_tests;
#[path = "presets/preset_browser.rs"]
mod preset_browser;
#[path = "presets/preset_browser_ui.rs"]
mod preset_browser_ui;
#[path = "presets/preset_thumbs.rs"]
mod preset_thumbs;
#[path = "presets/preset_ui.rs"]
mod preset_ui;
#[path = "render/preview.rs"]
mod preview;
#[path = "render/preview_ui.rs"]
mod preview_ui;
#[path = "create_tools/primitive_panel.rs"]
mod primitive_panel;
#[path = "render/render_ui.rs"]
mod render_ui;
#[path = "anim/timeline.rs"]
mod timeline;
#[path = "items/vertex_normal_ui.rs"]
mod vertex_normal_ui;
#[path = "view/view_display.rs"]
mod view_display;
#[path = "view/view_display_ui.rs"]
mod view_display_ui;
#[path = "view/view_fov.rs"]
mod view_fov;
#[path = "view/view_nav.rs"]
mod view_nav;
#[path = "window_layout/view_tab_presets.rs"]
mod view_tab_presets;
mod viewport;
#[path = "view/viewport_buttons.rs"]
mod viewport_buttons;
#[path = "view/viewport_properties.rs"]
mod viewport_properties;
#[path = "window_layout/views.rs"]
mod views;
/// One "does this scene need saving?" rule for the open scene and the
/// parked ones alike (Astra P8-002: a parked tab's numeric preview was
/// skipped by Quit): the scene differs from the saved one, or an Ellipsoid
/// or shape preview is waiting to be applied.
fn needs_saving(editor: &Editor, saved: &Scene, same: fn(&Scene, &Scene) -> bool) -> bool {
    !same(&editor.scene, saved)
        || editor
            .ellipsoid
            .as_ref()
            .is_some_and(|s| s.preview.is_some())
        || editor.shape.as_ref().is_some_and(|s| s.preview.is_some())
}

/// The launch arguments as text, for the command-line modes below. A
/// file name that isn't valid Unicode (possible on Linux) is kept as it is
/// for opening (`AppEnvironment`) and only shown lossily
/// here; `std::env::args` would panic on it (Astra P8-007).
fn command_line_words(raw: impl Iterator<Item = std::ffi::OsString>) -> Vec<String> {
    raw.map(|a| a.to_string_lossy().into_owned()).collect()
}

fn main() -> eframe::Result {
    let environment = app_environment::AppEnvironment::capture();
    // Crash files go to <data folder>/crashes (OPERANDI_DEV_DATA for QA apps).
    bug_report::install_panic_hook(environment.paths().crashes());
    // Bulk thumbnail re-render (PB-114 look change): no window opens.
    let args = command_line_words(std::env::args_os().skip(1));
    if args.first().map(String::as_str) == Some("--rerender-thumbnails") {
        std::process::exit(preset_thumbs::rerender_command(&args[1..]));
    }
    // The library pack's own mesh presets with rendered thumbnails.
    if args.first().map(String::as_str) == Some("--write-starter-meshes") {
        std::process::exit(preset_thumbs::write_starter_meshes_command(&args[1..]));
    }
    // Operandi's own matcap renders (matcap_render.rs): no window opens.
    if args.first().map(String::as_str) == Some("--render-matcap") {
        std::process::exit(matcap_render::command(&args[1..]));
    }
    #[cfg(feature = "qa")]
    if qa::offscreen_requested() {
        if let Err(e) = qa::offscreen(environment) {
            log_ring::log_line!("QA render failed: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    // The main window opens where it was last closed, unless that place is
    // on no screen now (window_state.rs).
    let saved_window = window_state::MainWindow::load(&environment.paths().main_window());
    let screens = window_state::screens(saved_window.pixels_per_point);
    let window = saved_window.placement(screens.as_deref());
    if saved_window.position.is_some() && window.position.is_none() {
        log_ring::log_line!("The saved window place is off screen; opening at the default place.");
    }
    let mut options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: gpu::wgpu_configuration(),
        viewport: window
            .apply(egui::ViewportBuilder::default())
            .with_app_id("org.operandi.rust.m15.dev"),
        ..Default::default()
    };
    // The logo Dock tile as the window/Dock icon.
    // The full wordmark is reserved for surfaces with enough space. On macOS
    // a runtime icon replaces the bundle's, so inside an .app we leave it to
    // the bundle (Liquid Glass on macOS 26+, the .icns before that). Leaving
    // the icon unset isn't enough: eframe then sets its own egui icon. An
    // empty IconData is eframe's documented "don't set an icon".
    if running_from_macos_app_bundle() {
        options.viewport = options.viewport.with_icon(egui::IconData::default());
    } else {
        match eframe::icon_data::from_png_bytes(branding::DOCK_ICON) {
            Ok(icon) => options.viewport = options.viewport.with_icon(icon),
            Err(error) => log_ring::log_line!("Approved Operandi app icon unavailable: {error}"),
        }
    }
    // Keep the same options on normal builds.
    options.viewport = options.viewport.with_app_id("org.operandi.rust.m15.dev");
    // Before the event loop starts, so a file that launched the app opens.
    #[cfg(target_os = "macos")]
    open_documents::install_before_launch();
    let title = environment.title().to_owned();
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            let mut app = Operandi::new(cc, environment);
            // Start from the restored state, so a window opened maximised
            // keeps its normal size and place for next time.
            app.main_window = window_state::Tracker::new(window);
            Ok(Box::new(app))
        }),
    )
}

/// True when this executable sits in `<name>.app/Contents/MacOS/` on macOS,
/// so the bundle supplies the Dock icon. False for `cargo run` and on other
/// platforms.
fn running_from_macos_app_bundle() -> bool {
    cfg!(target_os = "macos")
        && std::env::current_exe()
            .ok()
            .is_some_and(|exe| is_in_app_bundle(&exe))
}

fn is_in_app_bundle(exe: &std::path::Path) -> bool {
    let mut dirs = exe.ancestors().skip(1);
    let (Some(macos), Some(contents), Some(app)) = (dirs.next(), dirs.next(), dirs.next()) else {
        return false;
    };
    macos.file_name().is_some_and(|n| n == "MacOS")
        && contents.file_name().is_some_and(|n| n == "Contents")
        && app.extension().is_some_and(|e| e == "app")
}

#[cfg(test)]
mod app_bundle_icon_tests {
    use super::is_in_app_bundle;
    use std::path::Path;

    #[test]
    fn bundled_executable_is_detected() {
        assert!(is_in_app_bundle(Path::new(
            "/Users/x/Operandi Builds/Operandi main abc1234.app/Contents/MacOS/operandi-app"
        )));
    }

    #[test]
    fn cargo_target_executable_is_not_bundled() {
        assert!(!is_in_app_bundle(Path::new(
            "/Users/x/Operandi/target/release/operandi-app"
        )));
        assert!(!is_in_app_bundle(Path::new("/Contents/MacOS/operandi-app")));
        assert!(!is_in_app_bundle(Path::new("operandi-app")));
    }
}

#[cfg(test)]
#[path = "transforms/absolute_scaling_tests.rs"]
mod absolute_scaling_tests;
#[cfg(test)]
#[path = "view/active_mesh_pick_tests.rs"]
mod active_mesh_pick_tests;
#[cfg(test)]
mod bevel_gizmo_tests;
#[cfg(test)]
#[path = "window_layout/button_variants_tests.rs"]
mod button_variants_tests;
#[cfg(test)]
#[path = "render/camera_props_tests.rs"]
mod camera_props_tests;
#[cfg(test)]
#[path = "view/camera_view_settle_tests.rs"]
mod camera_view_settle_tests;
#[cfg(test)]
#[path = "tools/click_finish_tests.rs"]
mod click_finish_tests;
#[cfg(test)]
#[path = "tools/click_start_tests.rs"]
mod click_start_tests;
#[cfg(test)]
#[path = "commands/command_history_tests.rs"]
mod command_history_tests;
#[cfg(test)]
#[path = "commands/command_line_tests.rs"]
mod command_line_tests;
#[cfg(test)]
#[path = "commands/command_names_tests.rs"]
mod command_names_tests;
#[cfg(test)]
#[path = "tools/component_transform_tests.rs"]
mod component_transform_tests;
#[cfg(test)]
#[path = "items/create_utility_tests.rs"]
mod create_utility_tests;
#[cfg(test)]
#[path = "deformers/deform_tab_tests.rs"]
mod deform_tab_tests;
#[cfg(test)]
#[path = "tools/drag_anywhere_tests.rs"]
mod drag_anywhere_tests;
#[cfg(test)]
#[path = "graphics/drag_patch_tests.rs"]
mod drag_patch_tests;
#[cfg(test)]
#[path = "graphics/draw_depth_tests.rs"]
mod draw_depth_tests;
#[cfg(test)]
#[path = "create_tools/draw_point_mode_tests.rs"]
mod draw_point_mode_tests;
#[cfg(test)]
#[path = "create_tools/draw_tools_tests.rs"]
mod draw_tools_tests;
#[cfg(test)]
#[path = "graphics/drawing_equivalence_tests.rs"]
mod drawing_equivalence_tests;
#[cfg(test)]
#[path = "tools/drop_action_tests.rs"]
mod drop_action_tests;
#[cfg(test)]
#[path = "files/drop_files_tests.rs"]
mod drop_files_tests;
#[cfg(test)]
#[path = "input_devices/drop_keys_tests.rs"]
mod drop_keys_tests;
#[cfg(test)]
#[path = "polygons/drop_lag_tests.rs"]
mod drop_lag_tests;
#[cfg(test)]
#[path = "duplication/duplicate_tools_tests.rs"]
mod duplicate_tools_tests;
#[cfg(test)]
#[path = "edges/edge_tab_tests.rs"]
mod edge_tab_tests;
#[cfg(test)]
#[path = "tools/empty_expand_tests.rs"]
mod empty_expand_tests;
#[cfg(test)]
#[path = "render/environment_image_tests.rs"]
mod environment_image_tests;
#[cfg(test)]
#[path = "render/environment_preset_tests.rs"]
mod environment_preset_tests;
#[cfg(test)]
#[path = "render/environment_ui_tests.rs"]
mod environment_ui_tests;
#[cfg(test)]
#[path = "view/extend_handles_tests.rs"]
mod extend_handles_tests;
#[cfg(test)]
#[path = "files/file_chooser_tests.rs"]
mod file_chooser_tests;
#[cfg(test)]
#[path = "files/file_error_tests.rs"]
mod file_error_tests;
#[cfg(test)]
#[path = "graphics/grid_depth_tests.rs"]
mod grid_depth_tests;
#[cfg(test)]
mod handle_arrow_tests;
#[cfg(test)]
#[path = "menus/handle_style_menu_tests.rs"]
mod handle_style_menu_tests;
#[cfg(test)]
#[path = "view/handle_tips_tests.rs"]
mod handle_tips_tests;
#[cfg(test)]
#[path = "tools/haul_meshop_tests.rs"]
mod haul_meshop_tests;
#[cfg(test)]
#[path = "tools/haul_session_tests.rs"]
mod haul_session_tests;
#[cfg(test)]
#[path = "tools/haul_tests.rs"]
mod haul_tests;
#[cfg(test)]
#[path = "graphics/heavy_mesh_frames_tests.rs"]
mod heavy_mesh_frames_tests;
#[cfg(test)]
#[path = "settings/history_limit_more_tests.rs"]
mod history_limit_more_tests;
#[cfg(test)]
#[path = "settings/history_limit_tests.rs"]
mod history_limit_tests;
#[cfg(test)]
#[path = "input_devices/input_focus_tests.rs"]
mod input_focus_tests;
#[cfg(test)]
#[path = "polygons/inset_shift_tests.rs"]
mod inset_shift_tests;
#[cfg(test)]
#[path = "items/item_list_header_tests.rs"]
mod item_list_header_tests;
#[cfg(test)]
#[path = "input_devices/key_toggles_tests.rs"]
mod key_toggles_tests;
#[cfg(test)]
#[path = "kits/kit_commands_tests.rs"]
mod kit_commands_tests;
#[cfg(test)]
#[path = "selections/lasso_ui_tests.rs"]
mod lasso_ui_tests;
#[cfg(test)]
#[path = "window_layout/layout_keys_tests.rs"]
mod layout_keys_tests;
#[cfg(test)]
#[path = "window_layout/layout_tests.rs"]
mod layout_tests;
#[cfg(test)]
#[path = "window_layout/layouts_edit_tests.rs"]
mod layouts_edit_tests;
#[cfg(test)]
#[path = "window_layout/layouts_tests.rs"]
mod layouts_tests;
#[cfg(test)]
#[path = "render/light_properties_tests.rs"]
mod light_properties_tests;
#[cfg(test)]
#[path = "render/light_view_tests.rs"]
mod light_view_tests;
#[cfg(test)]
#[path = "tools/link_button_tests.rs"]
mod link_button_tests;
#[cfg(test)]
#[path = "tools/mini_properties_tests.rs"]
mod mini_properties_tests;
#[cfg(test)]
#[path = "controls/number_field_tests.rs"]
mod number_field_tests;
#[cfg(test)]
#[path = "controls/numeric_tab_tests.rs"]
mod numeric_tab_tests;
#[cfg(test)]
#[path = "presets/online_library_tests.rs"]
mod online_library_tests;
#[cfg(test)]
#[path = "slicing/path_slice_tools_tests.rs"]
mod path_slice_tools_tests;
#[cfg(test)]
#[path = "create_tools/pen_ui_tests.rs"]
mod pen_ui_tests;
#[cfg(test)]
#[path = "tools/perf_drags_tests.rs"]
mod perf_drags_tests;
#[cfg(test)]
#[path = "view/perf_view_tests.rs"]
mod perf_view_tests;
#[cfg(test)]
#[path = "render/picture_capture_tests.rs"]
mod picture_capture_tests;
#[cfg(test)]
#[path = "view/placement_snap_tests.rs"]
mod placement_snap_tests;
#[cfg(test)]
#[path = "presets/preset_drop_tests.rs"]
mod preset_drop_tests;
#[cfg(test)]
#[path = "presets/preset_ui_tests.rs"]
mod preset_ui_tests;
#[cfg(test)]
#[path = "create_tools/primitive_falloff_tests.rs"]
mod primitive_falloff_tests;
#[cfg(test)]
#[path = "create_tools/primitive_modifier_tests.rs"]
mod primitive_modifier_tests;
#[cfg(test)]
#[path = "create_tools/primitive_ui_tests.rs"]
mod primitive_ui_tests;
#[cfg(test)]
#[path = "render/render_item_ui_tests.rs"]
mod render_item_ui_tests;
#[cfg(test)]
#[path = "render/render_outputs_save_tests.rs"]
mod render_outputs_save_tests;
#[cfg(test)]
#[path = "tools/reset_tool_tests.rs"]
mod reset_tool_tests;
#[cfg(test)]
#[path = "items/scene_panel_tests.rs"]
mod scene_panel_tests;
#[cfg(test)]
#[path = "files/scenes_tests.rs"]
mod scenes_tests;
#[cfg(test)]
#[path = "controls/scrub_tests.rs"]
mod scrub_tests;
#[cfg(test)]
#[path = "selections/select_tab_tests.rs"]
mod select_tab_tests;
#[cfg(test)]
#[path = "selections/selection_ui_tests.rs"]
mod selection_ui_tests;
#[cfg(test)]
#[path = "nodes/shader_layer_menu_tests.rs"]
mod shader_layer_menu_tests;
#[cfg(test)]
#[path = "materials/shading_panel_tests.rs"]
mod shading_panel_tests;
#[cfg(test)]
#[path = "slicing/slice_tools_tests.rs"]
mod slice_tools_tests;
#[cfg(test)]
#[path = "view/snaps_panel_tests.rs"]
mod snaps_panel_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tools/tool_card_view_tests.rs"]
mod tool_card_view_tests;
#[cfg(test)]
#[path = "tools/tool_field_handoff_tests.rs"]
mod tool_field_handoff_tests;
#[path = "tools/tool_info.rs"]
mod tool_info;
#[cfg(test)]
#[path = "presets/tool_presets_tests.rs"]
mod tool_presets_tests;
#[cfg(test)]
#[path = "tools/tool_properties_header_tests.rs"]
mod tool_properties_header_tests;
#[cfg(test)]
#[path = "tools/tool_speed_guard_tests.rs"]
mod tool_speed_guard_tests;
#[cfg(test)]
#[path = "window_layout/tool_tab_cycle_tests.rs"]
mod tool_tab_cycle_tests;
#[path = "window_layout/toolbox_form.rs"]
mod toolbox_form;
#[cfg(test)]
#[path = "window_layout/toolbox_form_tests.rs"]
mod toolbox_form_tests;
#[cfg(test)]
#[path = "window_layout/toolbox_sets_tests.rs"]
mod toolbox_sets_tests;
#[cfg(test)]
#[path = "window_layout/tooltip_names_tests.rs"]
mod tooltip_names_tests;
#[cfg(test)]
#[path = "deformers/tweak_tests.rs"]
mod tweak_tests;
#[cfg(test)]
#[path = "polygons/twist_bevel_tests.rs"]
mod twist_bevel_tests;
#[cfg(test)]
#[path = "uv/uv_dialogs_tests.rs"]
mod uv_dialogs_tests;
#[cfg(test)]
#[path = "uv/uv_editing_tests.rs"]
mod uv_editing_tests;
#[cfg(test)]
#[path = "render/viewport_transparency_capture_tests.rs"]
mod viewport_transparency_capture_tests;
#[cfg(test)]
#[path = "deformers/weights_distortion_tests.rs"]
mod weights_distortion_tests;
#[path = "window_layout/workbench.rs"]
mod workbench;
#[cfg(test)]
#[path = "window_layout/workbench_tests.rs"]
mod workbench_tests;

#[cfg(test)]
#[path = "render/background_ui_tests.rs"]
mod background_ui_tests;
#[cfg(test)]
#[path = "commands/doors_tests.rs"]
mod doors_tests;
#[cfg(test)]
#[path = "deformers/falloff_batch2_tests.rs"]
mod falloff_batch2_tests;
#[cfg(test)]
#[path = "deformers/falloff_batch3_tests.rs"]
mod falloff_batch3_tests;
#[cfg(test)]
#[path = "deformers/falloff_batch4_tests.rs"]
mod falloff_batch4_tests;
#[cfg(test)]
#[path = "deformers/falloff_batch5_tests.rs"]
mod falloff_batch5_tests;
#[cfg(test)]
#[path = "deformers/falloff_batch6_tests.rs"]
mod falloff_batch6_tests;
#[cfg(test)]
#[path = "deformers/falloff_noise_scale_tests.rs"]
mod falloff_noise_scale_tests;
#[cfg(test)]
#[path = "deformers/falloff_panel_tests.rs"]
mod falloff_panel_tests;
#[cfg(test)]
#[path = "items/mesh_stack_tests.rs"]
mod mesh_stack_tests;
#[cfg(test)]
#[path = "viewport/meshop_handles_tests.rs"]
mod meshop_handles_tests;
#[cfg(test)]
#[path = "view/snap_service_tests.rs"]
mod snap_service_tests;
#[cfg(test)]
#[path = "view/snapping_tests.rs"]
mod snapping_tests;
#[cfg(test)]
#[path = "deformers/soft_move_falloff_tests.rs"]
mod soft_move_falloff_tests;
#[cfg(test)]
#[path = "controls/tree_list_tests.rs"]
mod tree_list_tests;
#[cfg(test)]
#[path = "view/viewport_center_tests.rs"]
mod viewport_center_tests;

#[cfg(test)]
#[path = "polygons/smooth_shift_escape_tests.rs"]
mod smooth_shift_escape_tests;

#[cfg(test)]
#[path = "polygons/smooth_shift_picking_freshness_tests.rs"]
mod smooth_shift_picking_freshness_tests;

#[cfg(test)]
#[path = "tools/performance_tests.rs"]
mod performance_tests;

#[cfg(test)]
#[path = "tools/perf_snapshot_tests.rs"]
mod perf_snapshot_tests;

#[cfg(test)]
#[path = "menus/pie_engine_tests.rs"]
mod pie_engine_tests;
#[cfg(test)]
#[path = "menus/pie_menu_tests.rs"]
mod pie_menu_tests;

#[cfg(test)]
#[path = "files/bridge_recovery_save_tests.rs"]
mod bridge_recovery_save_tests;

#[cfg(test)]
#[path = "polygons/thicken_shell_live_tests.rs"]
mod thicken_shell_live_tests;

#[cfg(test)]
#[path = "deformers/surface_tool_states_live_tests.rs"]
mod surface_tool_states_live_tests;

#[cfg(test)]
#[path = "polygons/extrude_numeric_panel_live_tests.rs"]
mod extrude_numeric_panel_live_tests;

#[cfg(test)]
#[path = "polygons/extrude_spike_chain_tests.rs"]
mod extrude_spike_chain_tests;

#[path = "commands/identity_audit.rs"]
mod identity_audit;

#[cfg(test)]
#[path = "files/new_scene_empty_mesh_tests.rs"]
mod new_scene_empty_mesh_tests;

#[cfg(test)]
#[path = "files/cone_new_document_retention_tests.rs"]
mod cone_new_document_retention_tests;

#[cfg(test)]
#[path = "create_tools/cone_numeric_repeat_history_tests.rs"]
mod cone_numeric_repeat_history_tests;

#[cfg(test)]
#[path = "polygons/polygon_align_tests.rs"]
mod polygon_align_tests;
#[cfg(test)]
#[path = "materials/polygon_material_tests.rs"]
mod polygon_material_tests;
#[cfg(test)]
#[path = "polygons/polygon_part_dialog_tests.rs"]
mod polygon_part_dialog_tests;
#[cfg(test)]
#[path = "polygons/polygon_quadruple_tests.rs"]
mod polygon_quadruple_tests;
#[cfg(test)]
#[path = "polygons/polygon_spin_tests.rs"]
mod polygon_spin_tests;
#[cfg(test)]
#[path = "polygons/polygon_split_tests.rs"]
mod polygon_split_tests;
#[cfg(test)]
#[path = "polygons/polygon_unify_tests.rs"]
mod polygon_unify_tests;

#[cfg(test)]
#[path = "polygons/polygon_triple_tests.rs"]
mod polygon_triple_tests;

#[cfg(test)]
#[path = "topology_tools/topology_mode_tests.rs"]
mod topology_mode_tests;
#[cfg(test)]
#[path = "topology_tools/topology_pen_ui_tests.rs"]
mod topology_pen_ui_tests;
#[cfg(test)]
#[path = "topology_tools/topology_tools_ui_tests.rs"]
mod topology_tools_ui_tests;

#[cfg(test)]
#[path = "polygons/polygon_reduction_tests.rs"]
mod polygon_reduction_tests;

#[cfg(test)]
#[path = "polygons/flip_basic_tests.rs"]
mod flip_basic_tests;
#[cfg(test)]
#[path = "graphics/gl_count_tests.rs"]
mod gl_count_tests;
#[cfg(test)]
#[path = "items/statistics_ui_tests.rs"]
mod statistics_ui_tests;
#[cfg(test)]
#[path = "polygons/subdivision_tests.rs"]
mod subdivision_tests;

#[cfg(test)]
#[path = "polygons/polygon_remove_tests.rs"]
mod polygon_remove_tests;

#[cfg(test)]
#[path = "edges/edge_loop_delete_tests.rs"]
mod edge_loop_delete_tests;

#[cfg(test)]
#[path = "polygons/polygon_collapse_tests.rs"]
mod polygon_collapse_tests;

#[cfg(test)]
#[path = "create_tools/pen_interactive_tests.rs"]
mod pen_interactive_tests;
#[cfg(test)]
#[path = "create_tools/solid_sketch_ui_tests.rs"]
mod solid_sketch_ui_tests;

#[cfg(test)]
#[path = "viewport/handle_tracking_tests.rs"]
mod handle_tracking_tests;
#[cfg(test)]
#[path = "transforms/scale_offset_tests.rs"]
mod scale_offset_tests;
#[cfg(test)]
#[path = "transforms/scale_to_zero_tests.rs"]
mod scale_to_zero_tests;
#[cfg(test)]
#[path = "transforms/set_position_tests.rs"]
mod set_position_tests;

#[cfg(test)]
#[path = "polygons/flatten_ngons_ui_tests.rs"]
mod flatten_ngons_ui_tests;

#[cfg(test)]
#[path = "edges/vertex_panel_tests.rs"]
mod vertex_panel_tests;

#[cfg(test)]
#[path = "slicing/add_loop_slide_options_tests.rs"]
mod add_loop_slide_options_tests;
#[cfg(test)]
#[path = "items/group_eye_tests.rs"]
mod group_eye_tests;
#[cfg(test)]
#[path = "files/journal_tests.rs"]
mod journal_tests;
#[cfg(test)]
#[path = "input_devices/look_around_tests.rs"]
mod look_around_tests;
#[cfg(test)]
#[path = "materials/material_editing_tests.rs"]
mod material_editing_tests;
#[cfg(test)]
#[path = "input_devices/nav_lock_hide_tests.rs"]
mod nav_lock_hide_tests;
#[cfg(test)]
#[path = "transforms/symmetry_tests.rs"]
mod symmetry_tests;
#[cfg(test)]
#[path = "view/zoom_to_surface_tests.rs"]
mod zoom_to_surface_tests;
