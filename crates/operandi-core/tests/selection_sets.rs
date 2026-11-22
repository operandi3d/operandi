//! Selection Sets (SEL-081/082): Assign, Use, Delete and Create Group on the
//! active mesh, their undo steps, saving/loading, and edge sets following
//! topology edits. Provisional choices are SET-Q01..Q08.
use glam::DVec3;
use operandi_core::*;

fn cube(mode: Mode) -> Editor {
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap().into();
    e.scene.selection.mode = mode;
    e.synchronize_identities().unwrap();
    e
}

fn attrs(e: &Editor) -> &MeshAttributes {
    &e.scene.active().unwrap().mesh.attributes
}

fn assign(e: &mut Editor, name: &str) -> Result<()> {
    e.dispatch(Command::AssignSelectionSet(name.into()))
}

fn use_set(e: &mut Editor, name: &str, mode: UseSetMode) -> Result<()> {
    e.dispatch(Command::UseSelectionSet {
        name: name.into(),
        mode,
    })
}

/// One edge of the cube's first polygon, lower point first.
fn first_edge(e: &Editor) -> (u32, u32) {
    let f = &e.scene.active().unwrap().mesh.faces[0].vertices;
    (f[0].min(f[1]), f[0].max(f[1]))
}

#[test]
fn assign_saves_the_polygon_selection_as_one_undo_step() {
    let mut e = cube(Mode::Polygon);
    e.scene.selection.faces = vec![2, 0];
    let geometry = e.scene.active().unwrap().mesh.faces.clone();
    assign(&mut e, "  Top  ").unwrap();
    assert_eq!(
        attrs(&e).polygon_sets["Top"]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 2],
        "name trimmed, members from the selection"
    );
    assert_eq!(e.scene.active().unwrap().mesh.faces, geometry);
    assert_eq!(e.scene.selection.faces, vec![2, 0], "selection untouched");
    e.dispatch(Command::Undo).unwrap();
    assert!(attrs(&e).polygon_sets.is_empty());
    e.dispatch(Command::Redo).unwrap();
    assert!(attrs(&e).polygon_sets.contains_key("Top"));
}

#[test]
fn assign_to_an_existing_name_adds_members() {
    let mut e = cube(Mode::Vertex);
    e.scene.selection.vertices = vec![0, 1];
    assign(&mut e, "Ring").unwrap();
    e.scene.selection.vertices = vec![1, 5];
    assign(&mut e, "Ring").unwrap();
    assert_eq!(
        attrs(&e).vertex_sets["Ring"]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![0, 1, 5]
    );
    // Assigning members it already has records no step.
    let steps = e.undo.len();
    assign(&mut e, "Ring").unwrap();
    assert_eq!(e.undo.len(), steps);
}

#[test]
fn assign_refuses_item_mode_empty_selection_and_blank_names() {
    let mut e = cube(Mode::Item);
    assert!(
        assign(&mut e, "A")
            .unwrap_err()
            .contains("Vertex, Edge or Polygon")
    );
    e.scene.selection.mode = Mode::Edge;
    assert!(assign(&mut e, "A").unwrap_err().contains("Select edges"));
    e.scene.selection.edges = vec![first_edge(&e)];
    assert!(assign(&mut e, "   ").unwrap_err().contains("name"));
    assert!(attrs(&e).edge_sets.is_empty());
}

#[test]
fn sets_are_per_mode_and_use_replaces_adds_and_removes() {
    let mut e = cube(Mode::Polygon);
    e.scene.selection.faces = vec![0, 1];
    assign(&mut e, "Same").unwrap();
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![3];
    assign(&mut e, "Same").unwrap();
    assert_eq!(e.selection_set_names(), vec!["Same".to_string()]);

    // Polygon mode recalls the polygon set, not the vertex one.
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![4];
    use_set(&mut e, "Same", UseSetMode::Add).unwrap();
    assert_eq!(e.scene.selection.faces, vec![4, 0, 1]);
    use_set(&mut e, "Same", UseSetMode::Remove).unwrap();
    assert_eq!(e.scene.selection.faces, vec![4]);
    use_set(&mut e, "Same", UseSetMode::Replace).unwrap();
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    assert_eq!(e.scene.selection.vertices, vec![3], "other modes untouched");

    // Use is a selection step: Undo puts the selection back.
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene.selection.faces, vec![4]);

    assert!(
        use_set(&mut e, "Nope", UseSetMode::Replace)
            .unwrap_err()
            .contains("No polygon")
    );
}

#[test]
fn edge_sets_recall_edges() {
    let mut e = cube(Mode::Edge);
    let edge = first_edge(&e);
    e.scene.selection.edges = vec![edge];
    assign(&mut e, "Seam").unwrap();
    assert!(attrs(&e).edge_sets["Seam"].contains(&edge));
    e.scene.selection.edges.clear();
    use_set(&mut e, "Seam", UseSetMode::Replace).unwrap();
    assert_eq!(e.scene.selection.edges, vec![edge]);
}

#[test]
fn delete_removes_sets_but_never_geometry() {
    let mut e = cube(Mode::Polygon);
    for (name, f) in [("A", 0), ("B", 1), ("C", 2)] {
        e.scene.selection.faces = vec![f];
        assign(&mut e, name).unwrap();
    }
    e.scene.selection.mode = Mode::Vertex;
    e.scene.selection.vertices = vec![0];
    assign(&mut e, "V").unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.dispatch(Command::CreateSelectionSetGroup {
        name: "G".into(),
        sets: vec!["A".into(), "B".into()],
    })
    .unwrap();
    let faces = e.scene.active().unwrap().mesh.faces.len();

    e.dispatch(Command::DeleteSelectionSet(Some("A".into())))
        .unwrap();
    assert_eq!(e.selection_set_names(), vec!["B".to_string(), "C".into()]);
    let groups = e.selection_set_groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0]
            .1
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        vec!["B"]
    );

    e.dispatch(Command::DeleteSelectionSet(None)).unwrap();
    assert!(attrs(&e).polygon_sets.is_empty());
    assert!(
        attrs(&e).vertex_sets.contains_key("V"),
        "only the current mode"
    );
    assert_eq!(e.scene.active().unwrap().mesh.faces.len(), faces);
    assert!(
        e.dispatch(Command::DeleteSelectionSet(None))
            .unwrap_err()
            .contains("no polygon")
    );

    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.selection_set_names(), vec!["B".to_string(), "C".into()]);
}

#[test]
fn groups_hold_named_sets_or_all_of_them() {
    let mut e = cube(Mode::Polygon);
    assert!(
        e.dispatch(Command::CreateSelectionSetGroup {
            name: "Empty".into(),
            sets: vec![]
        })
        .unwrap_err()
        .contains("Assign")
    );
    for (name, f) in [("A", 0), ("B", 1)] {
        e.scene.selection.faces = vec![f];
        assign(&mut e, name).unwrap();
    }
    e.dispatch(Command::CreateSelectionSetGroup {
        name: "All".into(),
        sets: vec![],
    })
    .unwrap();
    assert!(
        e.dispatch(Command::CreateSelectionSetGroup {
            name: "Bad".into(),
            sets: vec!["Missing".into()],
        })
        .unwrap_err()
        .contains("Missing")
    );
    let groups = e.selection_set_groups();
    assert_eq!(groups[0].0, "All");
    assert_eq!(
        groups[0].1,
        vec![
            SetRef {
                kind: SetKind::Polygon,
                name: "A".into()
            },
            SetRef {
                kind: SetKind::Polygon,
                name: "B".into()
            },
        ]
    );
    e.dispatch(Command::DeleteSelectionSetGroup("All".into()))
        .unwrap();
    assert!(e.selection_set_groups().is_empty());
    assert_eq!(e.selection_set_names().len(), 2, "sets stay");
}

#[test]
fn sets_and_groups_save_and_load() {
    let mut e = cube(Mode::Edge);
    e.scene.selection.edges = vec![first_edge(&e)];
    assign(&mut e, "Seam").unwrap();
    e.dispatch(Command::CreateSelectionSetGroup {
        name: "Seams".into(),
        sets: vec![],
    })
    .unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![3];
    assign(&mut e, "Cap").unwrap();

    let bytes = persistence::encode(&e.scene).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("\"version\": 13"));
    let back = persistence::decode(&bytes).unwrap();
    assert_eq!(back.items[0].mesh.attributes, *attrs(&e));

    // An older version number cannot carry edge sets or groups.
    let old = text.replace("\"version\": 13", "\"version\": 12");
    assert!(
        persistence::decode(old.as_bytes())
            .unwrap_err()
            .contains("version 13")
    );
}

#[test]
fn polygon_and_vertex_sets_alone_keep_the_older_file_version() {
    let mut e = cube(Mode::Polygon);
    e.scene.selection.faces = vec![0];
    assign(&mut e, "Cap").unwrap();
    let text = String::from_utf8(persistence::encode(&e.scene).unwrap()).unwrap();
    assert!(text.contains("\"version\": 7"));
}

#[test]
fn an_edge_removed_by_an_edit_leaves_its_sets() {
    let mut e = cube(Mode::Edge);
    let edges = e.scene.active().unwrap().mesh.edges();
    let (gone, kept) = (edges[0], edges[edges.len() - 1]);
    e.scene.selection.edges = vec![gone, kept];
    assign(&mut e, "Pair").unwrap();
    e.scene.selection.edges = vec![gone];
    e.dispatch(Command::EdgeCommand(EdgeCommand::Remove))
        .unwrap();
    let mesh = &e.scene.active().unwrap().mesh;
    let set = &mesh.attributes.edge_sets["Pair"];
    let now = mesh.edges();
    assert!(set.iter().all(|x| now.binary_search(x).is_ok()), "{set:?}");
    assert!(!set.is_empty(), "the untouched edge stays");
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(attrs(&e).edge_sets["Pair"].len(), 2);
}

#[test]
fn edge_sets_follow_renumbered_points() {
    // A floating point before the cube's points: Mesh Cleanup removes it and
    // every later point moves down one.
    let mut mesh = Mesh::box_mesh(DVec3::ZERO, DVec3::ONE, 1).unwrap();
    mesh.vertices.insert(0, DVec3::splat(9.));
    for f in &mut mesh.faces {
        for v in &mut f.vertices {
            *v += 1;
        }
    }
    let (a, b) = (mesh.faces[0].vertices[0], mesh.faces[0].vertices[1]);
    mesh.attributes
        .edge_sets
        .insert("E".into(), [(a.min(b), a.max(b))].into());
    let out = remove_floating_vertices(&mesh).unwrap().mesh;
    assert_eq!(
        out.attributes.edge_sets["E"]
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![(a.min(b) - 1, a.max(b) - 1)]
    );
}
