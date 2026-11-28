//! Undo while a tool is active (Jason, 2026-11-04; mid-tool undo spec v1
//! MU-MID-02..04 where it agrees).
//!
//! - Inside a tool each tweak (a drag, a typed value) is a step of the
//!   tool's own: Undo takes back the last tweak and keeps the tool.
//! - Once the tool is dropped, the whole session is one undo step.
//! - With no tweak left, Undo undoes the tool's activation: it closes and
//!   changes no geometry (MU-MID-02/03). Redo opens it again.
//! - Numeric Apply records a step; Undo takes it back with the tool active.
//! - Leaving a tool after Undo keeps what remains and clears Redo (U8).
use glam::DVec3;
use operandi_core::*;

fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e
}
fn faces(e: &Editor) -> usize {
    e.scene.active().map_or(0, |i| i.mesh.faces.len())
}
fn faces_of(scene: &Scene) -> usize {
    scene.active().map_or(0, |i| i.mesh.faces.len())
}
fn undo(e: &mut Editor) -> HistoryCompletion {
    e.dispatch_history_reported(HistoryRequest::Undo).unwrap()
}
fn redo(e: &mut Editor) -> HistoryCompletion {
    e.dispatch_history_reported(HistoryRequest::Redo).unwrap()
}
fn can(e: &Editor, action: HistoryAction) -> bool {
    e.history_eligibility(action).is_available()
}
/// A held drag: every preview joins one tweak until release.
fn held(e: &mut Editor, previews: impl IntoIterator<Item = Command>) {
    e.session.tweak_held = true;
    for c in previews {
        e.dispatch(c).unwrap();
    }
    e.end_tweak();
}
fn box_params(e: &Editor, x: f64, depth: f64) -> Parameters {
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.position = DVec3::new(x, 0., 0.);
    p.size = DVec3::new(1., 1., depth);
    if let Some(r) = p.radial.as_mut() {
        r.radius = DVec3::splat(0.5);
    }
    p
}
/// Draw a drawn primitive's preview at `x` with one drag.
fn draw(e: &mut Editor, x: f64) {
    let steps = [0.2, 0.6, 1.].map(|d| Command::Preview(box_params(e, x, d)));
    held(e, steps);
}
fn live_tool(e: &Editor) -> Option<Tool> {
    e.session.live.as_ref().map(|l| l.tool)
}
fn closed(label: &str) -> HistoryEffect {
    HistoryEffect::ToolClosed {
        label: label.into(),
    }
}
fn tweak_undone(label: &str) -> HistoryEffect {
    HistoryEffect::TweakUndone {
        label: label.into(),
    }
}

#[test]
fn box_z_drag_undo_keeps_the_box_and_drop_makes_one_step() {
    // Jason's example: drag out a box, pull the Z handle, Undo.
    let mut e = editor();
    let empty = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    draw(&mut e, 0.);
    let drawn = e.scene.clone();
    assert!(faces(&e) > 0);
    let z = [1.5, 2., 3.].map(|d| Command::Preview(box_params(&e, 0., d)));
    held(&mut e, z);
    let pulled = e.scene.clone();
    assert_ne!(pulled, drawn);

    // Undo takes back only the Z drag; the box and the tool stay.
    assert_eq!(
        e.history_eligibility(HistoryAction::Undo).label,
        Some("Cube")
    );
    let c = undo(&mut e);
    assert_eq!(c.effect, tweak_undone("Cube"));
    assert!(!c.foreground_closed);
    assert_eq!(e.scene, drawn);
    assert_eq!(live_tool(&e), Some(Tool::Box));
    assert_eq!(e.session.live.as_ref().unwrap().params.size.z, 1.);
    // Redo puts it back.
    redo(&mut e);
    assert_eq!(e.scene, pulled);
    assert_eq!(e.history.undo.len(), steps);

    // Drop: the whole box is one step, and one Undo removes all of it.
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Cube");
    undo(&mut e);
    assert_eq!(e.scene, empty);
    assert_eq!(live_tool(&e), None);
    redo(&mut e);
    assert_eq!(e.scene, pulled);
}

#[test]
fn drawn_primitive_undo_takes_back_the_draw_then_closes() {
    for tool in [Tool::Box, Tool::Sphere, Tool::Cylinder] {
        let mut e = editor();
        e.dispatch(Command::Activate(tool)).unwrap();
        draw(&mut e, -6.);
        e.dispatch(Command::Drop).unwrap();
        let one = e.scene.clone();
        let steps = e.history.undo.len();

        e.dispatch(Command::Activate(tool)).unwrap();
        draw(&mut e, 3.);
        let preview = e.scene.clone();
        assert!(faces(&e) > faces_of(&one));

        // The draw goes; the tool stays, ready to draw again.
        assert_eq!(undo(&mut e).effect, tweak_undone(tool.label()));
        assert_eq!(e.scene, one);
        assert_eq!(live_tool(&e), Some(tool));
        // Nothing left of this session: the next Undo closes the tool.
        let c = undo(&mut e);
        assert_eq!(c.effect, closed(tool.label()));
        assert!(c.foreground_closed);
        assert_eq!(e.scene, one);
        assert_eq!(e.history.undo.len(), steps);

        // Redo opens the tool, then brings the draw back.
        redo(&mut e);
        assert_eq!(live_tool(&e), Some(tool));
        redo(&mut e);
        assert_eq!(e.scene, preview);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.history.undo.len(), steps + 1);
    }
}

#[test]
fn drawn_primitive_closes_first_then_undo_removes_the_mesh() {
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    let c = undo(&mut e);
    assert_eq!(c.effect, closed("Cube"));
    assert!(e.scene.active().is_some());
    undo(&mut e); // New Mesh
    assert!(e.scene.active().is_none());
    assert!(e.session.live.is_none());
}

#[test]
fn cancel_takes_back_the_whole_session() {
    let mut e = editor();
    let empty = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    draw(&mut e, 0.);
    let z = [Command::Preview(box_params(&e, 0., 2.))];
    held(&mut e, z);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, empty);
    assert!(!can(&e, HistoryAction::Redo));
}

/// MTU-O-U7-Move: a unit cube with its top polygon selected.
fn top_face_cube() -> Editor {
    let mut e = editor();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    let mesh = &e.scene.active().unwrap().mesh;
    let top = mesh
        .faces
        .iter()
        .position(|f| f.vertices.iter().all(|&v| mesh.vertices[v as usize].y > 0.))
        .unwrap() as u32;
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![top];
    e.synchronize_identities().unwrap();
    e
}
fn drag_move(e: &mut Editor, delta: DVec3) {
    let base = e.session.live.as_ref().unwrap().params.clone();
    let frames = [0.5, 1.].map(|t| {
        Command::Preview(Parameters {
            delta: delta * t,
            ..base.clone()
        })
    });
    held(e, frames);
    e.dispatch(Command::CommitTransformStep).unwrap();
}

#[test]
fn move_drags_undo_one_at_a_time_then_close_move() {
    let mut e = top_face_cube();
    let original = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    drag_move(&mut e, DVec3::Y * 0.012);
    let first = e.scene.clone();
    drag_move(&mut e, DVec3::Y * 0.5);
    let second = e.scene.clone();
    assert_eq!(e.history.undo.len(), steps);

    // Each Undo takes back one drag; Move stays at zero (MU-MID-03).
    undo(&mut e);
    assert_eq!(e.scene, first);
    assert_eq!(live_tool(&e), Some(Tool::Move));
    assert_eq!(e.session.live.as_ref().unwrap().params.delta, DVec3::ZERO);
    undo(&mut e);
    assert_eq!(e.scene, original);
    assert_eq!(live_tool(&e), Some(Tool::Move));

    // Then Move closes; cube and top-face selection stay.
    let c = undo(&mut e);
    assert_eq!(c.effect, closed("Move"));
    assert_eq!(live_tool(&e), None);
    assert_eq!(e.scene, original);

    // Redo opens Move again, then the drags come back with Move active.
    redo(&mut e);
    assert_eq!(live_tool(&e), Some(Tool::Move));
    redo(&mut e);
    redo(&mut e);
    assert_eq!(e.scene, second);
    assert!(!can(&e, HistoryAction::Redo));

    // Drop: both drags are one Move step.
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    undo(&mut e);
    assert_eq!(e.scene, original);
}

#[test]
fn leaving_move_after_undo_clears_redo() {
    for (leave, kept) in [(Command::Cancel, false), (Command::Drop, true)] {
        let mut e = top_face_cube();
        let original = e.scene.clone();
        e.dispatch(Command::Activate(Tool::Move)).unwrap();
        drag_move(&mut e, DVec3::Y);
        let first = e.scene.clone();
        drag_move(&mut e, DVec3::X);
        undo(&mut e);
        assert!(can(&e, HistoryAction::Redo));
        e.dispatch(leave).unwrap();
        assert_eq!(live_tool(&e), None);
        // Drop keeps the remaining drag; Cancel takes back the session.
        assert_eq!(e.scene, if kept { first } else { original });
        assert!(!can(&e, HistoryAction::Redo));
    }
}

#[test]
fn unused_activation_is_undone_before_older_geometry() {
    // MTU-O-U3: a dropped Cube and a dropped Sphere, then Cube armed.
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    draw(&mut e, 0.);
    e.dispatch(Command::Drop).unwrap();
    let cube = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Sphere)).unwrap();
    draw(&mut e, 3.);
    e.dispatch(Command::Drop).unwrap();
    let both = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();

    assert_eq!(undo(&mut e).effect, closed("Cube"));
    assert_eq!(e.scene, both);
    undo(&mut e);
    assert_eq!(e.scene, cube);
    assert_eq!(live_tool(&e), None);
}

fn oracle_cone() -> ConeParameters {
    ConeParameters {
        position: DVec3::ZERO,
        size: DVec3::new(0.5, 1., 0.5),
        axis: 1,
        sides: 8,
        segments: 1,
        bottom_cap: ConeCap::Single,
        make_uvs: true,
        ..Default::default()
    }
}
fn sides(e: &Editor) -> u32 {
    e.session
        .cone
        .as_ref()
        .expect("Cone stays active")
        .requested
        .sides
}
fn set_sides(e: &mut Editor, sides: u32) {
    let p = ConeParameters {
        sides,
        ..e.session.cone.as_ref().unwrap().requested.clone()
    };
    e.dispatch(Command::ConfigureCone(p)).unwrap();
}

#[test]
fn cone_value_change_is_a_tweak_and_apply_a_step() {
    let mut e = editor();
    let empty = e.scene.clone();
    *e.presets.get_mut::<operandi_core::ConePreset>() = oracle_cone();
    e.dispatch(Command::ArmCone).unwrap();
    // A value change before Apply: Undo takes it back, Cone stays.
    set_sides(&mut e, 12);
    assert_eq!(undo(&mut e).effect, tweak_undone("Cone"));
    assert_eq!(sides(&e), 8);
    redo(&mut e);
    assert_eq!(sides(&e), 12);
    set_sides(&mut e, 8);

    e.dispatch(Command::Apply).unwrap();
    let one = e.scene.clone();
    set_sides(&mut e, 12);
    e.dispatch(Command::Apply).unwrap();
    let two = e.scene.clone();
    assert!(faces_of(&two) > faces_of(&one));

    // Undo takes back the second Apply with Sides still 12 (U2, first step).
    undo(&mut e);
    assert_eq!(e.scene, one);
    assert_eq!(sides(&e), 12);
    assert!(e.session.cone.as_ref().unwrap().preview.is_none());
    // Then the first Apply; like Ellipsoid, Undo of an Apply takes back
    // geometry only, so Sides stays 12.
    undo(&mut e);
    assert_eq!(e.scene, empty);
    assert_eq!(sides(&e), 12);
    // Earlier tweaks went into that Apply, so Cone closes next.
    assert_eq!(undo(&mut e).effect, closed("Cone"));
    assert!(e.session.cone.is_none());

    for _ in 0..3 {
        redo(&mut e);
    }
    assert_eq!(e.scene, two);
    assert!(!can(&e, HistoryAction::Redo));
}

#[test]
fn leaving_cone_after_undo_keeps_the_first_cone_and_clears_redo() {
    // MTU-O-U8a (Escape) and MTU-O-U8b (Q).
    for leave in [Command::Cancel, Command::Drop] {
        let mut e = editor();
        *e.presets.get_mut::<operandi_core::ConePreset>() = oracle_cone();
        e.dispatch(Command::ArmCone).unwrap();
        e.dispatch(Command::Apply).unwrap();
        let one = e.scene.clone();
        set_sides(&mut e, 12);
        e.dispatch(Command::Apply).unwrap();
        undo(&mut e);
        assert_eq!(e.scene, one);
        assert!(can(&e, HistoryAction::Redo));
        e.dispatch(leave).unwrap();
        assert!(e.session.cone.is_none());
        assert_eq!(e.scene, one);
        assert!(can(&e, HistoryAction::Undo));
        assert!(!can(&e, HistoryAction::Redo));
    }
}

#[test]
fn cone_drag_is_one_tweak_and_a_typed_field_another() {
    let mut e = editor();
    *e.presets.get_mut::<operandi_core::ConePreset>() = oracle_cone();
    e.dispatch(Command::ArmCone).unwrap();
    let frames = (1..=5).map(|i| {
        let r = 0.1 * f64::from(i);
        Command::ConfigureCone(ConeParameters {
            position: DVec3::new(0., r * 1.5, 0.),
            size: DVec3::new(r, r * 1.5, r),
            ..oracle_cone()
        })
    });
    held(&mut e, frames);
    let dragged = e.session.cone.as_ref().unwrap().requested.clone();
    set_sides(&mut e, 16);
    undo(&mut e);
    assert_eq!(e.session.cone.as_ref().unwrap().requested, dragged);
    undo(&mut e);
    assert_eq!(
        e.session.cone.as_ref().unwrap().requested,
        oracle_cone().normalized()
    );
    assert_eq!(undo(&mut e).effect, closed("Cone"));
}

#[test]
fn numeric_shape_undo_steps_through_tweaks_and_applies() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Torus)).unwrap();
    let values = e.session.shape.as_ref().unwrap().requested.clone();
    e.dispatch(Command::Apply).unwrap();
    let one = e.scene.clone();
    e.dispatch(Command::Apply).unwrap();
    let two = e.scene.clone();

    // Undo removes the second torus; the tool stays with its values.
    undo(&mut e);
    assert_eq!(e.scene, one);
    assert_eq!(
        e.session.shape.as_ref().expect("Torus stays").requested,
        values
    );
    redo(&mut e);
    assert_eq!(e.scene, two);

    // A value change is a tweak: Undo takes it back and keeps Torus.
    let ShapeParameters::Torus(mut t) = values.clone() else {
        panic!("torus")
    };
    t.sides += 4;
    e.dispatch(Command::ConfigureShape(ShapeParameters::Torus(t)))
        .unwrap();
    assert_eq!(undo(&mut e).effect, tweak_undone("Torus"));
    assert_eq!(e.scene, two);
    assert_eq!(e.session.shape.as_ref().unwrap().requested, values);

    // Cancel after Undo only closes the tool.
    undo(&mut e);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, one);
    assert!(e.session.shape.is_none());
}

#[test]
fn numeric_shape_closes_on_undo_then_its_mesh_goes() {
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    // Arming shows nothing (Jason, 2026-11-05), so the first Undo closes it.
    assert!(e.session.shape.is_some());
    assert_eq!(undo(&mut e).effect, closed("Capsule"));
    assert!(e.session.shape.is_none());
    undo(&mut e); // New Mesh
    assert!(e.scene.active().is_none());
}

#[test]
fn ellipsoid_undo_takes_back_a_value_then_closes() {
    let mut e = editor();
    e.dispatch(Command::Rename("renamed".into())).unwrap();
    e.dispatch(Command::ArmEllipsoid).unwrap();
    let armed = e.session.ellipsoid.as_ref().unwrap().requested.clone();
    let values = EllipsoidParameters {
        radius: DVec3::splat(0.5),
        ..armed.clone()
    };
    e.dispatch(Command::ConfigureEllipsoid(values)).unwrap();
    assert_eq!(undo(&mut e).effect, tweak_undone("Ellipsoid"));
    assert_eq!(e.session.ellipsoid.as_ref().unwrap().requested, armed);
    e.dispatch(Command::Apply).unwrap();
    assert!(faces(&e) > 0);
    undo(&mut e);
    assert_eq!(faces(&e), 0);
    assert!(e.session.ellipsoid.is_some());
    assert_eq!(undo(&mut e).effect, closed("Ellipsoid"));
    assert_eq!(e.scene.active().unwrap().name, "renamed");
}

#[test]
fn activating_a_tool_keeps_redo() {
    // Provisional (MTU-Q12): activation doesn't clear Redo; the undone
    // Rename comes back with Cone still armed.
    let mut e = editor();
    e.dispatch(Command::Rename("renamed".into())).unwrap();
    undo(&mut e);
    e.dispatch(Command::ArmCone).unwrap();
    assert!(can(&e, HistoryAction::Redo));
    redo(&mut e);
    assert_eq!(e.scene.active().unwrap().name, "renamed");
    assert!(e.session.cone.is_some());
}

#[test]
fn undo_and_redo_mid_tool_save_and_reopen_to_the_same_scene() {
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    draw(&mut e, 0.);
    let z = [Command::Preview(box_params(&e, 0., 2.))];
    held(&mut e, z);
    undo(&mut e);
    redo(&mut e);
    undo(&mut e);
    let expected = e.scene.clone();
    let path = std::env::temp_dir().join(format!(
        "operandi-mid-tool-undo-{}.opscene",
        std::process::id()
    ));
    // Save drops the tool and keeps what it shows.
    e.save_committed(&path).unwrap();
    let reopened = persistence::load(&path).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(
        reopened.active().unwrap().mesh.vertices,
        expected.active().unwrap().mesh.vertices
    );
    assert_eq!(
        reopened.active().unwrap().mesh.faces.len(),
        expected.active().unwrap().mesh.faces.len()
    );
}

#[test]
fn bevel_drags_undo_one_at_a_time_and_drop_is_one_step() {
    // Jason, 2026-11-04: the same rule for every tool, Bevel included.
    let mut e = top_face_cube();
    let original = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let start = e.scene.clone();
    let base = e.session.live.as_ref().unwrap().params.clone();
    let inset = [0.05, 0.1, 0.15].map(|i| {
        Command::Preview(Parameters {
            inset: i,
            ..base.clone()
        })
    });
    held(&mut e, inset);
    let inset_only = e.scene.clone();
    assert_ne!(inset_only, start);
    let shift = [0.1, 0.2, 0.3].map(|s| {
        Command::Preview(Parameters {
            inset: 0.15,
            shift: s,
            ..base.clone()
        })
    });
    held(&mut e, shift);
    let both = e.scene.clone();
    assert_ne!(both, inset_only);

    // Each Undo takes back one whole drag; Bevel stays.
    assert_eq!(undo(&mut e).effect, tweak_undone("Bevel"));
    assert_eq!(e.scene, inset_only);
    assert_eq!(live_tool(&e), Some(Tool::Bevel));
    undo(&mut e);
    assert_eq!(e.scene, start);
    assert_eq!(live_tool(&e), Some(Tool::Bevel));
    redo(&mut e);
    redo(&mut e);
    assert_eq!(e.scene, both);

    // Drop: one Bevel step, undone in one go.
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.history.undo.len(), steps + 1);
    undo(&mut e);
    assert_eq!(e.scene, original);
    assert_eq!(live_tool(&e), None);
}

#[test]
fn bevel_cancel_after_undo_takes_back_the_session() {
    let mut e = top_face_cube();
    let original = e.scene.clone();
    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let base = e.session.live.as_ref().unwrap().params.clone();
    for inset in [0.1, 0.2] {
        held(
            &mut e,
            [Command::Preview(Parameters {
                inset,
                ..base.clone()
            })],
        );
    }
    undo(&mut e);
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(e.scene, original);
    assert_eq!(live_tool(&e), None);
    assert!(!can(&e, HistoryAction::Redo));
}

#[test]
fn spike_value_change_undoes_and_keeps_spike() {
    let mut e = top_face_cube();
    e.dispatch(Command::ArmSpike).unwrap();
    let before = e.scene.clone();
    e.dispatch(Command::ConfigureSpike(0.5)).unwrap();
    let spiked = e.scene.clone();
    assert_eq!(undo(&mut e).effect, tweak_undone("Spike"));
    assert_eq!(e.scene, before);
    assert!(e.session.spike.is_some());
    redo(&mut e);
    assert_eq!(e.scene, spiked);
}
