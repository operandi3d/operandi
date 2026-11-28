//! Tack (TC-20..22) and Topology Pen Duplicate (TC-45), observed hands-on
//! 2026-11-17. Original test scenes built to the observations.
use glam::{DQuat, DVec3};
use operandi_core::*;

fn face(vertices: Vec<u32>) -> Face {
    Face {
        vertices,
        uv: None,
        material: "Default".into(),
        smooth: false,
        part: None,
        material_label: None,
    }
}

/// A 2 m square (facing +Y) turned 30° about X, as its own mesh item; then a
/// second mesh item, active, holding `active`.
fn scene_with(active: Mesh) -> (Editor, u64, DVec3) {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let turn = DQuat::from_rotation_x(30f64.to_radians());
    let square = Mesh {
        vertices: [(-1., -1.), (-1., 1.), (1., 1.), (1., -1.)]
            .into_iter()
            .map(|(x, z)| turn * DVec3::new(x, 0., z))
            .collect(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    let normal = square.normal(0);
    e.scene.active_mut().unwrap().mesh = square.into();
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = active.into();
    let id = e.scene.active().unwrap().id;
    e.history.undo.clear();
    (e, id, normal)
}

fn plane_height(p: DVec3, normal: DVec3) -> f64 {
    p.dot(normal)
}

#[test]
fn tc22_tack_lays_the_selected_face_flat_on_the_target_centre() {
    let cube = Mesh::box_mesh(DVec3::new(2., 1.5, 0.3), DVec3::splat(0.2), 1).unwrap();
    let bottom = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y < -0.9)
        .unwrap() as u32;
    let (mut e, _, normal) = scene_with(cube);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![bottom];
    e.dispatch(Command::Tack(Tack {
        target: DVec3::ZERO,
        normal,
        options: TackOptions::default(),
    }))
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    let bottom_points = &m.faces[bottom as usize].vertices;
    let centre = bottom_points
        .iter()
        .map(|&v| m.vertices[v as usize])
        .sum::<DVec3>()
        / 4.;
    assert!(
        centre.length() < 1e-9,
        "bottom face centre on the square's centre"
    );
    for (v, p) in m.vertices.iter().enumerate() {
        let h = plane_height(*p, normal);
        if bottom_points.contains(&(v as u32)) {
            assert!(h.abs() < 1e-9, "bottom point {v} on the plane: {h}");
        } else {
            assert!((h - 0.2).abs() < 1e-9, "top point {v} 0.2 above: {h}");
        }
    }
    // The bottom face now faces into the square.
    assert!(m.normal(bottom as usize).dot(normal) < -0.999);
    assert_eq!(e.history.undo.len(), 1);
    assert_eq!(e.history.undo.last().unwrap().label, "Tack");
}

#[test]
fn tack_with_align_off_only_moves_and_without_connected_moves_the_face() {
    let cube = Mesh::box_mesh(DVec3::new(2., 1.5, 0.3), DVec3::splat(0.2), 1).unwrap();
    let bottom = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y < -0.9)
        .unwrap() as u32;
    let (mut e, _, normal) = scene_with(cube.clone());
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![bottom];
    e.dispatch(Command::Tack(Tack {
        target: DVec3::ZERO,
        normal,
        options: TackOptions {
            align: false,
            connected: false,
            ..Default::default()
        },
    }))
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    let moved = (0..m.vertices.len())
        .filter(|&v| m.vertices[v] != cube.vertices[v])
        .count();
    assert_eq!(moved, 4, "only the selected face's points");
    // Same shape, just translated: the face keeps facing down.
    assert!(m.normal(bottom as usize).y < -0.999);
    // Nothing selected: refused, nothing changes.
    e.scene.selection.faces.clear();
    assert!(
        e.dispatch(Command::Tack(Tack {
            target: DVec3::ZERO,
            normal,
            options: TackOptions::default(),
        }))
        .is_err()
    );
}

#[test]
fn tc45_shift_drag_duplicates_an_edge_onto_the_background() {
    // A 0.4 m quad hovering 0.1 above the tilted square.
    let (_, _, normal) = scene_with(Mesh::default());
    let turn = DQuat::from_rotation_x(30f64.to_radians());
    let quad = Mesh {
        vertices: [(-0.2, -0.2), (-0.2, 0.2), (0.2, 0.2), (0.2, -0.2)]
            .into_iter()
            .map(|(x, z)| turn * DVec3::new(x, 0.1, z))
            .collect(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    let (mut e, id, _) = scene_with(quad);
    e.scene.selection.mode = Mode::Edge;
    // Right edge 2–3 pulled out along +X.
    e.dispatch(Command::TopologyDuplicate {
        item: id,
        a: 2,
        b: 3,
        offset: DVec3::new(0.4, 0., 0.),
    })
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    assert_eq!(m.faces.len(), 2);
    assert_eq!(m.vertices.len(), 6);
    for v in [4, 5] {
        let h = plane_height(m.vertices[v], normal);
        assert!(h.abs() < 1e-9, "new point {v} on the background: {h}");
    }
    // The old points kept their height (no snapping of what was there).
    assert!((plane_height(m.vertices[2], normal) - 0.1).abs() < 1e-9);
    // Both polygons face the same side.
    assert!(m.normal(0).dot(m.normal(1)) > 0.5);
    assert_eq!(e.history.undo.len(), 1);
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), 1);
    // A ray finds the background from the front, not from behind.
    assert!(e.background_hit(normal * 5., -normal).is_some());
    assert!(e.background_hit(-normal * 5., normal).is_none());
}

#[test]
fn tc22_copy_geometry_leaves_the_original_and_tacks_a_copy() {
    let cube = Mesh::box_mesh(DVec3::new(2., 1.5, 0.3), DVec3::splat(0.2), 1).unwrap();
    let bottom = (0..cube.faces.len())
        .find(|&f| cube.normal(f).y < -0.9)
        .unwrap() as u32;
    let (mut e, _, normal) = scene_with(cube.clone());
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![bottom];
    e.dispatch(Command::Tack(Tack {
        target: DVec3::ZERO,
        normal,
        options: TackOptions {
            copy: true,
            ..Default::default()
        },
    }))
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    assert_eq!(m.vertices.len(), 16, "a whole copy of the cube");
    assert_eq!(m.faces.len(), 12);
    // The original stays put.
    assert_eq!(&m.vertices[..8], &cube.vertices[..]);
    // The copy stands on the square: its points 0 or 0.2 above the plane.
    for p in &m.vertices[8..] {
        let h = plane_height(*p, normal);
        assert!(h.abs() < 1e-9 || (h - 0.2).abs() < 1e-9, "{h}");
    }
    assert_eq!(e.history.undo.len(), 1);
}

#[test]
fn tc46_topology_pen_moves_never_conform_even_with_the_constraint_on() {
    let turn = DQuat::from_rotation_x(30f64.to_radians());
    let quad = Mesh {
        vertices: [(-0.2, -0.2), (-0.2, 0.2), (0.2, 0.2), (0.2, -0.2)]
            .into_iter()
            .map(|(x, z)| turn * DVec3::new(x, 0.1, z))
            .collect(),
        faces: vec![face(vec![0, 1, 2, 3])].into(),
        ..Default::default()
    };
    let (mut e, id, normal) = scene_with(quad);
    e.dispatch(Command::BackgroundConstraint(Some(Default::default())))
        .unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.dispatch(Command::ActivatePreset("mesh.topology".into()))
        .unwrap();
    let p = e.session.live.as_ref().unwrap().params.clone();
    e.dispatch(Command::Preview(Parameters {
        tweak: Some(TweakGrab {
            item: id,
            element: TweakElement::Vertex(0),
        }),
        delta: DVec3::new(0.3, 0., 0.),
        ..p
    }))
    .unwrap();
    let m = e.scene.active().unwrap().mesh.clone();
    // Moved sideways, height kept: 0.1 above the background still.
    assert!((plane_height(m.vertices[0], normal) - 0.1).abs() < 0.05 + 1e-9);
    assert!(
        plane_height(m.vertices[0], normal).abs() > 1e-3,
        "not put on the surface"
    );
}
