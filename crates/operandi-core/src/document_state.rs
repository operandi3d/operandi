//! Which editor state belongs to one open scene (Astra P2-001).
//!
//! The app keeps one `Editor` per scene tab and swaps the scene's own
//! state in and out when tabs switch. That list was written by hand in the
//! app and had gaps: the live tool's tweak history, tool step counts,
//! pending command records, the multi-mesh Extrude state and more stayed
//! behind, so Undo after switching back was refused or lost.
//!
//! Here every `Editor` field is sorted into exactly one of two lists, and
//! [`every_field_is_sorted`] names every field without `..`: a field added
//! to `Editor` doesn't compile until it is put in one of them.
//!
//! - **Document:** this scene's data and everything tied to it (history,
//!   the open tool and its sessions, ids and caches of its items).
//! - **Shared:** app-wide settings and tool context that follow the user
//!   between scenes (presets, falloff and symmetry settings, clipboards,
//!   preferences, one-off status notes, dispatch bookkeeping).
use crate::Editor;
use std::mem::swap;

impl Editor {
    /// Swaps this editor's document state with `other`'s; shared state
    /// stays where it is.
    pub fn swap_document_state(&mut self, other: &mut Editor) {
        let (a, b) = (self, other);
        swap(&mut a.scene, &mut b.scene);
        swap(&mut a.undo, &mut b.undo);
        swap(&mut a.redo, &mut b.redo);
        swap(&mut a.history_serial, &mut b.history_serial);
        swap(&mut a.history_mark, &mut b.history_mark);
        swap(&mut a.pending_records, &mut b.pending_records);
        swap(&mut a.selection_run, &mut b.selection_run);
        swap(&mut a.selection_start, &mut b.selection_start);
        swap(&mut a.next_item_id, &mut b.next_item_id);
        swap(&mut a.identities, &mut b.identities);
        swap(&mut a.i2, &mut b.i2);
        swap(&mut a.anim, &mut b.anim);
        #[cfg(feature = "painting")]
        swap(&mut a.paint, &mut b.paint);
        // The open tool waits in its scene for the scene's return, with its
        // own history and sessions.
        swap(&mut a.live, &mut b.live);
        swap(&mut a.armed, &mut b.armed);
        swap(&mut a.extend_from_click, &mut b.extend_from_click);
        swap(&mut a.preset, &mut b.preset);
        swap(&mut a.inference, &mut b.inference);
        swap(&mut a.tweaks, &mut b.tweaks);
        swap(&mut a.tool_steps, &mut b.tool_steps);
        swap(&mut a.tool_reopen, &mut b.tool_reopen);
        swap(&mut a.tool_redo_floor, &mut b.tool_redo_floor);
        swap(&mut a.multi_fresh, &mut b.multi_fresh);
        swap(&mut a.multi_extrude, &mut b.multi_extrude);
        swap(&mut a.extrude_moved_caps, &mut b.extrude_moved_caps);
        swap(&mut a.pen_draft, &mut b.pen_draft);
        swap(&mut a.solid_sketch, &mut b.solid_sketch);
        swap(&mut a.bridge, &mut b.bridge);
        swap(&mut a.spike, &mut b.spike);
        swap(&mut a.reduction, &mut b.reduction);
        swap(&mut a.ellipsoid, &mut b.ellipsoid);
        swap(&mut a.shape, &mut b.shape);
        swap(&mut a.cone, &mut b.cone);
        // Ids and caches of this scene's items and elements.
        swap(&mut a.gesture_partners, &mut b.gesture_partners);
        swap(&mut a.symmetry_unpaired, &mut b.symmetry_unpaired);
        swap(&mut a.falloff_element, &mut b.falloff_element);
        swap(&mut a.falloff_stamps, &mut b.falloff_stamps);
        swap(&mut a.background_cache, &mut b.background_cache);
        swap(&mut a.meshops, &mut b.meshops);
        swap(&mut a.eval, &mut b.eval);
        swap(&mut a.base_edit, &mut b.base_edit);
        swap(&mut a.meshop_row, &mut b.meshop_row);
    }
}

/// Names every `Editor` field, with no `..`, so a new field must be sorted
/// into the document list above or the shared list here before it builds.
#[allow(dead_code)]
fn every_field_is_sorted(e: Editor) {
    let Editor {
        // Document (swapped above).
        scene: _,
        undo: _,
        redo: _,
        history_serial: _,
        history_mark: _,
        pending_records: _,
        selection_run: _,
        selection_start: _,
        next_item_id: _,
        identities: _,
        i2: _,
        anim: _,
        #[cfg(feature = "painting")]
            paint: _,
        live: _,
        armed: _,
        extend_from_click: _,
        preset: _,
        inference: _,
        tweaks: _,
        tool_steps: _,
        tool_reopen: _,
        tool_redo_floor: _,
        multi_fresh: _,
        multi_extrude: _,
        extrude_moved_caps: _,
        pen_draft: _,
        solid_sketch: _,
        bridge: _,
        spike: _,
        reduction: _,
        ellipsoid: _,
        shape: _,
        cone: _,
        gesture_partners: _,
        symmetry_unpaired: _,
        falloff_element: _,
        falloff_stamps: _,
        background_cache: _,
        meshops: _,
        eval: _,
        base_edit: _,
        meshop_row: _,
        // Shared: settings and tool context that follow the user.
        action_context: _,
        rig_compensation: _,
        symmetry: _,
        symmetry_resume: _,
        polygon_tag_kind: _,
        spatial_falloff: _,
        spatial_falloff_mix: _,
        added_falloffs: _,
        falloff_path: _,
        selected_weight_map: _,
        background_constraint: _,
        background_constraint_preset: _,
        background_view: _,
        history: _,
        auto_activate_flags: _,
        free_memory: _,
        undo_memory_budget: _,
        // Shared: last-used tool options (UIC-95).
        pen_options: _,
        solid_sketch_options: _,
        topology_pen_memory: _,
        bevel_states: _,
        bevel_state: _,
        bevel_profile: _,
        linear_falloff_preset: _,
        radial_falloff_preset: _,
        smooth_shift_preset: _,
        chamfer_preset: _,
        edge_bevel_preset: _,
        edge_chamfer_preset: _,
        poly_haul_preset: _,
        sketch_precision_preset: _,
        vertex_merge_preset: _,
        bend_preset: _,
        thicken_preset: _,
        surface_states: _,
        surface_state: _,
        bridge_preset: _,
        spike_strength: _,
        ellipsoid_preset: _,
        shape_presets: _,
        cone_preset: _,
        extrude_preset: _,
        cube_preset: _,
        sphere_unit_resolution: _,
        cylinder_unit_resolution: _,
        cylinder_shape: _,
        sphere_shape: _,
        primitive_uvs_off: _,
        slice_preset: _,
        subdivide_preset: _,
        unsubdivide_preset: _,
        freeze_preset: _,
        duplicate_preset: _,
        detriangulate_preset: _,
        relax_mesh_preset: _,
        uv_unwrap_preset: _,
        uv_atlas_preset: _,
        uv_island_memory: _,
        uv_texel: _,
        uv_seam_map: _,
        uv_symmetry: _,
        uv_pack_preset: _,
        retopo_preset: _,
        voxel_remesh_preset: _,
        octree_remesh_preset: _,
        boolean_preset: _,
        solid_drill_preset: _,
        axis_drill_preset: _,
        // Shared: app-wide clipboards and caches matched by content.
        clipboard: _,
        uv_map_clipboard: _,
        uv_clipboard: _,
        remesh_prepared: _,
        last_transform_step: _,
        // Shared: bookkeeping of the command running now and one-off
        // status notes the app shows and clears.
        dispatch_depth: _,
        undo_fold: _,
        undo_trim_paused: _,
        tweak_held: _,
        revision: _,
        symmetry_note: _,
        boolean_note: _,
        drill_note: _,
        command_note: _,
        transform_note: _,
    } = e;
}
