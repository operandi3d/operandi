//! Set Part upgrade, spec set-part-2026-11-04 (`cases.json` SP-C01 to SP-C08
//! and SP-05/SP-08). Meshes are original: two quads side by side.
use glam::DVec3;
use operandi_core::*;
use std::collections::BTreeSet;

fn two_quads(parts: [Option<&str>; 2]) -> Mesh {
    let v = |x: f64, z: f64| DVec3::new(x, 0., z);
    Mesh {
        vertices: vec![
            v(0., 0.),
            v(1., 0.),
            v(1., 1.),
            v(0., 1.),
            v(2., 0.),
            v(2., 1.),
        ],
        faces: [[0u32, 3, 2, 1], [1, 2, 5, 4]]
            .iter()
            .zip(parts)
            .map(|(q, part)| Face {
                vertices: q.to_vec(),
                uv: None,
                material: "Red".into(),
                smooth: false,
                part: part.map(Into::into),
                material_label: Some("Red".into()),
            })
            .collect(),
        ..Default::default()
    }
}

fn editor(mesh: Mesh, selected: &[u32]) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selected.to_vec();
    e.scene.validate().unwrap();
    e.undo.clear();
    e
}

fn assign(name: &str) -> Command {
    Command::SetPart(PartAssignment {
        name: name.into(),
        expected: None,
    })
}

fn parts(e: &Editor) -> Vec<Option<String>> {
    e.scene
        .active()
        .unwrap()
        .mesh
        .faces
        .iter()
        .map(|f| f.part.clone())
        .collect()
}

#[test]
fn sp_c01_selected_polygons_get_the_part_others_keep_theirs() {
    let mut e = editor(two_quads([Some("Lid"), Some("Body")]), &[0]);
    e.dispatch(assign("Spout")).unwrap();
    assert_eq!(parts(&e), [Some("Spout".into()), Some("Body".into())]);
    let faces = &e.scene.active().unwrap().mesh.faces;
    assert!(faces.iter().all(|f| f.material == "Red"), "SP-02");
    assert_eq!(e.undo.len(), 1);
    assert_eq!(e.undo[0].label, "Set Part");
}

#[test]
fn sp_c02_nothing_selected_tags_every_polygon_in_one_step() {
    let mut e = editor(two_quads([None, None]), &[]);
    let before = e.scene.clone();
    e.dispatch(assign("Body")).unwrap();
    assert_eq!(parts(&e), [Some("Body".into()), Some("Body".into())]);
    assert_eq!(e.undo.len(), 1);
    assert!(
        e.scene.selection.faces.is_empty(),
        "the selection stays empty"
    );
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
}

#[test]
fn sp_c02_nothing_selected_skips_line_polygons() {
    let mut mesh = two_quads([None, None]);
    mesh.attributes.representations = vec![
        PolygonRepresentation::Face,
        PolygonRepresentation::Polyline {
            closed: true,
            curve: Default::default(),
        },
    ];
    let mut e = editor(two_quads([None, None]), &[]);
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene
        .validate()
        .expect("a line polygon in the mesh is valid");
    e.dispatch(assign("Body")).unwrap();
    assert_eq!(parts(&e), [Some("Body".into()), None]);
}

#[test]
fn sp_c05_reapplying_the_same_name_to_everything_is_a_no_op() {
    let mut e = editor(two_quads([Some("Lid"), Some("Lid")]), &[]);
    let before = e.scene.clone();
    e.dispatch(assign("Lid")).unwrap();
    assert!(
        e.scene == before && e.undo.is_empty(),
        "no change and no undo step"
    );
}

#[test]
fn sp_c06_part_names_come_from_every_mesh_once_each() {
    let mut e = editor(two_quads([Some("Lid"), Some("Body")]), &[]);
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = two_quads([Some("Lid"), None]).into();
    e.scene.active_mut().unwrap().visible = false;
    assert_eq!(e.part_names(), ["Body", "Lid"]);
}

#[test]
fn sp_c08_set_part_leaves_selection_sets_and_smoothing_groups_alone() {
    let mut mesh = two_quads([Some("Lid"), None]);
    mesh.attributes
        .polygon_sets
        .insert("Top".into(), BTreeSet::from([0]));
    mesh.attributes
        .polygon_sets
        .insert("Front".into(), BTreeSet::from([0]));
    mesh.attributes.smoothing_labels = vec![Some("A".into()), None];
    let mut e = editor(mesh, &[0]);
    let mut expected = e.scene.clone();
    expected.active_mut().unwrap().mesh.faces[0].part = Some("Cap".into());
    e.dispatch(assign("Cap")).unwrap();
    assert_eq!(e.scene, expected);
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(
        polygon_tags(m, 0, PolygonTagKind::SelectionSet),
        ["Front", "Top"]
    );
    assert_eq!(polygon_tags(m, 0, PolygonTagKind::SmoothingGroup), ["A"]);
}

#[test]
fn other_modes_follow_the_observed_spec() {
    // set-part-observed-2026-11-05: Item mode, and Vertex/Edge mode with
    // nothing selected, name every polygon.
    for mode in [Mode::Vertex, Mode::Edge, Mode::Item] {
        let mut e = editor(two_quads([None, None]), &[]);
        e.scene.selection.mode = mode;
        e.dispatch(assign("A")).unwrap();
        let m = &e.scene.active().unwrap().mesh;
        assert!(
            m.faces.iter().all(|f| f.part.as_deref() == Some("A")),
            "{mode:?}"
        );
    }
    // A Vertex selection names nothing, silently (decided Q-PT-02).
    let mut e = editor(two_quads([None, None]), &[]);
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0];
    let before = e.clone();
    e.dispatch(assign("A")).unwrap();
    assert!(e.scene == before.scene);
}

#[test]
fn whole_mesh_target_goes_stale_when_the_mesh_changes() {
    let e = editor(two_quads([None, None]), &[]);
    let target = e.set_part_target().unwrap();
    assert_eq!(target.faces, [0, 1]);
    let mut changed = e.clone();
    changed.scene.active_mut().unwrap().mesh.faces[1].part = Some("X".into());
    let err = changed
        .dispatch(Command::SetPart(PartAssignment {
            name: "A".into(),
            expected: Some(target),
        }))
        .unwrap_err();
    assert!(err.contains("changed"), "{err}");
}
