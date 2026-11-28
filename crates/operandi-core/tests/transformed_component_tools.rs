//! Component tools on moved/rotated items (fix pass, audit X1): frames are
//! resolved in world space and edits map back to local points.
use glam::DVec3;
use operandi_core::*;

fn moved_cube() -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let item = e.scene.active_mut().unwrap();
    item.mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE * 2., 1)
        .unwrap()
        .into();
    item.transform.translation = DVec3::new(5., 0., 0.);
    item.transform.rotation = DVec3::new(0., 0., std::f64::consts::FRAC_PI_2);
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1];
    e.synchronize_identities().unwrap();
    e
}

#[test]
fn bevel_chamfer_haul_and_sketch_activate_on_a_moved_item() {
    for tool in [Tool::Bevel, Tool::PolyHaul, Tool::SketchExtrude] {
        let mut e = moved_cube();
        e.dispatch(Command::Activate(tool))
            .unwrap_or_else(|err| panic!("{tool:?}: {err}"));
        e.dispatch(Command::Cancel).unwrap();
    }
}

#[test]
fn component_move_on_a_rotated_item_moves_along_world_axes() {
    let mut e = moved_cube();
    let world = e.scene.world_matrix(e.scene.active().unwrap().id).unwrap();
    let before: Vec<DVec3> = e.scene.active().unwrap().mesh.faces[1]
        .vertices
        .iter()
        .map(|&v| world.transform_point3(e.scene.active().unwrap().mesh.vertices[v as usize]))
        .collect();
    e.dispatch(Command::Activate(Tool::Move)).unwrap();
    let mut p = e.session.live.as_ref().unwrap().params.clone();
    p.delta = DVec3::new(1., 0., 0.);
    e.dispatch(Command::Preview(p)).unwrap();
    e.dispatch(Command::Drop).unwrap();
    let item = e.scene.active().unwrap();
    let after: Vec<DVec3> = item.mesh.faces[1]
        .vertices
        .iter()
        .map(|&v| world.transform_point3(item.mesh.vertices[v as usize]))
        .collect();
    for (a, b) in before.iter().zip(&after) {
        assert!((*b - *a - DVec3::X).length() < 1e-9, "{a} -> {b}");
    }
}

#[test]
fn triple_splits_a_nonplanar_quad() {
    // Triple splits any polygon with more than 3 vertices.
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.3),
            DVec3::new(1., 1., 0.),
            DVec3::new(0., 1., 0.3),
        ],
        faces: vec![Face {
            vertices: vec![0, 1, 2, 3],
            uv: None,
            material: String::new(),
            material_label: None,
            smooth: false,
            part: None,
        }]
        .into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::Triple).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert_eq!(m.faces.len(), 2);
    assert!(m.faces.iter().all(|f| f.vertices.len() == 3));
}

#[test]
fn set_material_creates_a_new_tag_on_a_plain_cube() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 2];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::SetMaterial(MaterialAssignment {
        name: "Sphere Top".into(),
        expected: None,
        material: None,
    }))
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    for f in [0, 2] {
        assert_eq!(m.faces[f].material, "Sphere Top");
        assert_eq!(m.faces[f].material_label.as_deref(), Some("Sphere Top"));
    }
    assert_eq!(m.faces[1].material, "");
    // Default returns the polygons to the default material.
    e.dispatch(Command::SetMaterial(MaterialAssignment {
        name: "Default".into(),
        expected: None,
        material: None,
    }))
    .unwrap();
    let m = &e.scene.active().unwrap().mesh;
    assert!(m.faces.iter().all(|f| f.material.is_empty()));
}
