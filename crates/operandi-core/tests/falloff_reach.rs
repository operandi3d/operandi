//! Falloff batch 2 (falloff spec 2026-11-15, FO-45, FO-47, FO-48): Soft
//! Selection, Element and Coplanar. Original test data; ramps marked
//! provisional in the spec (FO-Q05, FO-Q06) follow its default readings.
use glam::DVec3;
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: String::new(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A 1 × 6 strip of quads along X: bottom points 0..=6 at x = 0..6, top
/// points 7..=13 at y = 1. With `fold`, the last quad (x = 5..6) tilts up
/// by 60° about x = 5, and with `second`, a separate copy of the first
/// quad sits at z = 5.
fn strip(fold: bool, second: bool) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut vertices = Vec::new();
    for y in 0..2 {
        for x in 0..=6 {
            let mut p = DVec3::new(x as f64, y as f64, 0.);
            if fold && x == 6 {
                let a = 60f64.to_radians();
                p = DVec3::new(5. + a.cos(), y as f64, a.sin());
            }
            vertices.push(p);
        }
    }
    let mut faces: Vec<Face> = (0..6u32)
        .map(|i| face(vec![i, i + 1, i + 8, i + 7]))
        .collect();
    if second {
        for p in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)] {
            vertices.push(DVec3::new(p.0, p.1, 5.));
        }
        faces.push(face(vec![14, 15, 16, 17]));
    }
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0, 7];
    e.history.undo.clear();
    e
}
fn points(e: &Editor) -> Vec<DVec3> {
    e.scene.active().unwrap().mesh.vertices.clone()
}
fn live(e: &Editor) -> Parameters {
    e.session.live.as_ref().unwrap().params.clone()
}
fn weights(e: &Editor, f: Falloff) -> Vec<f64> {
    let id = e.scene.active().unwrap().id;
    let mut stack = ActiveFalloff::new(f);
    stack.element = e.falloff_element;
    stack.weights(&e.scene, id).unwrap()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}
fn soft(radius: f64, connectivity: bool) -> Falloff {
    Falloff::SoftSelection(SoftSelectionFalloff {
        radius,
        connectivity,
        ..Default::default()
    })
}

/// FO-47 factory values (measured) and FO-48 / FO-45 defaults.
#[test]
fn defaults_match_the_spec() {
    let s = SoftSelectionFalloff::default();
    assert_eq!(
        (
            s.radius,
            s.connectivity,
            s.shape,
            s.show_vertex,
            s.show_edge
        ),
        (0.1, true, FalloffShape::Linear, true, false)
    );
    let c = CoplanarFalloff::default();
    assert_eq!(
        (c.angle, c.connectivity, c.shape, c.show_vertex, c.show_edge),
        (0., true, FalloffShape::Linear, true, true)
    );
    let el = ElementFalloff::default();
    assert_eq!(
        (el.mode, el.range, el.connect, el.always_raycast, el.shape),
        (
            TweakMode::Auto,
            0.,
            TweakConnect::Ignore,
            true,
            FalloffShape::Linear
        )
    );
    assert!(soft(-1., true).validate().is_err());
    assert!(
        Falloff::Coplanar(CoplanarFalloff {
            angle: 200.,
            ..Default::default()
        })
        .validate()
        .is_err()
    );
}

/// FO-47: selected points 1, others Shape(d / Radius), 0 from Radius on;
/// distance runs along polygon sides with Use Connectivity.
#[test]
fn soft_selection_fades_over_radius_beyond_the_selection() {
    let e = strip(false, false);
    let w = weights(&e, soft(4., true));
    for x in 0..=6 {
        let want = (1. - x as f64 / 4.).max(0.);
        assert!(close(w[x], want), "x = {x}: {} not {want}", w[x]);
        assert!(close(w[7 + x], want));
    }
    // Radius 0: only the selection.
    let w = weights(&e, soft(0., true));
    assert_eq!(w.iter().filter(|&&w| w > 0.).count(), 2);
}

/// Use Connectivity off measures straight lines and reaches separate
/// pieces; on, a separate piece weighs 0.
#[test]
fn soft_selection_connectivity_decides_separate_pieces() {
    let e = strip(false, true);
    let on = weights(&e, soft(6., true));
    assert!(on[14..18].iter().all(|&w| w == 0.));
    let off = weights(&e, soft(6., false));
    // Point 14 sits 5 straight above point 0.
    assert!(close(off[14], 1. - 5. / 6.), "{}", off[14]);
    // Straight-line equals along-the-strip on a flat row.
    assert!(close(off[3], on[3]));
}

/// FO-47: the only falloff that moves geometry outside the selection; Cancel
/// leaves the mesh as it was, and the dropped move is one undo step.
#[test]
fn soft_selection_move_reaches_outside_and_undoes_in_one_step() {
    let mut e = strip(false, false);
    let before = points(&e);
    e.dispatch(Command::SpatialFalloff(Some(soft(2., true))))
        .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Z,
        ..live(&e)
    }))
    .unwrap();
    let after = points(&e);
    for x in 0..=6 {
        let want = (1. - x as f64 / 2.).max(0.);
        assert!(close(after[x].z - before[x].z, want), "x = {x}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Z,
        ..live(&e)
    }))
    .unwrap();
    e.dispatch(Command::Drop).unwrap();
    let moved = points(&e);
    assert_ne!(moved, before);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(points(&e), before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(points(&e), moved);
}

/// FO-48: Angle 0 takes the flat neighbours at weight 1 but not the quad
/// folded up by 60°; Angle 120 takes it at Shape(60 / 120).
#[test]
fn coplanar_spreads_across_polygons_facing_the_same_way() {
    let mut e = strip(true, false);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    let cop = |angle: f64, connectivity: bool| {
        Falloff::Coplanar(CoplanarFalloff {
            angle,
            connectivity,
            ..Default::default()
        })
    };
    let w = weights(&e, cop(0., true));
    assert!((0..=5).all(|x| w[x] == 1.), "{w:?}");
    assert_eq!(w[6], 0.);
    let w = weights(&e, cop(120., true));
    assert!(close(w[6], 0.5), "{}", w[6]);
    // The flat neighbours share the fold's points at x = 5: they keep 1.
    assert_eq!(w[5], 1.);
}

/// Use Connectivity on stops at the mesh's own piece; off, any polygon
/// facing the same way takes part.
#[test]
fn coplanar_connectivity_decides_separate_pieces() {
    let mut e = strip(false, true);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    let cop = |connectivity| {
        Falloff::Coplanar(CoplanarFalloff {
            connectivity,
            ..Default::default()
        })
    };
    assert!(weights(&e, cop(true))[14..18].iter().all(|&w| w == 0.));
    assert!(weights(&e, cop(false))[14..18].iter().all(|&w| w == 1.));
    // Vertex mode: the polygons whose corners are all selected set the
    // facing; with none, nothing spreads.
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0, 1, 8, 7];
    assert!(weights(&e, cop(true))[0..7].iter().all(|&w| w == 1.));
    e.scene.selection.vertices = vec![0];
    let w = weights(&e, cop(true));
    assert_eq!(w.iter().filter(|&&w| w > 0.).count(), 1);
}

/// FO-45: before a pick nothing weighs anything; after one the picked
/// element weighs 1 and Range spreads with the Shape, as Tweak's Range.
#[test]
fn element_weights_follow_the_pick_and_range() {
    let mut e = strip(false, false);
    e.scene.selection.vertices.clear();
    let el = |range: f64| {
        Falloff::Element(ElementFalloff {
            range,
            ..Default::default()
        })
    };
    // A pick needs an Element layer in the stack.
    let id = e.scene.active().unwrap().id;
    let grab = TweakGrab {
        item: id,
        element: TweakElement::Vertex(3),
    };
    assert!(e.dispatch(Command::FalloffElementPick(Some(grab))).is_err());
    e.dispatch(Command::SpatialFalloff(Some(el(2.)))).unwrap();
    assert!(weights(&e, el(2.)).iter().all(|&w| w == 0.));
    e.dispatch(Command::FalloffElementPick(Some(grab))).unwrap();
    let w = weights(&e, el(2.));
    assert_eq!(w[3], 1.);
    assert!(close(w[2], 0.5) && close(w[4], 0.5) && w[1] == 0.);
    let w = weights(&e, el(0.));
    assert_eq!(w.iter().filter(|&&w| w > 0.).count(), 1);
    // Picking another falloff type clears the pick.
    e.dispatch(Command::SpatialFalloff(Some(soft(1., true))))
        .unwrap();
    assert_eq!(e.falloff_element, None);
}

/// FO-45: a Move with an Element pick moves only the picked element (and
/// its Range), whatever is selected; Cancel restores.
#[test]
fn element_move_moves_the_picked_element() {
    let mut e = strip(false, false);
    let before = points(&e);
    let id = e.scene.active().unwrap().id;
    e.dispatch(Command::SpatialFalloff(Some(Falloff::Element(
        ElementFalloff::default(),
    ))))
    .unwrap();
    e.dispatch(Command::FalloffElementPick(Some(TweakGrab {
        item: id,
        element: TweakElement::Edge(4, 11),
    })))
    .unwrap();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    e.dispatch(Command::Preview(Parameters {
        delta: DVec3::Z,
        ..live(&e)
    }))
    .unwrap();
    let after = points(&e);
    for (v, (a, b)) in after.iter().zip(&before).enumerate() {
        let lift = a.z - b.z;
        let want = if v == 4 || v == 11 { 1. } else { 0. };
        assert!(close(lift, want), "point {v} moved {lift}");
    }
    e.dispatch(Command::Cancel).unwrap();
    assert_eq!(points(&e), before);
}

/// The command line adds each with its keyed options.
#[test]
fn commands_take_their_options() {
    let parsed = |line: &str| match command_line::parse_sculpt(line) {
        Some(command_line::SculptLine::Falloff { falloff, .. }) => falloff,
        other => panic!("{line}: {other:?}"),
    };
    assert_eq!(
        parsed("falloff.softSelection radius:0.5 connectivity:false"),
        soft(0.5, false)
    );
    assert_eq!(
        parsed("falloff.coplanar angle:10"),
        Falloff::Coplanar(CoplanarFalloff {
            angle: 10.,
            ..Default::default()
        })
    );
    assert_eq!(
        parsed("falloff.element mode:polygonCenter range:0.3 connect:rigid shape:smooth"),
        Falloff::Element(ElementFalloff {
            mode: TweakMode::PolygonCenter,
            range: 0.3,
            connect: TweakConnect::Rigid,
            shape: FalloffShape::Smooth,
            ..Default::default()
        })
    );
    assert!(matches!(
        command_line::parse_sculpt("falloff.softSelection radius:-1"),
        Some(command_line::SculptLine::Usage(_))
    ));
}
