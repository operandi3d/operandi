use glam::{DVec2, DVec3};
use operandi_core::*;

#[test]
fn flip_then_align_preserves_full_scene_corner_data_and_history() {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    let target = e.scene.selection.item.unwrap();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::new(5., 0., 0.), DVec3::ONE, 1)
        .unwrap()
        .into();
    e.scene.select_item(Some(target));
    let face = |vertices| Face {
        vertices,
        uv: None,
        material: "m".into(),
        smooth: false,
        part: Some("part".into()),
        material_label: None,
    };
    let mut mesh = Mesh {
        vertices: vec![
            DVec3::ZERO,
            DVec3::X,
            DVec3::new(2., 0., 0.),
            DVec3::Y,
            DVec3::new(1., 1., 0.),
            DVec3::new(2., 1., 0.),
        ],
        faces: vec![face(vec![0, 1, 4, 3]), face(vec![1, 2, 5, 4])].into(),
        ..Default::default()
    };
    mesh.uv_maps.insert(
        "uv".into(),
        vec![
            vec![Some(DVec2::ZERO); 4],
            vec![
                Some(DVec2::new(1., 0.)),
                None,
                Some(DVec2::new(2., 1.)),
                Some(DVec2::new(1., 1.)),
            ],
        ],
    );
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![1];
    e.synchronize_identities().unwrap();
    e.dispatch(Command::FlipPolygons).unwrap();
    // Face 0 selected first is the Align reference.
    e.scene.selection.faces = vec![0, 1];
    let before = e.scene.clone();
    let ids_before = e.identities.current.clone();
    let mut expected = before.clone();
    let m = &mut expected.active_mut().unwrap().mesh;
    // Flip and Align both keep the first corner and reverse the rest, so
    // Align returns face 1 to its original corners and UVs exactly.
    m.faces[1].vertices = vec![1, 2, 5, 4];
    m.uv_maps.get_mut("uv").unwrap()[1] = vec![
        Some(DVec2::new(1., 0.)),
        None,
        Some(DVec2::new(2., 1.)),
        Some(DVec2::new(1., 1.)),
    ];
    m.attributes.extrude_edge_uses.clear();
    assert_eq!(e.align_flip_count().unwrap(), 1);
    e.dispatch(Command::AlignPolygons).unwrap();
    assert_eq!(e.scene, expected);
    let after_ids = e.identities.current.clone();
    assert_eq!(
        ids_before.meshes[&target].faces,
        after_ids.meshes[&target].faces
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.current, ids_before);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, expected);
    assert_eq!(e.identities.current, after_ids);
}
