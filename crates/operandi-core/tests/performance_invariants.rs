//! Behavioral guards for PERF-01/02. Original analytic meshes and attributes.
use glam::{DVec2, DVec3};
use operandi_core::*;
use std::collections::BTreeSet;

fn attributed_scene() -> Scene {
    let mut scene = Scene::default();
    scene.new_mesh().unwrap();
    let mut mesh = globe_geometry(
        DVec3::ZERO,
        RadialParameters {
            radius: DVec3::ONE,
            sides: 48,
            segments: 24,
        },
    )
    .unwrap();
    for (i, f) in mesh.faces.iter_mut().enumerate() {
        f.uv = Some(
            (0..f.vertices.len())
                .map(|c| DVec2::new(i as f64, c as f64))
                .collect(),
        );
        f.material = format!("material-{}", i % 3);
        f.part = Some("original fixture".into());
        f.smooth = i % 2 == 0;
    }
    mesh.uv_maps.insert(
        "seams".into(),
        mesh.faces
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (0..f.vertices.len())
                    .map(|c| (c % 2 == 0).then_some(DVec2::new(i as f64, c as f64)))
                    .collect()
            })
            .collect(),
    );
    scene.active_mut().unwrap().mesh = mesh.into();
    scene
}

#[test]
fn ordered_remembered_edges_validate_in_every_mode_without_mutation() {
    let mut scene = attributed_scene();
    scene.selection.edges = scene
        .active()
        .unwrap()
        .mesh
        .edges()
        .into_iter()
        .rev()
        .collect();
    scene.selection.vertices = vec![9, 3, 1];
    scene.selection.faces = vec![11, 4, 2];
    for mode in [Mode::Edge, Mode::Vertex, Mode::Polygon, Mode::Item] {
        scene.selection.mode = mode;
        let before = scene.clone();
        scene.validate().unwrap();
        assert_eq!(scene, before);
        assert_eq!(
            persistence::decode(&persistence::encode(&scene).unwrap()).unwrap(),
            before
        );
    }
    // Remembered components still need validation outside their active mode.
    for invalid in [(u32::MAX, u32::MAX), (1, 1), (0, 1000), (1, 0)] {
        let mut bad = scene.clone();
        bad.selection.edges.push(invalid);
        let before = bad.clone();
        assert_eq!(bad.validate().unwrap_err(), "Invalid component selection");
        assert_eq!(bad, before);
    }
    let mut duplicate = scene.clone();
    duplicate.selection.edges.push(duplicate.selection.edges[0]);
    assert_eq!(
        duplicate.validate().unwrap_err(),
        "Duplicate component selection"
    );
    scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap().into();
    assert!(
        scene.validate().is_err(),
        "topology replacement must reject stale selections"
    );
}

#[test]
fn edge_enumeration_keeps_canonical_sorted_topological_identity() {
    let mut mesh = attributed_scene().active().unwrap().mesh.clone();
    // Coincident copies remain topologically distinct, including open boundaries.
    let mut copy = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 0).unwrap();
    copy.faces.pop();
    append_geometry(&mut mesh, copy).unwrap();
    let expected: BTreeSet<_> = mesh
        .faces
        .iter()
        .flat_map(|f| {
            f.vertices
                .iter()
                .zip(f.vertices.iter().cycle().skip(1))
                .map(|(&a, &b)| (a.min(b), a.max(b)))
        })
        .collect();
    assert_eq!(mesh.edges(), expected.into_iter().collect::<Vec<_>>());
    assert!(Mesh::default().edges().is_empty());
    mesh.faces.reverse();
    let before = mesh.edges();
    for face in &mut mesh.faces {
        face.vertices.reverse();
    }
    assert_eq!(
        mesh.edges(),
        before,
        "face/corner traversal order does not assign edge identity"
    );
}

#[test]
fn dense_edge_preview_cancel_history_and_roundtrip_preserve_attributes() {
    let mut scene = attributed_scene();
    scene.selection.mode = Mode::Edge;
    scene.selection.edges = scene.active().unwrap().mesh.edges();
    let original = scene.clone();
    let mut editor = Editor {
        scene,
        ..Default::default()
    };
    editor.action_context.center = CenterMode::Origin;
    editor.dispatch(Command::Activate(Tool::Scale)).unwrap();
    for factor in [0.95, 1.05, 0.95] {
        editor
            .dispatch(Command::Preview(Parameters {
                delta: DVec3::splat(factor),
                ..Default::default()
            }))
            .unwrap();
        let mesh = &editor.scene.active().unwrap().mesh;
        let src = &original.active().unwrap().mesh;
        assert_eq!(mesh.faces, src.faces);
        assert_eq!(mesh.uv_maps, src.uv_maps);
        assert_eq!(editor.scene.selection, original.selection);
        for (&a, &b) in mesh.vertices.iter().zip(&src.vertices) {
            assert_eq!(a, b * factor);
        }
    }
    let before = editor.clone();
    assert!(
        editor
            .dispatch(Command::Preview(Parameters {
                delta: DVec3::ZERO,
                ..Default::default()
            }))
            .is_err()
    );
    assert_eq!(editor.scene, before.scene);
    editor.dispatch(Command::Cancel).unwrap();
    assert_eq!(editor.scene, original);
    assert!(editor.undo.is_empty());
    editor.dispatch(Command::Activate(Tool::Move)).unwrap();
    editor
        .dispatch(Command::Preview(Parameters {
            delta: DVec3::X * 0.125,
            ..Default::default()
        }))
        .unwrap();
    editor.dispatch(Command::Drop).unwrap();
    assert_eq!(editor.undo.len(), 1);
    let committed = editor.scene.clone();
    editor.dispatch(Command::Undo).unwrap();
    assert_eq!(editor.scene, original);
    editor.dispatch(Command::Redo).unwrap();
    assert_eq!(editor.scene, committed);
    assert_eq!(
        persistence::decode(&persistence::encode(&committed).unwrap()).unwrap(),
        committed
    );
    // Validating must still reject damaged topology before selecting components.
    editor.scene.active_mut().unwrap().mesh.faces[0].vertices[0] = u32::MAX;
    assert!(editor.scene.validate().is_err());
}
