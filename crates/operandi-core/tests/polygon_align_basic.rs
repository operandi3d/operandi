//! MP-C08 Align basic acceptance (ALN-B01..B03 core obligations).
//! ALN-F01 is read from the admitted packet; every other case is independently
//! authored here with literal expected loops and corner rows.
use glam::{DVec2, DVec3, DVec4};
use operandi_core::*;

fn face(vertices: Vec<u32>, material: &str) -> Face {
    Face {
        vertices,
        uv: None,
        material: material.into(),
        smooth: false,
        part: None,
        material_label: Some(material.into()),
    }
}
fn editor(vertices: Vec<DVec3>, faces: Vec<Face>, selection: Vec<u32>) -> Editor {
    let mut e = Editor::default();
    e.dispatch(Command::NewMesh).unwrap();
    let mut mesh = Mesh {
        vertices,
        faces: faces.into(),
        ..Default::default()
    };
    mesh.attributes.extrude_edge_uses = exceptions(&mesh);
    e.scene.active_mut().unwrap().mesh = mesh.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = selection;
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    e
}
/// Independent statement of the exact same-direction/nonmanifold ledger.
fn exceptions(mesh: &Mesh) -> std::collections::BTreeMap<String, Vec<[u32; 3]>> {
    let mut uses = std::collections::BTreeMap::<(u32, u32), Vec<[u32; 3]>>::new();
    for (fi, f) in mesh.faces.iter().enumerate() {
        for i in 0..f.vertices.len() {
            let (a, b) = (f.vertices[i], f.vertices[(i + 1) % f.vertices.len()]);
            uses.entry((a.min(b), a.max(b)))
                .or_default()
                .push([fi as u32, a, b]);
        }
    }
    uses.into_iter()
        .filter(|(_, u)| u.len() > 2 || (u.len() == 2 && u[0][1..] == u[1][1..]))
        .map(|((a, b), u)| (format!("{a}:{b}"), u))
        .collect()
}
fn loops(e: &Editor) -> Vec<Vec<u32>> {
    let m = &e.scene.active().unwrap().mesh;
    m.faces.iter().map(|f| f.vertices.clone()).collect()
}

/// Run Align and check count, history, Undo/Redo and identity restoration.
fn align_and_check_history(e: &mut Editor, expected_count: usize) {
    let before = e.scene.clone();
    let before_ids = e.identities.current.clone();
    let undo_len = e.history.undo.len();
    assert_eq!(e.align_flip_count().unwrap(), expected_count);
    e.dispatch(Command::AlignPolygons).unwrap();
    assert_eq!(e.history.undo.len(), undo_len + 1);
    assert!(e.history.redo.is_empty());
    assert_eq!(e.scene.selection, before.selection);
    let after = e.scene.clone();
    let after_ids = e.identities.current.clone();
    e.dispatch(Command::Undo).unwrap();
    assert_eq!(e.scene, before);
    assert_eq!(e.identities.current, before_ids);
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(e.scene, after);
    assert_eq!(e.identities.current, after_ids);
    assert_eq!(
        persistence::decode(&persistence::encode(&after).unwrap()).unwrap(),
        after
    );
}

/// Independently authored pair with x mirrored, an arbitrary rigid placement
/// and translation; expected loops are literal anchored reversals.
#[test]
fn translated_and_mirrored_pairs_keep_lowest_slot() {
    let base = [
        DVec3::new(0., 0., 0.),
        DVec3::new(1.5, 0., 0.),
        DVec3::new(3., 0.2, 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1.5, 1.1, 0.),
        DVec3::new(3., 0.9, 0.),
    ];
    let rotation = glam::DQuat::from_euler(glam::EulerRot::XYZ, 0.4, -1.1, 2.3);
    let transforms: [&dyn Fn(DVec3) -> DVec3; 3] = [
        &|p| p + DVec3::new(-7.25, 3.5, 12.),
        &|p| DVec3::new(-p.x, p.y, p.z),
        &|p| rotation * DVec3::new(-p.x, p.y, p.z) + DVec3::new(100., -40., 0.5),
    ];
    for t in transforms {
        let vertices: Vec<DVec3> = base.iter().map(|&p| t(p)).collect();
        for (before, expected) in [
            (
                [vec![3, 0, 1, 4], vec![1, 4, 5, 2]],
                [vec![3, 0, 1, 4], vec![1, 2, 5, 4]],
            ),
            (
                [vec![2, 5, 4, 1], vec![4, 1, 0, 3]],
                [vec![2, 5, 4, 1], vec![4, 3, 0, 1]],
            ),
        ] {
            // Face 0 selected first is the reference.
            {
                let order = vec![0, 1];
                let faces = before.iter().map(|l| face(l.clone(), "M")).collect();
                let mut e = editor(vertices.clone(), faces, order.clone());
                let reference = e.scene.active().unwrap().mesh.normal(0);
                align_and_check_history(&mut e, 1);
                assert_eq!(loops(&e), expected.to_vec());
                assert_eq!(e.scene.selection.faces, order);
                let m = &e.scene.active().unwrap().mesh;
                assert!((m.normal(0) - reference).length() < 1e-9);
                assert!((m.normal(1) - reference).length() < 1e-9);
                assert_eq!(m.vertices, vertices);
            }
        }
    }
}

/// Swapped slots with full authored data, an unrelated item and an unrelated
/// unselected face adjacent to the reversed quad.
#[test]
fn attributed_pair_preserves_corner_data_labels_and_unrelated_scene() {
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(3., 0., 0.),
        DVec3::new(3., 1., 0.),
    ];
    // Slot 0 = B (reference), slot 1 = A (reversed), slot 2 = unselected C
    // sharing edge 2-5 with B.
    let faces = vec![
        face(vec![1, 4, 5, 2], "Blue"),
        face(vec![0, 1, 4, 3], "Red"),
        face(vec![2, 5, 7, 6], "Green"),
    ];
    let mut e = editor(vertices, faces, vec![0, 1]);
    // Unrelated item, untouched.
    let active = e.scene.selection.item;
    e.dispatch(Command::NewMesh).unwrap();
    e.scene.active_mut().unwrap().mesh = Mesh {
        vertices: vec![DVec3::ZERO, DVec3::X, DVec3::Y],
        faces: vec![face(vec![0, 1, 2], "Other")].into(),
        ..Default::default()
    }
    .into();
    let other_item = e.scene.active().unwrap().clone();
    e.dispatch(Command::SelectItem(active)).unwrap();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0, 1];
    let m = &mut e.scene.active_mut().unwrap().mesh;
    m.faces[1].part = Some("Left".into());
    m.faces[1].smooth = true;
    m.faces[1].uv = Some(vec![
        DVec2::new(0., 0.),
        DVec2::new(0.5, 0.),
        DVec2::new(0.5, 1.),
        DVec2::new(0., 1.),
    ]);
    m.uv_maps.insert(
        "Texture".into(),
        vec![
            vec![None, Some(DVec2::new(9., 9.)), None, None],
            vec![
                Some(DVec2::new(0.1, 0.2)),
                None,
                Some(DVec2::new(0.3, 0.4)),
                Some(DVec2::new(0.5, 0.6)),
            ],
            vec![None; 4],
        ],
    );
    m.attributes.uv_records.insert(
        "Texture".into(),
        vec![
            vec![None, Some(7), None, None],
            vec![Some(1), None, Some(2), Some(3)],
            vec![None; 4],
        ],
    );
    m.attributes.corner_weights.insert(
        "w".into(),
        vec![
            vec![None; 4],
            vec![Some(0.1), Some(0.2), None, Some(0.4)],
            vec![Some(1.); 4],
        ],
    );
    m.attributes.corner_colors.insert(
        "c".into(),
        vec![
            vec![None; 4],
            vec![None, Some(DVec4::X), Some(DVec4::Y), None],
            vec![None; 4],
        ],
    );
    m.attributes.typed_maps.insert(
        "rgb".into(),
        TypedMap {
            kind: "rgb3".into(),
            dimension: 3,
            zero_default: false,
            points: [(3, vec![1., 2., 3.])].into(),
            corners: vec![
                vec![None; 4],
                vec![Some(vec![0., 0., 1.]), None, None, Some(vec![3., 3., 3.])],
                vec![None; 4],
            ],
        },
    );
    m.attributes
        .point_uvs
        .insert("Texture".into(), [(6, DVec2::ONE)].into());
    m.attributes.polygon_sets.insert("set".into(), [1].into());
    m.attributes
        .vertex_sets
        .insert("verts".into(), [0, 3].into());
    m.attributes.creases.push(MeshCrease {
        a: 0,
        b: 1,
        weight: 0.25,
    });
    e.scene.validate().unwrap();
    e.synchronize_identities().unwrap();
    let before = e.scene.clone();
    let before_ids = e.identities.current.meshes[&active.unwrap()].clone();

    let mut expected = before.clone();
    let x = &mut expected
        .items
        .iter_mut()
        .find(|i| Some(i.id) == active)
        .unwrap()
        .mesh;
    x.faces[1].vertices = vec![0, 3, 4, 1];
    // The same-direction shared edge 1-4 is resolved, so its record is gone.
    assert!(x.attributes.extrude_edge_uses.remove("1:4").is_some());
    assert!(x.attributes.extrude_edge_uses.is_empty());
    x.faces[1].uv = Some(vec![
        DVec2::new(0., 0.),
        DVec2::new(0., 1.),
        DVec2::new(0.5, 1.),
        DVec2::new(0.5, 0.),
    ]);
    x.uv_maps.get_mut("Texture").unwrap()[1] = vec![
        Some(DVec2::new(0.1, 0.2)),
        Some(DVec2::new(0.5, 0.6)),
        Some(DVec2::new(0.3, 0.4)),
        None,
    ];
    x.attributes.uv_records.get_mut("Texture").unwrap()[1] = vec![Some(1), Some(3), Some(2), None];
    x.attributes.corner_weights.get_mut("w").unwrap()[1] =
        vec![Some(0.1), Some(0.4), None, Some(0.2)];
    x.attributes.corner_colors.get_mut("c").unwrap()[1] =
        vec![None, None, Some(DVec4::Y), Some(DVec4::X)];
    x.attributes.typed_maps.get_mut("rgb").unwrap().corners[1] =
        vec![Some(vec![0., 0., 1.]), Some(vec![3., 3., 3.]), None, None];

    align_and_check_history(&mut e, 1);
    assert_eq!(e.scene, expected);
    assert_eq!(e.scene.selection.faces, vec![0, 1]);
    let after = e
        .scene
        .items
        .iter()
        .find(|i| i.id == other_item.id)
        .unwrap();
    assert_eq!(*after, other_item);
    // Face/vertex/edge identities kept; reversed face corners move with points.
    let ids = &e.identities.current.meshes[&active.unwrap()];
    assert_eq!(ids.faces, before_ids.faces);
    assert_eq!(ids.vertices, before_ids.vertices);
    assert_eq!(ids.edges, before_ids.edges);
    let c = &before_ids.corners[1];
    assert_eq!(ids.corners[1], vec![c[0], c[3], c[2], c[1]]);
    assert_eq!(ids.corners[0], before_ids.corners[0]);
}

#[test]
fn reversed_face_adjacent_to_unselected_face_updates_exact_ledger() {
    // C shares edge 0-3 with the reversed slot-1 face; after Align, C and the
    // reversed face disagree there and the exact ledger records it.
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(-1., 0., 0.),
        DVec3::new(-1., 1., 0.),
    ];
    let faces = vec![
        face(vec![1, 4, 5, 2], "B"),
        face(vec![0, 1, 4, 3], "A"),
        face(vec![6, 0, 3, 7], "C"),
    ];
    let mut e = editor(vertices, faces, vec![0, 1]);
    assert!(
        e.scene
            .active()
            .unwrap()
            .mesh
            .attributes
            .extrude_edge_uses
            .contains_key("1:4")
    );
    align_and_check_history(&mut e, 1);
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.faces[1].vertices, vec![0, 3, 4, 1]);
    assert_eq!(m.attributes.extrude_edge_uses, exceptions(m));
    assert_eq!(
        m.attributes.extrude_edge_uses.keys().collect::<Vec<_>>(),
        vec!["0:3"]
    );
}

#[test]
fn consistent_pair_is_a_no_change_with_zero_count() {
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
    ];
    let faces = vec![face(vec![0, 1, 4, 3], "A"), face(vec![1, 2, 5, 4], "B")];
    let mut e = editor(vertices, faces, vec![0, 1]);
    let before = e.clone();
    assert_eq!(e.align_flip_count().unwrap(), 0);
    e.dispatch(Command::AlignPolygons).unwrap();
    assert_eq!(e.scene, before.scene);
    assert_eq!(e.history.undo.len(), before.history.undo.len());
}

fn pair() -> Editor {
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(5., 0., 0.),
        DVec3::new(6., 0., 0.),
        DVec3::new(6., 1., 0.),
        DVec3::new(5., 1., 0.),
        DVec3::new(1., 0.5, 2.),
    ];
    let faces = vec![
        face(vec![0, 1, 4, 3], "A"),
        face(vec![1, 4, 5, 2], "B"),
        face(vec![6, 7, 8, 9], "D"),
        face(vec![0, 1, 10], "T"),
    ];
    editor(vertices, faces, vec![0, 1])
}
fn refuses(e: &mut Editor) {
    let before = e.clone();
    assert!(e.align_flip_count().is_err());
    assert!(e.dispatch(Command::AlignPolygons).is_err());
    assert!(*e == before);
}

#[test]
fn unsupported_selections_refuse_atomically() {
    // Single polygons, larger and disconnected selections are supported since
    // the fix pass; empty and stale selections still refuse.
    for faces in [vec![], vec![0, 9]] {
        let mut e = pair();
        e.scene.selection.faces = faces;
        refuses(&mut e);
    }
    let mut e = pair();
    e.scene.selection.mode = Mode::Item;
    refuses(&mut e);
    // Authored normals on the face that would be reversed.
    let mut e = pair();
    e.scene
        .active_mut()
        .unwrap()
        .mesh
        .attributes
        .corner_normals
        .insert(
            "n".into(),
            vec![
                vec![None; 4],
                vec![Some(DVec3::Z); 4],
                vec![None; 4],
                vec![None; 3],
            ],
        );
    refuses(&mut e);
}

#[test]
fn redo_and_active_owner_are_not_consumed_by_refusal() {
    let mut e = pair();
    e.dispatch(Command::AlignPolygons).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.history.redo.is_empty());
    e.scene.selection.faces = vec![];
    refuses(&mut e);
    e.scene.selection.faces = vec![0, 1];
    e.dispatch(Command::Redo).unwrap();
    assert_eq!(loops(&e)[1], vec![1, 2, 5, 4]);

    // UIC-307: an armed Reduction settles, Align runs, Reduction arms again.
    let mut e = pair();
    e.dispatch(Command::ArmReduction).unwrap();
    assert!(e.session.reduction.is_some());
    e.dispatch(Command::AlignPolygons).unwrap();
    assert!(e.session.reduction.is_some());
}

/// Peer-review P2: a valid exact-incidence mesh where an edge of either
/// selected quad (anchor or the face that would be reversed) has three uses.
/// The pair's shared edge itself stays at exactly two uses.
fn fan_editor(slots: [Vec<u32>; 2], fan_on: (u32, u32), order: Vec<u32>) -> Editor {
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(-0.5, 0.5, 1.),
        DVec3::new(-0.5, 0.5, -1.),
        DVec3::new(2.5, 0.5, 1.),
        DVec3::new(2.5, 0.5, -1.),
    ];
    let (p, q) = match fan_on {
        (0, 3) => (6, 7),
        (2, 5) => (8, 9),
        _ => panic!("unsupported fan edge"),
    };
    let (a, b) = fan_on;
    let [s0, s1] = slots;
    let faces = vec![
        face(s0, "S0"),
        face(s1, "S1"),
        face(vec![a, b, p], "T"),
        face(vec![b, a, q], "T"),
    ];
    let mut e = editor(vertices, faces, order.clone());
    let m = &e.scene.active().unwrap().mesh;
    let key = format!("{a}:{b}");
    assert_eq!(m.attributes.extrude_edge_uses[&key].len(), 3);
    assert_eq!(m.attributes.extrude_edge_uses, exceptions(m));
    // Park a genuine redo entry, then restore the ordered selection.
    e.dispatch(Command::NewMesh).unwrap();
    e.dispatch(Command::Undo).unwrap();
    assert!(!e.history.redo.is_empty());
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = order;
    e
}

#[test]
fn nonmanifold_edges_are_not_crossed() {
    // A fan face on a selected quad's edge makes that edge nonmanifold; Align
    // still orients the pair through their shared manifold edge.
    let a = vec![0, 1, 4, 3];
    let b_mixed = vec![1, 4, 5, 2];
    for fan_on in [(0, 3), (2, 5)] {
        let mut e = fan_editor([a.clone(), b_mixed.clone()], fan_on, vec![0, 1]);
        e.dispatch(Command::AlignPolygons).unwrap();
        assert_eq!(loops(&e)[1], vec![1, 2, 5, 4]);
    }
}

/// Positive control: a nonmanifold edge elsewhere in the same valid mesh does
/// not make an otherwise supported pair ineligible.
#[test]
fn nonmanifold_edge_off_the_pair_keeps_supported_outcomes() {
    let vertices = vec![
        DVec3::new(0., 0., 0.),
        DVec3::new(1., 0., 0.),
        DVec3::new(2., 0., 0.),
        DVec3::new(0., 1., 0.),
        DVec3::new(1., 1., 0.),
        DVec3::new(2., 1., 0.),
        DVec3::new(5., 0., 0.),
        DVec3::new(6., 0., 0.),
        DVec3::new(5.5, 0.5, 1.),
        DVec3::new(5.5, 0.5, -1.),
        DVec3::new(5.5, 1., 0.),
    ];
    let fan = [
        face(vec![6, 7, 8], "T"),
        face(vec![7, 6, 9], "T"),
        face(vec![6, 7, 10], "T"),
    ];
    // Already consistent: success, count 0, no mutation, no history.
    let mut faces = vec![face(vec![0, 1, 4, 3], "A"), face(vec![1, 2, 5, 4], "B")];
    faces.extend(fan.clone());
    let mut e = editor(vertices.clone(), faces, vec![0, 1]);
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.attributes.extrude_edge_uses["6:7"].len(), 3);
    let before = e.clone();
    assert_eq!(e.align_flip_count().unwrap(), 0);
    e.dispatch(Command::AlignPolygons).unwrap();
    assert_eq!(e.scene, before.scene);
    assert_eq!(e.history.undo.len(), before.history.undo.len());
    // Reversal needed: anchor kept, other face anchored-reversed, unrelated
    // nonmanifold record unchanged.
    let mut faces = vec![face(vec![0, 1, 4, 3], "A"), face(vec![1, 4, 5, 2], "B")];
    faces.extend(fan);
    let mut e = editor(vertices, faces, vec![0, 1]);
    let m = &e.scene.active().unwrap().mesh;
    let fan_record = m.attributes.extrude_edge_uses["6:7"].clone();
    align_and_check_history(&mut e, 1);
    let m = &e.scene.active().unwrap().mesh;
    assert_eq!(m.faces[0].vertices, vec![0, 1, 4, 3]);
    assert_eq!(m.faces[1].vertices, vec![1, 2, 5, 4]);
    assert_eq!(m.attributes.extrude_edge_uses, exceptions(m));
    assert_eq!(
        m.attributes.extrude_edge_uses.keys().collect::<Vec<_>>(),
        vec!["6:7"]
    );
    assert_eq!(m.attributes.extrude_edge_uses["6:7"], fan_record);
}

#[test]
fn one_selected_polygon_aligns_its_whole_piece() {
    // Row of three quads; the middle and right ones are reversed.
    let vertices: Vec<DVec3> = (0..2)
        .flat_map(|y| (0..4).map(move |x| DVec3::new(x as f64, y as f64, 0.)))
        .collect();
    let face = |vertices: Vec<u32>| Face {
        vertices,
        uv: None,
        material: String::new(),
        material_label: None,
        smooth: false,
        part: None,
    };
    let mut m = Mesh {
        vertices,
        faces: vec![
            face(vec![0, 1, 5, 4]),
            face(vec![1, 2, 6, 5]),
            face(vec![2, 3, 7, 6]),
        ]
        .into(),
        ..Default::default()
    };
    m.faces[1].vertices = vec![1, 5, 6, 2];
    m.faces[2].vertices = vec![2, 6, 7, 3];
    m.attributes.extrude_edge_uses = exceptions(&m);
    m.validate().unwrap();
    let mut e = Editor::default();
    e.scene.new_mesh().unwrap();
    e.scene.active_mut().unwrap().mesh = m.into();
    e.scene.selection.mode = Mode::Polygon;
    e.scene.selection.faces = vec![0];
    e.scene.validate().unwrap();
    assert_eq!(e.align_flip_count().unwrap(), 2);
    e.dispatch(Command::AlignPolygons).unwrap();
    let out = &e.scene.active().unwrap().mesh;
    out.validate().unwrap();
    assert!(
        out.attributes.extrude_edge_uses.is_empty(),
        "now consistent"
    );
    assert_eq!(e.history.undo.len(), 1);
}
