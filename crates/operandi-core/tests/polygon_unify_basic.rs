//! Independent UNY-F01 scenes, including exact exceptional edge ledger.
use glam::DVec3;
use operandi_core::*;

#[test]
fn unify_removes_every_duplicate_group_of_any_size() {
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
    // A triangle stacked three times (two reversed, so their shared edges are
    // exceptional) and a pentagon twice, all with nothing selected.
    let mut m = Mesh {
        vertices: vec![
            DVec3::new(0., 0., 0.),
            DVec3::new(1., 0., 0.),
            DVec3::new(0., 1., 0.),
            DVec3::new(3., 0., 0.),
            DVec3::new(4., 0., 0.),
            DVec3::new(4.5, 1., 0.),
            DVec3::new(3.5, 2., 0.),
            DVec3::new(2.5, 1., 0.),
        ],
        faces: vec![
            face(vec![0, 1, 2]),
            face(vec![3, 4, 5, 6, 7]),
            face(vec![0, 2, 1]),
            face(vec![7, 6, 5, 4, 3]),
        ]
        .into(),
        ..Default::default()
    };
    m.faces.push(face(vec![1, 2, 0]));
    let mut edges: std::collections::BTreeMap<(u32, u32), Vec<[u32; 3]>> = Default::default();
    for (fi, f) in m.faces.iter().enumerate() {
        let n = f.vertices.len();
        for i in 0..n {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % n]);
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    m.attributes.extrude_edge_uses = edges
        .into_iter()
        .filter(|(_, u)| u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]))
        .map(|((a, b), u)| (format!("{a}:{b}"), u))
        .collect();
    m.validate().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.validate().unwrap();
    e.dispatch(Command::UnifyPolygons(UnifyRequest {
        force: false,
        expected: None,
    }))
    .unwrap();
    let out = &e.scene.active().unwrap().mesh;
    out.validate().unwrap();
    assert_eq!(out.faces.len(), 2, "one triangle and one pentagon remain");
    assert!(out.attributes.extrude_edge_uses.is_empty());
}
