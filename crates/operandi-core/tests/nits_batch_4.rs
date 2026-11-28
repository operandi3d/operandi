//! Nits batch 4 (Jason, 2026-11-17): regression tests.
use glam::DVec3;
use operandi_core::*;

/// A 3 × 3 grid in X/Y, Edge mode, one border edge selected; flags
/// honoured as in the app.
fn grid_edge() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh::default();
    for y in 0..4 {
        for x in 0..4 {
            mesh.vertices.push(DVec3::new(x as f64, y as f64, 0.));
        }
    }
    for y in 0..3u32 {
        for x in 0..3u32 {
            let a = y * 4 + x;
            mesh.faces.push(Face {
                vertices: vec![a, a + 1, a + 5, a + 4],
                uv: None,
                material: "Default".into(),
                smooth: false,
                part: None,
                material_label: None,
            });
        }
    }
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    // The top border's middle edge.
    e.scene.selection.edges = vec![(13, 14)];
    e.history.undo.clear();
    e.auto_activate_flags = true;
    e
}

/// N25: with Edge Extend (Z), the first click in a view makes the extend at
/// once (new edges on top of the old, zero offset), so the next tool, such
/// as Scale, works on them without pulling them out first.
#[test]
fn n25_edge_extend_first_click_makes_the_extend() {
    let mut e = grid_edge();
    let faces = e.scene.active().unwrap().mesh.faces.len();
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    assert!(e.is_armed(), "Edge Extend waits for a click (CS table)");
    e.dispatch(Command::ToolClick {
        point: DVec3::new(1.5, 3., 0.),
        plane: 2,
    })
    .unwrap();
    assert!(
        e.scene.active().unwrap().mesh.faces.len() > faces,
        "the click made the extend"
    );
    let extended = e.scene.selection.edges.clone();
    e.dispatch(Command::Activate(Tool::Scale)).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    assert!(mesh.faces.len() > faces, "switching tools keeps it");
    mesh.validate().unwrap();
    assert_eq!(
        e.scene.selection.edges, extended,
        "the new edges stay selected"
    );
}

/// N27, revised by ACS-01 (2026-11-19): with Symmetry on, a tool's handle
/// sits at the centre of the selection's + half, not halfway between the
/// halves, whichever side was picked first (selection order doesn't
/// matter, ACS-01). Off: the whole selection's centre.
#[test]
fn n27_symmetry_handle_sits_on_the_plus_half() {
    let mut e = strip();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    let item = e.scene.selection.item.unwrap();
    for pick in [(8, 9), (5, 6)] {
        e.scene.selection.edges.clear();
        // The top border's edge at the picked end; its mirror comes too.
        e.dispatch(Command::Select {
            item,
            hit: Component::Edge(pick.0, pick.1),
            replace: true,
            remove: false,
        })
        .unwrap();
        assert_eq!(
            e.scene.selection.edges.len(),
            2,
            "the mirror edge is selected"
        );
        let c = handle(&e);
        assert!((c.x - 1.5).abs() < 1e-9, "{c}");
    }
    e.dispatch(Command::Symmetry(Symmetry::default())).unwrap();
    assert!(handle(&e).x.abs() < 1e-9, "off: the whole selection");
}

/// The pivot service's centre for the selection (no tool open).
fn handle(e: &Editor) -> DVec3 {
    e.pivot_request(&e.scene, Tool::Move)
        .frame()
        .unwrap()
        .center
}

#[test]
fn polygon_stage_handles_follow_the_picked_side_with_symmetry() {
    let mut e = strip();
    assert_eq!(e.action_context.center, CenterMode::None);
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(Mode::Polygon)).unwrap();
    let item = e.scene.selection.item.unwrap();
    e.dispatch(Command::Select {
        item,
        hit: Component::Polygon(3),
        replace: true,
        remove: false,
    })
    .unwrap();
    assert_eq!(e.scene.selection.faces, [3, 0]);
    let before = e.scene.clone();

    for tool in [Tool::Bevel, Tool::Extrude, Tool::SmoothShift, Tool::Thicken] {
        e.dispatch(Command::Activate(tool)).unwrap();
        assert_eq!(
            e.symmetry_handle_centre(),
            Some(DVec3::new(1.5, 0.5, 0.)),
            "{tool:?}"
        );
        e.dispatch(Command::Cancel).unwrap();
        assert_eq!(e.scene, before);
    }

    for tool in [Tool::Move, Tool::Rotate, Tool::Scale, Tool::Transform] {
        e.dispatch(Command::Activate(tool)).unwrap();
        let centre = e
            .session
            .live
            .as_ref()
            .unwrap()
            .action_frame
            .unwrap()
            .center;
        assert_eq!(centre, DVec3::new(1.5, 0.5, 0.), "{tool:?}");
        e.dispatch(Command::Cancel).unwrap();
    }

    let mut selection_centre = e.clone();
    selection_centre
        .dispatch(Command::ActionContext(ActionContext {
            center: CenterMode::Selection,
            axis: AxisOrientation::World,
        }))
        .unwrap();
    selection_centre
        .dispatch(Command::Activate(Tool::Bevel))
        .unwrap();
    // ACS-01: Selection takes the + half's centre too (it used to keep the
    // whole selection's centre on the plane).
    assert_eq!(
        selection_centre.symmetry_handle_centre(),
        Some(DVec3::new(1.5, 0.5, 0.))
    );

    let mut without_symmetry = e.clone();
    without_symmetry
        .dispatch(Command::Symmetry(Symmetry::default()))
        .unwrap();
    assert_eq!(without_symmetry.scene.selection.faces, [3, 0]);
    without_symmetry
        .dispatch(Command::Activate(Tool::Bevel))
        .unwrap();
    assert_eq!(without_symmetry.symmetry_handle_centre(), None);

    e.dispatch(Command::Activate(Tool::Bevel)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.shift = 0.2;
    p.inset = 0.1;
    e.dispatch(Command::Preview(p.clone())).unwrap();
    let centre = e.symmetry_handle_centre().unwrap();
    assert!(
        centre.distance(DVec3::new(1.5, 0.5, 0.2)) < 1e-9,
        "{centre}"
    );
    let caps: Vec<_> = e
        .scene
        .selection
        .faces
        .iter()
        .map(|&face| e.scene.active().unwrap().mesh.center(face as usize))
        .collect();
    assert_eq!(caps.len(), 2);
    for x in [-1.5, 1.5] {
        assert!(
            caps.iter()
                .any(|cap| cap.distance(DVec3::new(x, 0.5, 0.2)) < 1e-9)
        );
    }
    e.dispatch(Command::Drop).unwrap();
    without_symmetry.dispatch(Command::Preview(p)).unwrap();
    without_symmetry.dispatch(Command::Drop).unwrap();
    assert_eq!(e.scene, without_symmetry.scene);
    let after = e.scene.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
}

/// A 4 × 1 strip from x = -2 to 2 (point = row × 5 + column), symmetric
/// across X = 0.
fn strip() -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    // A 4 × 1 strip from x = -2 to 2 (point = column), mirrored across X = 0.
    let mut mesh = Mesh::default();
    for y in 0..2 {
        for x in 0..5 {
            mesh.vertices.push(DVec3::new(x as f64 - 2., y as f64, 0.));
        }
    }
    for x in 0..4u32 {
        mesh.faces.push(Face {
            vertices: vec![x, x + 1, x + 6, x + 5],
            uv: None,
            material: "Default".into(),
            smooth: false,
            part: None,
            material_label: None,
        });
    }
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e
}

/// N27: with Symmetry, Edge Extend's Scale shrinks each half about its own
/// centre, the far half mirrored, so the result stays symmetric.
#[test]
fn n27_extend_scale_works_about_each_sides_own_centre() {
    let mut e = strip();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    let item = e.scene.selection.item.unwrap();
    e.dispatch(Command::Select {
        item,
        hit: Component::Edge(8, 9),
        replace: true,
        remove: false,
    })
    .unwrap();
    e.dispatch(Command::Activate(Tool::EdgeExtend)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(0., 1., 0.);
    p.scale_delta = DVec3::new(0.5, 1., 1.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    mesh.validate().unwrap();
    let mut top: Vec<f64> = mesh
        .vertices
        .iter()
        .filter(|v| (v.y - 2.).abs() < 1e-9)
        .map(|v| v.x)
        .collect();
    top.sort_by(f64::total_cmp);
    let want = [-1.75, -1.25, 1.25, 1.75];
    assert_eq!(top.len(), 4, "{top:?}");
    for (x, w) in top.iter().zip(want) {
        assert!((x - w).abs() < 1e-9, "{top:?}");
    }
}

/// N27's view side is replaced by ACS-01 (pointer position and selection
/// order don't matter): a selection listed − first still puts the handle on
/// the + half.
#[test]
fn n27_selection_order_doesnt_move_the_handle() {
    let mut e = strip();
    e.dispatch(Command::Symmetry(Symmetry::x())).unwrap();
    e.dispatch(Command::Mode(Mode::Edge)).unwrap();
    e.scene.selection.edges = vec![(5, 6), (8, 9)];
    let c = handle(&e);
    assert!((c.x - 1.5).abs() < 1e-9, "{c}");
}
