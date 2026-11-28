//! Drop keeps what a tool shows, and picking another tool drops the open one
//! first (Jason, 2026-11-05).
//!
//! - Cone: Drop with a preview keeps it as one "Cone" step; one Undo
//!   removes it and the tool stays closed. Armed with no preview, Drop
//!   records nothing.
//! - Every primitive's Drop keeps its preview.
//! - Starting any tool while another is open drops the open one (its
//!   geometry is one undo step) and starts the new one. A Drop that fails
//!   (a field error) refuses the switch and changes nothing.
use glam::DVec3;
use operandi_core::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum T {
    Box,
    Sphere,
    Cylinder,
    Cone,
    Ellipsoid,
    Torus,
    Capsule,
    Arc,
    Tube,
    Move,
    Bevel,
    Pen,
    Spike,
    Reduction,
}
const PRIMITIVES: [T; 9] = [
    T::Box,
    T::Sphere,
    T::Cylinder,
    T::Cone,
    T::Ellipsoid,
    T::Torus,
    T::Capsule,
    T::Arc,
    T::Tube,
];
const COMPONENT: [T; 5] = [T::Move, T::Bevel, T::Pen, T::Spike, T::Reduction];

fn start(t: T) -> Command {
    match t {
        T::Box => Command::Activate(Tool::Box),
        T::Sphere => Command::Activate(Tool::Sphere),
        T::Cylinder => Command::Activate(Tool::Cylinder),
        T::Cone => Command::ArmCone,
        T::Ellipsoid => Command::ArmEllipsoid,
        T::Torus => Command::ArmShape(ShapeKind::Torus),
        T::Capsule => Command::ArmShape(ShapeKind::Capsule),
        T::Arc => Command::ArmShape(ShapeKind::Arc),
        T::Tube => Command::ArmShape(ShapeKind::Tube),
        T::Move => Command::Activate(Tool::Move),
        T::Bevel => Command::Activate(Tool::Bevel),
        T::Pen => Command::PenStart,
        T::Spike => Command::ArmSpike,
        T::Reduction => Command::ArmReduction,
    }
}
fn task(t: T) -> ActiveTask {
    match t {
        T::Box => ActiveTask::Live(Tool::Box),
        T::Sphere => ActiveTask::Live(Tool::Sphere),
        T::Cylinder => ActiveTask::Live(Tool::Cylinder),
        T::Cone => ActiveTask::Cone,
        T::Ellipsoid => ActiveTask::Ellipsoid,
        T::Torus => ActiveTask::Shape(ShapeKind::Torus),
        T::Capsule => ActiveTask::Shape(ShapeKind::Capsule),
        T::Arc => ActiveTask::Shape(ShapeKind::Arc),
        T::Tube => ActiveTask::Shape(ShapeKind::Tube),
        T::Move => ActiveTask::Live(Tool::Move),
        T::Bevel => ActiveTask::Live(Tool::Bevel),
        T::Pen => ActiveTask::Pen,
        T::Spike => ActiveTask::Spike,
        T::Reduction => ActiveTask::Reduction,
    }
}

/// A unit cube with its top polygon selected, in Polygon mode, so the
/// component tools have something to work on.
fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
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
fn faces(e: &Editor) -> usize {
    e.scene.active().map_or(0, |i| i.mesh.faces.len())
}

/// Show tool `t`'s preview (or make its edit), as a viewport drag or a
/// typed value does.
fn draw(e: &mut Editor, t: T) {
    let x = 3.;
    match t {
        T::Box | T::Sphere | T::Cylinder | T::Move | T::Bevel => {
            let mut p = e.session.live.as_ref().unwrap().params.clone();
            match t {
                T::Move => p.delta = DVec3::Y * 0.5,
                T::Bevel => p.inset = 0.1,
                _ => {
                    p.position = DVec3::new(x, 0., 0.);
                    p.size = DVec3::ONE;
                    if let Some(r) = p.radial.as_mut() {
                        r.radius = DVec3::splat(0.5);
                    }
                }
            }
            e.session.tweak_held = true;
            e.dispatch(Command::Preview(p)).unwrap();
            e.end_tweak();
        }
        T::Cone => e
            .dispatch(Command::ConfigureCone(ConeParameters {
                position: DVec3::new(x, 0., 0.),
                size: DVec3::new(0.5, 1., 0.5),
                sides: 8,
                segments: 1,
                ..Default::default()
            }))
            .unwrap(),
        T::Ellipsoid => {
            let p = EllipsoidParameters {
                radius: DVec3::splat(0.5),
                ..e.session.ellipsoid.as_ref().unwrap().requested.clone()
            };
            e.dispatch(Command::ConfigureEllipsoid(p)).unwrap();
        }
        // Arming Torus, Capsule or Arc shows nothing (Jason, 2026-11-05);
        // place the remembered shape the way a viewport drag or panel edit
        // would.
        T::Torus | T::Capsule | T::Arc => {
            let p = e.session.shape.as_ref().unwrap().requested.clone();
            e.dispatch(Command::ConfigureShape(p)).unwrap();
        }
        T::Tube => e
            .dispatch(Command::ConfigureShape(ShapeParameters::Tube(
                TubeParameters {
                    points: vec![DVec3::new(x, 0., 0.), DVec3::new(x + 1., 0., 0.)],
                    sides: 4,
                    segments: 2,
                    radius: 0.25,
                    ..Default::default()
                },
            )))
            .unwrap(),
        T::Pen => {
            for p in [
                DVec3::new(x, 0., 0.),
                DVec3::new(x + 1., 0., 0.),
                DVec3::new(x, 1., 0.),
            ] {
                e.dispatch(Command::PenCapture(p)).unwrap();
            }
        }
        // No preview by design (numeric Apply tools).
        T::Spike | T::Reduction => {}
    }
}

#[test]
fn cone_drop_keeps_the_drawn_cone_and_one_undo_removes_it() {
    let mut e = editor();
    let base = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(Command::ArmCone).unwrap();
    // The viewport draw sends ConfigureCone frames during a held drag.
    e.session.tweak_held = true;
    for r in [0.2, 0.4, 0.6] {
        e.dispatch(Command::ConfigureCone(ConeParameters {
            size: DVec3::new(r, 1., r),
            ..Default::default()
        }))
        .unwrap();
    }
    e.end_tweak();
    let shown = e.cone_preview_scene().unwrap().unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.cone.is_none());
    assert_eq!(e.scene, shown);
    assert!(faces(&e) > faces_of(&base));
    assert_eq!(e.history.undo.len(), steps + 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Cone");
    // The preset keeps the dropped values.
    assert_eq!(
        e.presets.get::<operandi_core::ConePreset>().size,
        DVec3::new(0.6, 1., 0.6)
    );

    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, base);
    assert!(
        e.session.cone.is_none(),
        "Undo after Drop keeps the tool closed"
    );
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, shown);
    assert!(e.session.cone.is_none());
}

#[test]
fn armed_cone_drop_with_no_preview_is_neutral() {
    let mut e = editor();
    let before = (e.scene.clone(), e.history.undo.len());
    e.dispatch(Command::ArmCone).unwrap();
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.cone.is_none());
    assert_eq!((e.scene.clone(), e.history.undo.len()), before);
}

#[test]
fn cone_drop_with_a_field_error_refuses_and_keeps_the_session() {
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    let invalid = ConeParameters {
        gaps: 50.,
        ..e.session.cone.as_ref().unwrap().requested.clone()
    };
    assert!(e.dispatch(Command::ConfigureCone(invalid)).is_err());
    let before = e.clone();
    assert!(e.dispatch(Command::Drop).is_err());
    assert!(e == before);
}

#[test]
fn every_primitive_drop_keeps_its_preview_as_one_step() {
    for t in PRIMITIVES {
        let mut e = editor();
        let base = e.scene.clone();
        let steps = e.history.undo.len();
        e.dispatch(start(t)).unwrap();
        draw(&mut e, t);
        e.dispatch(Command::Drop).unwrap();
        assert_eq!(e.session.active_task(), None, "{t:?}");
        assert!(faces(&e) > faces_of(&base), "{t:?} kept nothing");
        assert_eq!(e.history.undo.len(), steps + 1, "{t:?}");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base, "{t:?}");
        assert_eq!(e.session.active_task(), None, "{t:?} reopened on Undo");
    }
}
fn faces_of(scene: &Scene) -> usize {
    scene.active().map_or(0, |i| i.mesh.faces.len())
}

/// Tool `a` open with its preview drawn; starting `b` keeps `a`'s geometry
/// as one step and opens `b`.
fn check_switch(a: T, b: T) {
    let mut e = editor();
    let base = e.scene.clone();
    let steps = e.history.undo.len();
    e.dispatch(start(a)).unwrap();
    draw(&mut e, a);
    let draws = !matches!(a, T::Spike | T::Reduction);
    e.dispatch(start(b))
        .unwrap_or_else(|err| panic!("{a:?} -> {b:?}: {err}"));
    assert_eq!(e.session.active_task(), Some(task(b)), "{a:?} -> {b:?}");
    if draws {
        assert_eq!(e.history.undo.len(), steps + 1, "{a:?} -> {b:?}");
        let step = e.history.undo.last().unwrap();
        assert_eq!(step.before, base, "{a:?} -> {b:?}");
        assert_ne!(step.after, base, "{a:?} -> {b:?}: geometry lost");
    } else {
        assert_eq!(e.history.undo.len(), steps, "{a:?} -> {b:?}");
    }
    if draws && !matches!(b, T::Pen | T::Reduction) {
        // Undo closes b (a shape armed with its preview shown discards
        // that preview first), then removes a's work in one go.
        for _ in 0..2 {
            if e.session.active_task().is_some() {
                e.dispatch(Command::Undo).unwrap();
            }
        }
        assert_eq!(e.session.active_task(), None, "{a:?} -> {b:?}");
        assert_eq!(e.history.undo.len(), steps + 1, "{a:?} -> {b:?}");
        e.dispatch(Command::Undo).unwrap();
        assert_eq!(e.scene, base, "{a:?} -> {b:?}");
    }
}

#[test]
fn switching_between_primitives_drops_the_open_one() {
    for a in PRIMITIVES {
        for b in PRIMITIVES {
            if a != b {
                check_switch(a, b);
            }
        }
    }
}

#[test]
fn switching_between_primitives_and_component_tools_drops_the_open_one() {
    for a in [T::Box, T::Cone, T::Ellipsoid, T::Torus] {
        for b in COMPONENT {
            check_switch(a, b);
        }
    }
    for a in COMPONENT {
        for b in [T::Box, T::Cone, T::Ellipsoid, T::Torus, T::Move, T::Bevel] {
            if a != b {
                check_switch(a, b);
            }
        }
    }
    // Pen and Reduction, which used to refuse each other.
    check_switch(T::Reduction, T::Pen);
    check_switch(T::Pen, T::Reduction);
}

#[test]
fn the_open_tools_own_button_drops_it() {
    for t in PRIMITIVES.into_iter().chain([T::Move, T::Bevel, T::Spike]) {
        let mut e = editor();
        let steps = e.history.undo.len();
        e.dispatch(start(t)).unwrap();
        draw(&mut e, t);
        e.dispatch(start(t)).unwrap();
        assert_eq!(e.session.active_task(), None, "{t:?}");
        let expected = steps + usize::from(t != T::Spike);
        assert_eq!(e.history.undo.len(), expected, "{t:?}");
    }
    // Pen Start with a draft open keeps drawing.
    let mut e = editor();
    e.dispatch(Command::PenStart).unwrap();
    draw(&mut e, T::Pen);
    let before = (
        e.session.pen_draft.clone(),
        e.scene.clone(),
        e.history.undo.len(),
    );
    e.dispatch(Command::PenStart).unwrap();
    assert_eq!(
        (
            e.session.pen_draft.clone(),
            e.scene.clone(),
            e.history.undo.len()
        ),
        before
    );
}

#[test]
fn switching_from_a_tool_with_a_field_error_changes_nothing() {
    // Cone with an invalid field.
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    let invalid = ConeParameters {
        gaps: 50.,
        ..e.session.cone.as_ref().unwrap().requested.clone()
    };
    assert!(e.dispatch(Command::ConfigureCone(invalid)).is_err());
    for b in [T::Box, T::Ellipsoid, T::Torus, T::Move, T::Pen, T::Spike] {
        let before = e.clone();
        assert!(e.dispatch(start(b)).is_err(), "Cone -> {b:?}");
        assert!(e == before, "Cone -> {b:?}");
    }
    // Capsule with an invalid radius.
    let mut e = editor();
    e.dispatch(Command::ArmShape(ShapeKind::Capsule)).unwrap();
    let bad = CapsuleParameters {
        radius: DVec3::new(0., 0.75, 0.25),
        ..Default::default()
    };
    assert!(
        e.dispatch(Command::ConfigureShape(ShapeParameters::Capsule(bad)))
            .is_err()
    );
    for b in [T::Cone, T::Box, T::Bevel, T::Reduction] {
        let before = e.clone();
        assert!(e.dispatch(start(b)).is_err(), "Capsule -> {b:?}");
        assert!(e == before, "Capsule -> {b:?}");
    }
}

#[test]
fn a_new_tool_that_cannot_start_leaves_the_open_one() {
    // Ellipsoid needs the None or World action center; Cone does not.
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    e.action_context.center = CenterMode::Origin;
    e.dispatch(Command::ConfigureCone(
        e.session.cone.as_ref().unwrap().requested.clone(),
    ))
    .unwrap();
    let before = e.clone();
    assert!(e.dispatch(Command::ArmEllipsoid).is_err());
    assert!(e == before);
}

#[test]
fn unit_insertion_drops_the_open_tool_first() {
    let units = [
        |d| Command::InsertUnit {
            tool: Tool::Box,
            destination: d,
        },
        Command::InsertEllipsoidUnit,
    ];
    for a in [T::Box, T::Cone, T::Ellipsoid, T::Torus] {
        for (u, unit) in units.into_iter().enumerate() {
            for destination in [UnitDestination::ExistingMesh, UnitDestination::NewMesh] {
                let mut e = editor();
                let base = e.scene.clone();
                let steps = e.history.undo.len();
                e.dispatch(start(a)).unwrap();
                draw(&mut e, a);
                let cmd = unit(destination);
                e.dispatch(cmd.clone())
                    .unwrap_or_else(|err| panic!("{a:?} unit {u} {destination:?}: {err}"));
                assert_eq!(
                    e.session.active_task(),
                    None,
                    "{a:?} unit {u} {destination:?}"
                );
                // The tool's work, then the unit: two steps.
                assert_eq!(
                    e.history.undo.len(),
                    steps + 2,
                    "{a:?} unit {u} {destination:?}"
                );
                let tool_step = &e.history.undo[steps];
                assert_eq!(tool_step.before, base, "{a:?} unit {u} {destination:?}");
                assert_ne!(tool_step.after, base, "{a:?} unit {u} {destination:?}");
                let items = usize::from(destination == UnitDestination::NewMesh);
                assert_eq!(e.scene.items.len(), base.items.len() + items);
                e.dispatch(Command::Undo).unwrap();
                assert_eq!(
                    e.scene, e.history.undo[steps].after,
                    "{a:?} unit {u} {destination:?}"
                );
                e.dispatch(Command::Undo).unwrap();
                assert_eq!(e.scene, base, "{a:?} unit {u} {destination:?}");
            }
        }
    }
}

#[test]
fn unit_insertion_refuses_when_the_open_tool_cannot_drop_or_selection_is_open() {
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    let invalid = ConeParameters {
        gaps: 50.,
        ..e.session.cone.as_ref().unwrap().requested.clone()
    };
    assert!(e.dispatch(Command::ConfigureCone(invalid)).is_err());
    for destination in [UnitDestination::ExistingMesh, UnitDestination::NewMesh] {
        for cmd in [
            Command::InsertUnit {
                tool: Tool::Sphere,
                destination,
            },
            Command::InsertEllipsoidUnit(destination),
        ] {
            let before = e.clone();
            assert!(e.dispatch(cmd).is_err());
            assert!(e == before);
        }
    }
    let mut e = editor();
    e.dispatch(Command::BeginSelection).unwrap();
    let before = e.clone();
    assert!(
        e.dispatch(Command::InsertUnit {
            tool: Tool::Box,
            destination: UnitDestination::ExistingMesh,
        })
        .is_err()
    );
    assert!(e == before);
}

#[test]
fn cone_apply_is_a_plain_step_and_undo_never_reopens_cone() {
    // Apply, Drop, Undo: the applied Cone goes and the tool stays closed.
    let mut e = editor();
    let base = e.scene.clone();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    e.dispatch(Command::Apply).unwrap();
    let applied = e.scene.clone();
    assert_eq!(e.history.undo.last().unwrap().label, "Cone");
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, base);
    assert!(e.session.cone.is_none());
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, applied);
    assert!(e.session.cone.is_none());

    // With Cone still open, Undo of an Apply keeps it open with its
    // current values, like Ellipsoid.
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    let values = e.session.cone.as_ref().unwrap().requested.clone();
    e.dispatch(Command::Apply).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, base);
    assert_eq!(e.session.cone.as_ref().unwrap().requested, values);
}

/// Changing selection mode (1/2/3/4) with a tool open drops the tool first,
/// the same rule as picking another tool (Jason, 2026-11-05): Polygon mode,
/// Move open, press 2, and you land in Edge mode with Move dropped.
#[test]
fn changing_mode_drops_the_open_tool_first() {
    let all = PRIMITIVES.into_iter().chain(COMPONENT);
    for t in all {
        for mode in [Mode::Edge, Mode::Vertex, Mode::Item, Mode::Polygon] {
            let mut e = editor();
            let base = e.scene.clone();
            let steps = e.history.undo.len();
            e.dispatch(start(t)).unwrap();
            draw(&mut e, t);
            e.dispatch(Command::Mode(mode))
                .unwrap_or_else(|err| panic!("{t:?} -> {mode:?}: {err}"));
            assert_eq!(e.session.active_task(), None, "{t:?} -> {mode:?}");
            assert!(!e.i2_active(), "{t:?} -> {mode:?}");
            assert_eq!(e.scene.selection.mode, mode, "{t:?} -> {mode:?}");
            if matches!(t, T::Spike | T::Reduction) {
                assert_eq!(e.history.undo.len(), steps, "{t:?} -> {mode:?}");
            } else {
                // The tool's work is kept as one undo step.
                assert_eq!(e.history.undo.len(), steps + 1, "{t:?} -> {mode:?}");
                let step = e.history.undo.last().unwrap();
                assert_eq!(step.before.items, base.items, "{t:?} -> {mode:?}");
                assert_ne!(step.after.items, base.items, "{t:?} -> {mode:?}");
            }
        }
    }
}

#[test]
fn changing_mode_drops_rotate_and_scale_too() {
    for tool in [Tool::Move, Tool::Rotate, Tool::Scale] {
        let mut e = editor();
        e.dispatch(Command::Activate(tool)).unwrap();
        assert_eq!(
            e.mode_route_eligibility(),
            ModeRouteEligibility::DropsToolFirst
        );
        e.dispatch(Command::Mode(Mode::Edge)).unwrap();
        assert!(e.session.live.is_none(), "{tool:?}");
        assert_eq!(e.scene.selection.mode, Mode::Edge, "{tool:?}");
        assert_eq!(e.mode_route_eligibility(), ModeRouteEligibility::Direct);
    }
}

#[test]
fn changing_mode_from_a_tool_that_cannot_drop_changes_nothing() {
    // Cone with an invalid field.
    let mut e = editor();
    e.dispatch(Command::ArmCone).unwrap();
    draw(&mut e, T::Cone);
    let invalid = ConeParameters {
        gaps: 50.,
        ..e.session.cone.as_ref().unwrap().requested.clone()
    };
    assert!(e.dispatch(Command::ConfigureCone(invalid)).is_err());
    let before = e.clone();
    assert!(e.dispatch(Command::Mode(Mode::Edge)).is_err());
    assert!(e == before);
    // A Pen draft with no points yet: Drop asks for one, so the mode stays.
    let mut e = editor();
    e.dispatch(Command::PenStart).unwrap();
    let before = e.clone();
    let err = e.dispatch(Command::Mode(Mode::Edge)).unwrap_err();
    assert!(err.contains("Pen"), "{err}");
    assert!(e == before);
}
