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
        swap(&mut a.history.undo, &mut b.history.undo);
        swap(&mut a.history.redo, &mut b.history.redo);
        swap(&mut a.history.history_serial, &mut b.history.history_serial);
        swap(&mut a.history.history_mark, &mut b.history.history_mark);
        swap(
            &mut a.history.pending_records,
            &mut b.history.pending_records,
        );
        swap(&mut a.history.selection_run, &mut b.history.selection_run);
        swap(&mut a.selection_start, &mut b.selection_start);
        swap(&mut a.next_item_id, &mut b.next_item_id);
        swap(&mut a.identities, &mut b.identities);
        swap(&mut a.i2, &mut b.i2);
        swap(&mut a.anim, &mut b.anim);
        #[cfg(feature = "painting")]
        swap(&mut a.paint, &mut b.paint);
        // The open tool waits in its scene for the scene's return, with its
        // own history and sessions.
        swap(&mut a.session.live, &mut b.session.live);
        swap(&mut a.session.armed, &mut b.session.armed);
        swap(
            &mut a.session.extend_from_click,
            &mut b.session.extend_from_click,
        );
        swap(&mut a.session.preset, &mut b.session.preset);
        swap(&mut a.inference, &mut b.inference);
        swap(&mut a.session.tweaks, &mut b.session.tweaks);
        swap(&mut a.history.tool_steps, &mut b.history.tool_steps);
        swap(&mut a.session.tool_reopen, &mut b.session.tool_reopen);
        swap(
            &mut a.history.tool_redo_floor,
            &mut b.history.tool_redo_floor,
        );
        swap(&mut a.multi_fresh, &mut b.multi_fresh);
        swap(&mut a.multi_extrude, &mut b.multi_extrude);
        swap(&mut a.extrude_moved_caps, &mut b.extrude_moved_caps);
        swap(&mut a.session.pen_draft, &mut b.session.pen_draft);
        swap(&mut a.session.solid_sketch, &mut b.session.solid_sketch);
        swap(&mut a.session.bridge, &mut b.session.bridge);
        swap(&mut a.session.spike, &mut b.session.spike);
        swap(&mut a.session.reduction, &mut b.session.reduction);
        swap(&mut a.session.ellipsoid, &mut b.session.ellipsoid);
        swap(&mut a.session.shape, &mut b.session.shape);
        swap(&mut a.session.cone, &mut b.session.cone);
        // Ids and caches of this scene's items and elements.
        swap(
            &mut a.session.gesture_partners,
            &mut b.session.gesture_partners,
        );
        swap(&mut a.symmetry_unpaired, &mut b.symmetry_unpaired);
        swap(&mut a.falloff_element, &mut b.falloff_element);
        swap(&mut a.falloff_stamps, &mut b.falloff_stamps);
        swap(&mut a.background_cache, &mut b.background_cache);
        swap(&mut a.mesh_stack, &mut b.mesh_stack);
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
        // Split field by field in `every_history_field_is_sorted`.
        history: _,
        // Split field by field in `every_session_field_is_sorted`.
        session: _,
        inference: _,
        selection_start: _,
        next_item_id: _,
        identities: _,
        i2: _,
        anim: _,
        #[cfg(feature = "painting")]
            paint: _,
        multi_fresh: _,
        multi_extrude: _,
        extrude_moved_caps: _,
        symmetry_unpaired: _,
        falloff_element: _,
        falloff_stamps: _,
        background_cache: _,
        mesh_stack: _,
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
        auto_activate_flags: _,
        free_memory: _,
        // Shared: last-used tool options (UIC-95).
        presets: _,
        pen_options: _,
        solid_sketch_options: _,
        topology_pen_memory: _,
        bevel_states: _,
        bevel_state: _,
        bevel_profile: _,
        linear_falloff_preset: _,
        radial_falloff_preset: _,
        surface_states: _,
        surface_state: _,
        spike_strength: _,
        sphere_unit_resolution: _,
        cylinder_unit_resolution: _,
        cylinder_shape: _,
        sphere_shape: _,
        primitive_uvs_off: _,
        uv_island_memory: _,
        uv_texel: _,
        uv_seam_map: _,
        uv_symmetry: _,
        // Shared: app-wide clipboards and caches matched by content.
        clipboard: _,
        uv_map_clipboard: _,
        uv_clipboard: _,
        remesh_prepared: _,
        // Shared: bookkeeping of the command running now and one-off
        // status notes the app shows and clears.
        revision: _,
        symmetry_note: _,
        boolean_note: _,
        drill_note: _,
        command_note: _,
        transform_note: _,
    } = e;
}

/// The same for the undo history's fields (`crate::History`): the steps
/// and their counters belong to the scene; the limits the app sets and the
/// bookkeeping of the command running now are shared.
#[allow(dead_code)]
fn every_history_field_is_sorted(h: crate::History) {
    let crate::History {
        // Document (swapped above).
        undo: _,
        redo: _,
        history_serial: _,
        history_mark: _,
        pending_records: _,
        selection_run: _,
        tool_steps: _,
        tool_redo_floor: _,
        // Shared: settings and bookkeeping.
        settings: _,
        undo_memory_budget: _,
        undo_fold: _,
        undo_trim_paused: _,
    } = h;
}

/// The same for the running tool session's fields (`crate::ToolSession`):
/// the open tool and what it holds belong to the scene; the bookkeeping of
/// the command running now and the last transform drag are shared.
#[allow(dead_code)]
fn every_session_field_is_sorted(t: crate::ToolSession) {
    let crate::ToolSession {
        // Document (swapped above).
        live: _,
        armed: _,
        extend_from_click: _,
        preset: _,
        tweaks: _,
        tool_reopen: _,
        gesture_partners: _,
        bridge: _,
        spike: _,
        reduction: _,
        ellipsoid: _,
        shape: _,
        cone: _,
        pen_draft: _,
        solid_sketch: _,
        // Shared: a cache matched by content, and bookkeeping of the
        // command and drag running now.
        last_transform_step: _,
        dispatch_depth: _,
        tweak_held: _,
    } = t;
}
