//! Click to start (tool-click-start-2026-11-16): the Auto-Activate flag
//! table, the armed state (no change, Apply once, Drop a no-op) and the
//! first click's order (centre, axis, falloff, the tool's own setup).
//! Original test data: a 2 m box.
use glam::DVec3;
use operandi_core::tool_preset::{auto_activate, tool_flags};
use operandi_core::*;

/// A 2 m box at the origin in Vertex mode, flags honoured as in the app.
fn editor() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::splat(2.), 0)
        .unwrap()
        .into();
    e.scene.selection.mode = Mode::Vertex;
    e.history.undo.clear();
    e.auto_activate_flags = true;
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn params(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn bent(e: &mut Editor) {
    let mut p = params(e);
    p.bend.angle = 90.;
    e.dispatch(Command::Preview(p)).unwrap();
}

/// CS-01/05/07: the flag table, plain tools and preset rows.
#[test]
fn cs05_the_flag_table() {
    for t in [
        Tool::Bend,
        Tool::Push,
        Tool::Smooth,
        Tool::Jitter,
        Tool::Quantize,
        Tool::RadialAlign,
        Tool::LinearAlign,
        Tool::Extrude,
        Tool::SmoothShift,
        Tool::EdgeExtend,
        Tool::Box,
        Tool::Mirror,
        Tool::Array,
        Tool::Clone,
        Tool::RadialArray,
        Tool::Lathe,
        Tool::EdgeSlice,
        Tool::PenSlice,
        Tool::CurveSlice,
    ] {
        assert!(!auto_activate(t, None), "{t:?} waits for a click");
    }
    // CS-07 (CS-Q01 decided 2026-11-16, an approved deviation).
    assert!(
        !auto_activate(Tool::EdgeChamfer, None),
        "Edge Chamfer waits for a click (CS-07)"
    );
    for t in [
        // CS-Q01 revisited 2026-11-23: the Bevels show their gizmo at once.
        Tool::Bevel,
        Tool::EdgeBevel,
        Tool::VertexBevel,
        Tool::Move,
        Tool::Rotate,
        Tool::Scale,
        Tool::Transform,
        Tool::VertexExtrude,
        Tool::LoopSlice,
        Tool::EdgeSlide,
        Tool::Chamfer,
    ] {
        assert!(auto_activate(t, None), "{t:?} starts at once");
    }
    // Rows override their actor's flag: Shear is Move, Taper is Scale.
    assert!(!auto_activate(Tool::Move, Some("xfrm.shear")));
    assert!(!auto_activate(Tool::Scale, Some("xfrm.taper")));
    assert!(!auto_activate(Tool::Push, Some("Flare")));
    assert!(!auto_activate(Tool::Tweak, Some("ElementMove")));
    assert!(auto_activate(Tool::Rotate, Some("xfrm.twist")));
    assert!(auto_activate(Tool::Transform, Some("Flex")));
    assert!(auto_activate(Tool::Move, Some("TransformMove")));
    assert!(!tool_flags().is_empty());
}

/// CS-01/22: picking a flagged tool arms it: the mesh doesn't change,
/// panel values are kept but not shown, and dropping it is not a step.
#[test]
fn cs22_armed_changes_nothing_and_drop_is_a_no_op() {
    let mut e = editor();
    let before = points(&e);
    *e.presets.get_mut::<operandi_core::bend::BendPreset>() = Some(BendOptions {
        angle: 45.,
        ..Default::default()
    });
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert_eq!(e.armed_tool(), Some(Tool::Bend));
    assert_eq!(points(&e), before, "a remembered Angle isn't shown");
    assert_eq!(
        e.armed_status().unwrap(),
        "Click in a view to start Bend, or set values and press Apply."
    );
    bent(&mut e);
    assert_eq!(params(&e).bend.angle, 90., "the panel's value is kept");
    assert_eq!(points(&e), before, "but nothing changes");
    e.dispatch(Command::Drop).unwrap();
    assert!(e.session.live.is_none() && !e.is_armed());
    assert_eq!(points(&e), before);
    assert!(e.history.undo.is_empty(), "no undo step");
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), before);
    assert!(e.history.undo.is_empty());
}

/// CS-21: Apply runs the armed tool once as one undo step and leaves it
/// armed on the result with the same values.
#[test]
fn cs21_apply_runs_once_and_stays_armed() {
    let mut e = editor();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    let mut p = params(&e);
    p.bend.spine = DVec3::new(0., 2., 0.);
    p.bend.base = DVec3::new(0., -1., 0.);
    p.bend.axis = 2;
    p.bend.angle = 90.;
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Apply).unwrap();
    let once = points(&e);
    assert_ne!(once, before);
    assert_eq!(e.history.undo.len(), 1);
    assert!(e.is_armed(), "Apply leaves it armed");
    assert_eq!(params(&e).bend.angle, 90.);
    assert_eq!(
        e.session.live.as_ref().unwrap().base,
        e.scene,
        "armed on the result"
    );
    // Dropping it now keeps the applied result and adds nothing.
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(points(&e), once);
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
}

/// CS-02/10/11/14: the first click makes Bend live: Automatic centre at
/// the click, Axis on the work plane's normal, the spine fitted, then the
/// panel's Angle shows on the mesh.
#[test]
fn cs02_first_click_centre_axis_then_the_tools_setup() {
    let mut e = editor();
    let before = points(&e);
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert_eq!(e.action_context.center, CenterMode::Automatic);
    bent(&mut e);
    let at = DVec3::new(0.5, 0.2, 1.);
    e.dispatch(Command::ToolClick {
        point: at,
        plane: 2,
    })
    .unwrap();
    assert!(!e.is_armed());
    let b = params(&e).bend;
    assert_eq!(b.base, at, "CS-10");
    assert_eq!(b.axis, 2, "CS-11: the front plane's normal");
    assert!(
        (b.spine - DVec3::new(-1.5, 0., 0.)).length() < 1e-9,
        "CS-14 {}",
        b.spine
    );
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        at
    );
    assert_ne!(points(&e), before, "the panel's Angle now shows");
    // Bad clicks leave an armed tool armed.
    e.dispatch(Command::Cancel).unwrap();
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert!(
        e.dispatch(Command::ToolClick {
            point: DVec3::NAN,
            plane: 2
        })
        .is_err()
    );
    assert!(e.is_armed());
}

/// CS-10/12/13 on a preset (Taper = Scale + Linear falloff): Automatic
/// moves the pivot to the click; another centre stays; the falloff keeps
/// its fit to the target.
#[test]
fn cs12_cs13_other_centres_stay_and_the_falloff_keeps_its_fit() {
    let mut e = editor();
    e.dispatch(Command::ActivatePreset("xfrm.taper".into()))
        .unwrap();
    assert_eq!(e.armed_tool(), Some(Tool::Scale));
    let falloff = e.spatial_falloff;
    assert!(matches!(falloff, Some(Falloff::Linear(_))));
    let at = DVec3::new(0.3, 1., 0.);
    if e.action_context.center == CenterMode::Automatic {
        e.dispatch(Command::ToolClick {
            point: at,
            plane: 2,
        })
        .unwrap();
        assert_eq!(
            e.session
                .live
                .as_ref()
                .unwrap()
                .action_frame
                .unwrap()
                .center,
            at
        );
    }
    assert_eq!(e.spatial_falloff, falloff, "CS-13");
    e.dispatch(Command::Drop).unwrap();

    // Origin centre: the click doesn't move it (CS-12).
    e.dispatch(Command::ActionContext(ActionContext {
        center: CenterMode::Origin,
        axis: AxisOrientation::World,
    }))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Push)).unwrap();
    assert!(e.is_armed());
    let centre = e
        .session
        .live
        .as_ref()
        .unwrap()
        .action_frame
        .map(|f| f.center);
    e.dispatch(Command::ToolClick {
        point: at,
        plane: 2,
    })
    .unwrap();
    assert!(!e.is_armed());
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .map(|f| f.center),
        centre
    );
}

/// Tools that start at once, tools that wait by their own press code,
/// and an editor that doesn't honour the flags are never armed.
#[test]
fn not_armed_when_on_or_waiting_on_its_own_or_flags_off() {
    let mut e = editor();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    assert!(!e.is_armed());
    e.dispatch(Command::Drop).unwrap();
    e.dispatch(Command::Activate(Tool::Box)).unwrap();
    assert!(!e.is_armed(), "primitives wait by drawing");
    e.dispatch(Command::Cancel).unwrap();
    e.auto_activate_flags = false;
    e.dispatch(Command::Activate(Tool::Bend)).unwrap();
    assert!(!e.is_armed());
}

/// TP-19 (Soft Move, the Mug tutorial): the row's Radial falloff keeps its
/// fitted size but starts where the click places the tool, sharing the
/// move handle's spot; the falloff is then sized apart from the move, and
/// the drop gives the user's falloff back.
#[test]
fn tp19_soft_move_falloff_starts_at_the_click() {
    let mut e = editor();
    e.dispatch(Command::ActivatePreset("xfrm.magnet".into()))
        .unwrap();
    assert_eq!(e.armed_tool(), Some(Tool::Move));
    let Some(Falloff::Radial(fitted)) = e.spatial_falloff else {
        panic!("Soft Move brings a Radial falloff");
    };
    // The factory action centre is None, which a click moves like
    // Automatic (AC-12).
    assert_eq!(e.action_context.center, CenterMode::None);
    // Off the mesh, up and to the left, on the front work plane.
    let at = DVec3::new(-1.5, 0.8, 0.);
    e.dispatch(Command::ToolClick {
        point: at,
        plane: 2,
    })
    .unwrap();
    assert_eq!(
        e.session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center,
        at
    );
    let Some(Falloff::Radial(placed)) = e.spatial_falloff else {
        panic!("still Radial");
    };
    assert_eq!(placed.center, at, "the falloff shares the tool's spot");
    assert_eq!(placed.size, fitted.size, "its fitted size is kept");
    assert_eq!(placed.shape, fitted.shape);

    // Narrow it on X (the falloff's X box), then move: only the points
    // inside it go, the nearest the most.
    let narrow = RadialFalloff {
        size: DVec3::new(0.8, 3., 3.),
        ..placed
    };
    e.dispatch(Command::SpatialFalloff(Some(narrow.into())))
        .unwrap();
    let before = points(&e);
    let mut p = params(&e);
    p.delta = DVec3::new(0., 1., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    let after = points(&e);
    for (a, b) in before.iter().zip(&after) {
        let w = narrow.weight(*a);
        assert!((b.y - a.y - w).abs() < 1e-9, "{a} moved by its weight {w}");
        if a.x > 0. {
            assert_eq!(a, b, "the far side stays");
        }
    }
    assert!(before.iter().zip(&after).any(|(a, b)| a != b));
    assert_eq!(e.spatial_falloff, Some(narrow.into()), "the move keeps it");
    e.dispatch(Command::Drop).unwrap();
    assert_eq!(e.spatial_falloff, None, "TP-12: the user had none");
}

/// TP-19 with a moved mesh: the falloff lives in the mesh's own space
/// (FO-35, Use World Transforms off), so the click is taken into it.
#[test]
fn tp19_the_click_is_taken_into_the_meshs_space() {
    let mut e = editor();
    let id = e.scene.active().unwrap().id;
    e.scene.item_mut(id).unwrap().transform.translation = DVec3::new(10., 0., 0.);
    e.dispatch(Command::ActivatePreset("xfrm.magnet".into()))
        .unwrap();
    let at = DVec3::new(8.5, 0.8, 0.);
    e.dispatch(Command::ToolClick {
        point: at,
        plane: 2,
    })
    .unwrap();
    let Some(Falloff::Radial(f)) = e.spatial_falloff else {
        panic!("Radial");
    };
    assert!(!f.world);
    assert!(
        (f.center - DVec3::new(-1.5, 0.8, 0.)).length() < 1e-9,
        "{}",
        f.center
    );
}
