//! Original CLEAN-F02 mapping and New Mesh scene oracle; no filesystem I/O.
use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;
use std::collections::{BTreeMap, BTreeSet};

fn editor_with_interleaved_orphan() -> Editor {
    let mut editor = Editor::default();
    editor.scene.new_mesh().unwrap();
    let mesh = &mut editor.scene.active_mut().unwrap().mesh;
    mesh.vertices = vec![
        DVec3::ZERO,
        DVec3::new(4., 3., 2.),
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(0., 1., 0.),
    ];
    mesh.faces = vec![Face {
        vertices: vec![0, 2, 3, 4],
        uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
        material: "CleanupOriginalA".into(),
        smooth: false,
        part: Some("Kept".into()),
        material_label: None,
    }]
    .into();
    mesh.attributes.point_weights.insert(
        "weight".into(),
        BTreeMap::from([(0, 0.25), (1, 0.5), (3, 0.75)]),
    );
    // Continuous point UVs require a same-named corner map; all its
    // corners are absent, independently of Face::uv's authored coordinates.
    mesh.uv_maps.insert("pointUV".into(), vec![vec![None; 4]]);
    mesh.attributes.point_uvs.insert(
        "pointUV".into(),
        BTreeMap::from([(1, DVec2::new(7., 9.)), (4, DVec2::new(0.25, 0.875))]),
    );
    mesh.attributes.point_colors.insert(
        "color".into(),
        BTreeMap::from([(4, DVec4::new(1., 0.5, 0., 1.))]),
    );
    mesh.attributes.relative_morphs.insert(
        "morph".into(),
        BTreeMap::from([(2, DVec3::new(0.125, 0., 0.))]),
    );
    mesh.attributes
        .vertex_sets
        .insert("Keep".into(), BTreeSet::from([0, 4]));
    mesh.attributes
        .vertex_sets
        .insert("Mixed".into(), BTreeSet::from([1, 3]));
    mesh.attributes.creases.push(MeshCrease {
        a: 0,
        b: 2,
        weight: 0.5,
    });
    editor.scene.active_mut().unwrap().transform.translation = DVec3::new(1., 2., 3.);
    editor.scene.new_mesh().unwrap();
    let other = editor.scene.active_mut().unwrap();
    other.transform.translation = DVec3::new(-2., 1., 4.);
    other.mesh.vertices = vec![
        DVec3::new(10., 0., 0.),
        DVec3::new(12., 0., 0.),
        DVec3::new(12., 1., 0.),
        DVec3::new(10., 1., 0.),
        DVec3::new(14., 3., 2.),
    ];
    other.mesh.faces = vec![Face {
        vertices: vec![0, 1, 2, 3],
        uv: Some(vec![DVec2::ZERO, DVec2::X, DVec2::ONE, DVec2::Y]),
        material: "CleanupOriginalB".into(),
        smooth: false,
        part: Some("Unrelated".into()),
        material_label: None,
    }]
    .into();
    other.mesh.attributes.point_weights.insert(
        "weight".into(),
        BTreeMap::from([(0, 0.25), (2, 0.75), (4, 0.5)]),
    );
    other
        .mesh
        .uv_maps
        .insert("pointUV".into(), vec![vec![None; 4]]);
    other
        .mesh
        .attributes
        .point_uvs
        .insert("pointUV".into(), BTreeMap::from([(4, DVec2::new(7., 9.))]));
    editor.scene.select_item(Some(1));
    editor.scene.selection.mode = Mode::Item;
    editor.scene.selection.vertices = vec![4, 1, 0];
    editor.scene.selection.edges = vec![(0, 2), (3, 4)];
    editor.scene.selection.faces = vec![0];
    editor.scene.validate().unwrap();
    editor.synchronize_identities().unwrap();
    editor
}

#[test]
fn cleanup_maps_sparse_payload_selection_identity_and_history() {
    let mut editor = editor_with_interleaved_orphan();
    let before = editor.scene.clone();
    // Independently literal CLEAN-F02 expected scene; no operation builds it.
    let mut expected = before.clone();
    let mesh = &mut expected.active_mut().unwrap().mesh;
    mesh.vertices = vec![
        DVec3::ZERO,
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(0., 1., 0.),
    ];
    mesh.faces[0].vertices = vec![0, 1, 2, 3];
    mesh.attributes
        .point_weights
        .insert("weight".into(), BTreeMap::from([(0, 0.25), (2, 0.75)]));
    mesh.attributes.point_uvs.insert(
        "pointUV".into(),
        BTreeMap::from([(3, DVec2::new(0.25, 0.875))]),
    );
    mesh.attributes.point_colors.insert(
        "color".into(),
        BTreeMap::from([(3, DVec4::new(1., 0.5, 0., 1.))]),
    );
    mesh.attributes.relative_morphs.insert(
        "morph".into(),
        BTreeMap::from([(1, DVec3::new(0.125, 0., 0.))]),
    );
    mesh.attributes
        .vertex_sets
        .insert("Keep".into(), BTreeSet::from([0, 3]));
    mesh.attributes
        .vertex_sets
        .insert("Mixed".into(), BTreeSet::from([2]));
    mesh.attributes.creases = vec![MeshCrease {
        a: 0,
        b: 1,
        weight: 0.5,
    }];
    expected.selection.vertices = vec![3, 0];
    expected.selection.edges = vec![(0, 1), (2, 3)];
    let unrelated = editor.scene.items[1].clone();
    let ids_before = editor.identities.current.clone();
    let next_before = editor.identities.next_id().unwrap();
    editor.dispatch(Command::CleanupFloating(None)).unwrap();
    assert_eq!(editor.scene, expected);
    let item = editor.scene.active().unwrap();
    assert_eq!(
        item.mesh.vertices,
        vec![
            DVec3::ZERO,
            DVec3::new(2., 0., 0.),
            DVec3::new(2., 1., 0.),
            DVec3::new(0., 1., 0.)
        ]
    );
    assert_eq!(item.mesh.faces[0].vertices, vec![0, 1, 2, 3]);
    assert_eq!(
        item.mesh.attributes.point_weights["weight"],
        BTreeMap::from([(0, 0.25), (2, 0.75)])
    );
    assert_eq!(
        item.mesh.attributes.point_uvs["pointUV"],
        BTreeMap::from([(3, DVec2::new(0.25, 0.875))])
    );
    assert_eq!(
        item.mesh.attributes.point_colors["color"],
        BTreeMap::from([(3, DVec4::new(1., 0.5, 0., 1.))])
    );
    assert_eq!(
        item.mesh.attributes.relative_morphs["morph"],
        BTreeMap::from([(1, DVec3::new(0.125, 0., 0.))])
    );
    assert_eq!(
        item.mesh.attributes.vertex_sets["Keep"],
        BTreeSet::from([0, 3])
    );
    assert_eq!(
        item.mesh.attributes.vertex_sets["Mixed"],
        BTreeSet::from([2])
    );
    assert_eq!(
        item.mesh.attributes.creases,
        vec![MeshCrease {
            a: 0,
            b: 1,
            weight: 0.5
        }]
    );
    assert_eq!(editor.scene.selection.vertices, vec![3, 0]);
    assert_eq!(editor.scene.selection.edges, vec![(0, 1), (2, 3)]);
    assert_eq!(editor.scene.selection.faces, vec![0]);
    let old = &ids_before.meshes[&1];
    let new = &editor.identities.current.meshes[&1];
    assert_eq!(
        new.vertices,
        vec![
            old.vertices[0],
            old.vertices[2],
            old.vertices[3],
            old.vertices[4]
        ]
    );
    assert_eq!(new.faces, old.faces);
    assert_eq!(new.corners, old.corners);
    assert_eq!(new.edges, old.edges);
    assert_eq!(editor.identities.next_id().unwrap(), next_before);
    assert_eq!(editor.scene.items[1], unrelated);
    let after = editor.scene.clone();
    let ids_after = editor.identities.current.clone();
    assert_eq!(editor.undo.last().unwrap().label, "Mesh Cleanup");
    editor.dispatch(Command::Undo).unwrap();
    assert_eq!(editor.scene, before);
    assert_eq!(editor.identities.current, ids_before);
    editor.dispatch(Command::Redo).unwrap();
    assert_eq!(editor.scene, after);
    assert_eq!(editor.identities.current, ids_after);
    let before_noop = editor.clone();
    editor.dispatch(Command::CleanupFloating(None)).unwrap();
    assert!(
        editor == before_noop,
        "already clean must preserve history and identity observations"
    );
}

#[test]
fn cleanup_f01_matches_the_entire_independently_authored_scene() {
    let mut editor = editor_with_interleaved_orphan();
    let mesh = &mut editor.scene.active_mut().unwrap().mesh;
    mesh.vertices = vec![
        DVec3::ZERO,
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(4., 3., 2.),
    ];
    mesh.faces[0].vertices = vec![0, 1, 2, 3];
    mesh.attributes.point_weights.insert(
        "weight".into(),
        BTreeMap::from([(0, 0.25), (2, 0.75), (4, 0.5)]),
    );
    mesh.attributes
        .point_uvs
        .insert("pointUV".into(), BTreeMap::from([(4, DVec2::new(7., 9.))]));
    mesh.attributes.point_colors.clear();
    mesh.attributes.relative_morphs.clear();
    mesh.attributes.vertex_sets.clear();
    mesh.attributes.creases.clear();
    editor.scene.selection.vertices = vec![0, 4];
    editor.scene.selection.edges = vec![(0, 1), (2, 3)];
    editor.scene.validate().unwrap();
    editor.synchronize_identities().unwrap();
    let before = editor.scene.clone();
    let mut expected = before.clone();
    let mesh = &mut expected.active_mut().unwrap().mesh;
    mesh.vertices = vec![
        DVec3::ZERO,
        DVec3::new(2., 0., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(0., 1., 0.),
    ];
    mesh.attributes
        .point_weights
        .insert("weight".into(), BTreeMap::from([(0, 0.25), (2, 0.75)]));
    mesh.attributes
        .point_uvs
        .insert("pointUV".into(), BTreeMap::new());
    expected.selection.vertices = vec![0];
    editor.dispatch(Command::CleanupFloating(None)).unwrap();
    assert_eq!(editor.scene, expected);
    editor.dispatch(Command::Undo).unwrap();
    assert_eq!(editor.scene, before);
    editor.dispatch(Command::Redo).unwrap();
    assert_eq!(editor.scene, expected);
}

#[test]
fn cleanup_requires_an_explicit_visible_idle_item_and_preserves_redo_on_refusal() {
    let mut editor = editor_with_interleaved_orphan();
    editor.dispatch(Command::CleanupFloating(None)).unwrap();
    editor.dispatch(Command::Undo).unwrap();
    editor.scene.selection.item = None;
    let before = editor.scene.clone();
    let redo = editor.redo.clone();
    assert!(editor.dispatch(Command::CleanupFloating(None)).is_err());
    assert_eq!(editor.scene, before);
    assert!(editor.redo == redo);
}

#[test]
fn cleanup_rejects_unknown_typed_association_without_mutating_input() {
    let editor = editor_with_interleaved_orphan();
    let mut input = editor.scene.active().unwrap().mesh.to_owned_value();
    input.attributes.typed_maps.insert(
        "future".into(),
        TypedMap {
            kind: "unknown-future-class".into(),
            dimension: 1,
            zero_default: false,
            points: BTreeMap::from([(1, vec![0.5])]),
            corners: vec![],
        },
    );
    let before = input.clone();
    assert!(remove_floating_vertices(&input).is_err());
    assert_eq!(input, before);
}
