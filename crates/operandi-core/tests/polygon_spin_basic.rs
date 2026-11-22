use glam::DVec3;
use operandi_core::*;

fn plain_pair(vertices: Vec<DVec3>, a: Vec<u32>, b: Vec<u32>) -> Editor {
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        material_label: None,
        smooth: false,
        part: None,
    };
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices,
        faces: vec![face(a), face(b)].into(),
        ..Default::default()
    }
    .into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 1];
    e.scene.validate().unwrap();
    e
}

#[test]
fn triangle_pair_flips_its_diagonal() {
    // Square split along 0-2; spinning gives the 1-3 diagonal.
    let v = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(0., 1., 0.),
    ];
    let mut e = plain_pair(v, vec![0, 1, 2], vec![0, 2, 3]);
    e.dispatch(Command::SpinQuads).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert!(m.edges().contains(&(1, 3)));
    assert!(!m.edges().contains(&(0, 2)));
}

#[test]
fn pentagon_pair_on_a_curved_surface_spins() {
    // Two pentagons sharing edge 0-1, lifted at the middle so neither is planar.
    let v = vec![
        DVec3::new(0., 0., 0.3),
        DVec3::new(0., 2., 0.3),
        DVec3::new(-1., 2.5, 0.),
        DVec3::new(-2., 1., 0.),
        DVec3::new(-1., -0.5, 0.),
        DVec3::new(1., 2.5, 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(1., -0.5, 0.),
    ];
    let mut e = plain_pair(v, vec![0, 1, 2, 3, 4], vec![1, 0, 7, 6, 5]);
    e.dispatch(Command::SpinQuads).unwrap();
    let m = &e.scene.active().unwrap().mesh;
    m.validate().unwrap();
    assert!(m.faces.iter().all(|f| f.vertices.len() == 5));
    assert!(!m.edges().contains(&(0, 1)));
    // Mixed sizes still refuse.
    let v = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(2., 0.5, 0.),
    ];
    let mut e = plain_pair(v, vec![0, 1, 2, 3], vec![1, 4, 2]);
    let before = e.clone();
    let err = e.dispatch(Command::SpinQuads).unwrap_err();
    assert!(err.contains("same number of edges"), "{err}");
    assert!(e == before);
}
